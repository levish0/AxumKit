-- Atomically reserve a pending email signup.
--
-- KEYS[1] = email index key   (email_signup:email:{email})
-- KEYS[2] = handle index key  (email_signup:handle:{handle})
-- KEYS[3] = token payload key (email_verification:{token})
--
-- ARGV[1] = token value (stored in index keys)
-- ARGV[2] = JSON payload (stored in token key)
-- ARGV[3] = TTL in seconds
--
-- Returns:
--   1  = success (all three keys set)
--  -1  = email index already exists
--  -2  = handle index already exists

if redis.call("EXISTS", KEYS[1]) == 1 then
    return -1
end

if redis.call("EXISTS", KEYS[2]) == 1 then
    return -2
end

local ttl = tonumber(ARGV[3])
redis.call("SET", KEYS[1], ARGV[1], "EX", ttl)
redis.call("SET", KEYS[2], ARGV[1], "EX", ttl)
redis.call("SET", KEYS[3], ARGV[2], "EX", ttl)

return 1
