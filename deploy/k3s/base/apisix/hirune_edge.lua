local bit = require("bit")
local core = require("apisix.core")
local ffi = require("ffi")
local sha256 = require("resty.sha256")
local uuid = require("resty.jit-uuid")

ffi.cdef [[
int inet_pton(int af, const char *src, void *dst);
const char *inet_ntop(int af, const void *src, char *dst, unsigned int size);
]]

local function digest(value)
    local hash = assert(sha256:new())
    assert(hash:update(value))
    return assert(hash:final())
end

local secret = os.getenv("INTERNAL_PROXY_SECRET")
if secret and secret ~= "" then
    assert(#secret >= 32 and secret:match("^[!-~]+$"), "invalid internal proxy secret configuration")
    secret = digest(secret)
else
    secret = nil
end

local origins = {}
for origin in (os.getenv("ALLOWED_ORIGINS") or ""):gmatch("[^,]+") do
    origins[origin:match("^%s*(.-)%s*$")] = true
end

local function trusted_proxy(provided)
    if not secret or type(provided) ~= "string" then
        return false
    end
    local provided_digest = digest(provided)
    local difference = 0
    for index = 1, 32 do
        difference = bit.bor(difference, bit.bxor(secret:byte(index), provided_digest:byte(index)))
    end
    return difference == 0
end

local function canonical_ip(value)
    if type(value) ~= "string" or value:find("\0", 1, true) then
        return nil
    end
    value = value:match("^%s*(.-)%s*$")
    local address = ffi.new("unsigned char[16]")
    local output = ffi.new("char[46]")
    if ffi.C.inet_pton(2, value, address) == 1 then
        return ffi.string(assert(ffi.C.inet_ntop(2, address, output, 46)))
    end
    if ffi.C.inet_pton(10, value, address) ~= 1 then
        return nil
    end
    local mapped = address[10] == 255 and address[11] == 255
    for index = 0, 9 do
        mapped = mapped and address[index] == 0
    end
    if mapped then
        return ffi.string(assert(ffi.C.inet_ntop(2, ffi.cast("unsigned char *", address) + 12, output, 46)))
    end
    return ffi.string(assert(ffi.C.inet_ntop(10, address, output, 46)))
end

local function error_response(status, code, message, headers)
    headers = headers or {}
    local request_id = uuid()
    headers["X-Request-ID"] = request_id
    headers["Content-Type"] = "application/json"
    headers["Cache-Control"] = "no-store"
    headers["X-Content-Type-Options"] = "nosniff"
    if status == 429 then
        headers["Retry-After"] = headers["Retry-After"] or headers["retry-after"] or "60"
    end
    local origin = ngx.req.get_headers(0)["origin"]
    if type(origin) == "string" and origins[origin] then
        headers["Access-Control-Allow-Origin"] = origin
        headers["Access-Control-Allow-Credentials"] = "true"
        headers["Access-Control-Expose-Headers"] = "X-Request-ID, Retry-After"
    end
    headers["Vary"] = "Origin"
    return status, {error = {code = code, message = message}, request_id = request_id}, headers
end

-- A private context variable cannot be supplied through a request header.
core.ctx.register_var("hirune_edge_client_ip", function(ctx)
    return ctx.hirune_edge_client_ip
end)

local _M = {}

local function maintenance_active()
    local file = io.open("/run/hirune-maintenance/state", "r")
    if not file then
        return true
    end
    local state = file:read(16)
    file:close()
    -- Missing or invalid operator state must not reopen application admission.
    return state ~= "off\n"
end

function _M.gateway_health()
    core.response.set_header("Cache-Control", "no-store")
    return core.response.exit(200, {status = "ok", maintenance = maintenance_active()})
end

local function maintenance_preflight(headers)
    local origin = headers["origin"]
    local methods = {GET = true, HEAD = true, POST = true, PUT = true, DELETE = true, OPTIONS = true}
    local allowed_headers = {["content-type"] = true, authorization = true, range = true, ["if-range"] = true}
    local requested_headers = headers["access-control-request-headers"] or ""
    if ngx.req.get_method() ~= "OPTIONS" or type(origin) ~= "string" or not origins[origin]
        or type(headers["access-control-request-method"]) ~= "string"
        or not methods[headers["access-control-request-method"]]
        or type(requested_headers) ~= "string" then
        return false
    end
    for name in requested_headers:gmatch("[^,]+") do
        if not allowed_headers[name:match("^%s*(.-)%s*$"):lower()] then
            return false
        end
    end
    core.response.set_header({
        ["Access-Control-Allow-Origin"] = origin,
        ["Access-Control-Allow-Credentials"] = "true",
        ["Access-Control-Allow-Methods"] = "GET, HEAD, POST, PUT, DELETE, OPTIONS",
        ["Access-Control-Allow-Headers"] = "content-type, authorization, range, if-range",
        ["Cache-Control"] = "no-store",
        ["Vary"] = "Origin, Access-Control-Request-Method, Access-Control-Request-Headers",
    })
    return true
end

function _M.rewrite(_, ctx)
    if maintenance_active() then
        if maintenance_preflight(ngx.req.get_headers(0)) then
            return core.response.exit(204)
        end
        local status, body, headers = error_response(503, "maintenance", "The service is temporarily unavailable for maintenance.", {
            ["Retry-After"] = "30",
        })
        core.response.set_header(headers)
        return core.response.exit(status, body)
    end
    local headers = ngx.req.get_headers(0)
    local selected
    if trusted_proxy(headers["x-internal-secret"]) then
        selected = headers["x-real-client-ip"]
    end
    if selected == nil then
        selected = headers["cf-connecting-ip"]
    end
    if selected == nil then
        selected = ctx.var.remote_addr
    end
    local client = canonical_ip(selected)
    if not client then
        local status, body, response_headers = error_response(400, "invalid_input", "The request is invalid.")
        core.response.set_header(response_headers)
        return core.response.exit(status, body)
    end
    ctx.hirune_edge_client_ip = client
end

function _M.transform(status, body, headers)
    if status == 429 and not ngx.var.upstream_status then
        return error_response(429, "edge_rate_limited", "Too many requests.", headers)
    end
    return status, body, headers
end

function _M.header_filter()
    if ngx.status == 429 and ngx.header["Access-Control-Allow-Origin"] then
        local exposed = ngx.header["Access-Control-Expose-Headers"] or ""
        if type(exposed) == "table" then
            exposed = table.concat(exposed, ", ")
        end
        if not exposed:lower():find("retry-after", 1, true) then
            ngx.header["Access-Control-Expose-Headers"] = exposed == "" and "Retry-After" or exposed .. ", Retry-After"
        end
    end
end

return _M
