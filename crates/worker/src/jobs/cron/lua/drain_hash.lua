-- Atomically read and clear a hash.
-- Returns a flat array [field, value, field, value, ...] of the hash contents
-- and deletes the key, so concurrent worker instances never double-apply the
-- same buffered counts.
local data = redis.call("HGETALL", KEYS[1])
if #data > 0 then
    redis.call("DEL", KEYS[1])
end
return data
