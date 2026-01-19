-- Sliding Window Rate Limiter
-- Algorithm: Sliding Window Log using Redis Sorted Set
--
-- Arguments:
--   KEYS[1] = rate limit key (e.g., "rate_limit:auth.login:user123")
--   ARGV[1] = current timestamp in milliseconds
--   ARGV[2] = window size in milliseconds
--   ARGV[3] = max requests allowed in window
--   ARGV[4] = unique request ID (UUID v7)
--
-- Returns: {allowed (0/1), count, retry_after_seconds}

local key = KEYS[1]
local now = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local max_requests = tonumber(ARGV[3])
local request_id = ARGV[4]

-- 1. Remove entries outside the sliding window
redis.call('ZREMRANGEBYSCORE', key, 0, now - window)

-- 2. Count current requests in window
local count = redis.call('ZCARD', key)

-- 3. Check if rate limit exceeded
if count >= max_requests then
    -- Calculate retry_after from oldest entry
    local oldest = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
    local retry_after = 0
    if #oldest >= 2 then
        retry_after = math.ceil((tonumber(oldest[2]) + window - now) / 1000)
        if retry_after < 0 then
            retry_after = 0
        end
    end
    return {0, count, retry_after}
end

-- 4. Add new request (using unique request_id to avoid Set collisions)
redis.call('ZADD', key, now, request_id)

-- 5. Set TTL slightly longer than window to ensure cleanup
redis.call('EXPIRE', key, math.ceil(window / 1000) + 1)

return {1, count + 1, 0}