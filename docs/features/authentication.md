# Authentication

AxumKit uses **session-based authentication** with Redis. Sessions are stored as Redis keys with sliding TTL and absolute expiration.

## Session Lifecycle

### Create Session (Login)

When a user logs in (email/password or OAuth), a session is created:

1. Generate a random session ID
2. Store session data in Redis: `session:{session_id}`
3. Set sliding TTL (default: 168 hours) and max lifetime (default: 720 hours)
4. Set `session_id` cookie on the response

### Session Refresh (Sliding Expiration)

On each authenticated request, the session TTL is conditionally refreshed:

- If remaining TTL is below `AUTH_SESSION_REFRESH_THRESHOLD` % of the sliding TTL, extend it
- This avoids unnecessary Redis writes on every request
- Max lifetime (`max_expires_at`) is never extended — it's an absolute limit

### Session Extraction

Handlers use extractor types to access the session:

```rust
// Requires authentication — returns 401 if no valid session
pub async fn protected_handler(
    RequiredSession(session): RequiredSession,
) -> Result<Json<Response>, Errors> {
    let user_id = session.user_id;
    // ...
}

// Works with or without authentication
pub async fn public_handler(
    OptionalSession(session): OptionalSession,
) -> Result<Json<Response>, Errors> {
    if let Some(session) = session {
        // authenticated
    }
    // ...
}
```

### Logout

Logout deletes the session from Redis and clears the cookie.

## Email/Password Login

```
POST /v0/auth/login
```

Flow:
1. Validate email and password
2. Verify password hash (Argon2)
3. Check if email is verified
4. If TOTP is enabled → return `totp_required: true` with a temporary token
5. Otherwise → create session, set cookie

## User Registration

```
POST /v0/users
```

1. Validate handle, email, password
2. Hash password with Argon2
3. Create user record
4. Send verification email via worker (NATS job)

## Email Verification

```
POST /v0/auth/verify-email
```

1. User receives email with verification token
2. POST the token to verify
3. Sets `verified_at` timestamp on the user

```
POST /v0/auth/resend-verification-email
```

Resends the verification email (requires active session).

## Password Reset

```
POST /v0/auth/forgot-password
```

1. User submits their email
2. System generates a reset token, stores in Redis
3. Sends reset email via worker

```
POST /v0/auth/reset-password
```

1. User submits the token + new password
2. Validates token, hashes new password, updates user

## Password Change

```
POST /v0/auth/change-password
```

Requires active session. User provides current password and new password.

## Email Change

```
POST /v0/auth/change-email
```

1. User requests email change (requires session + current password)
2. System sends confirmation to the new email

```
POST /v0/auth/confirm-email-change
```

Confirms the email change with the token from the confirmation email.

## Session Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `AUTH_SESSION_MAX_LIFETIME_HOURS` | — | Absolute session expiration |
| `AUTH_SESSION_SLIDING_TTL_HOURS` | — | TTL extended on activity |
| `AUTH_SESSION_REFRESH_THRESHOLD` | — | % of TTL remaining to trigger refresh |
| `AUTH_EMAIL_VERIFICATION_TOKEN_EXPIRE_TIME` | `1` min | Verification token TTL |
| `AUTH_PASSWORD_RESET_TOKEN_EXPIRE_TIME` | `15` min | Reset token TTL |
| `AUTH_EMAIL_CHANGE_TOKEN_EXPIRE_TIME` | `15` min | Email change token TTL |
