# Rate Limiting

AxumKit implements a **sliding window rate limiter** using Redis Sorted Sets and a Lua script for atomic execution.

## Algorithm

The sliding window log algorithm:

1. **Remove expired entries:** Delete all entries outside the current time window
2. **Count requests:** Count remaining entries in the sorted set
3. **Check limit:** If count >= max, reject with `429 Too Many Requests`
4. **Add entry:** Add the current request with timestamp as score and UUID v7 as member
5. **Set TTL:** Expire the key slightly after the window duration

All 5 steps run atomically in a single Lua script — no race conditions.

## Lua Script

Located at `crates/axumkit-server/src/middleware/lua/sliding_window.lua`:

```lua
local key = KEYS[1]
local now = tonumber(ARGV[1])       -- Current time (ms)
local window = tonumber(ARGV[2])    -- Window size (ms)
local max_requests = tonumber(ARGV[3])
local request_id = ARGV[4]          -- UUID v7

-- Remove entries outside window
redis.call('ZREMRANGEBYSCORE', key, 0, now - window)

-- Count current requests
local count = redis.call('ZCARD', key)

-- Check limit
if count >= max_requests then
    -- Calculate retry_after from oldest entry
    local oldest = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
    local retry_after = math.ceil((oldest[2] + window - now) / 1000)
    return {0, count, retry_after}
end

-- Add request and set TTL
redis.call('ZADD', key, now, request_id)
redis.call('EXPIRE', key, math.ceil(window / 1000) + 1)
return {1, count + 1, 0}
```

The script is loaded once via `LazyLock` and reused for all rate limit checks.

## Per-Route Configuration

Rate limits are configured per route using `RateLimitConfig`:

```rust
pub struct RateLimitConfig {
    pub route_name: &'static str,  // Redis key suffix
    pub max_requests: u32,         // Max requests in window
    pub window_secs: u64,          // Window duration
}
```

## Identification

Rate limiting uses the **anonymous user ID** (set by the `anonymous_user_middleware` as a cookie) rather than IP address. This provides more accurate per-client limiting, especially behind proxies.

Redis key format: `rate_limit:{route_name}:{anonymous_user_id}`

An IP-based variant (`rate_limit_by_ip`) is also available but not active by default.

## Response Headers

All responses include rate limit headers:

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Maximum requests allowed |
| `X-RateLimit-Remaining` | Requests remaining in current window |
| `X-RateLimit-Reset` | Seconds until the window resets |
| `Retry-After` | Seconds to wait before retrying (only on 429) |

## Error Response

When rate limit is exceeded:

```json
{
  "status": 429,
  "code": "rate_limit:exceeded"
}
```

HTTP Status: `429 Too Many Requests`
