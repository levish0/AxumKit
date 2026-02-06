# Error Codes

All errors follow the format `domain:operation`. The `details` field is only included in development mode.

## User Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `user:invalid_password` | 401 | Wrong password |
| `user:password_not_set` | 401 | User has no password (OAuth-only account) |
| `user:invalid_session` | 401 | Session is invalid |
| `user:not_verified` | 401 | Email not verified |
| `user:not_found` | 404 | User not found |
| `user:unauthorized` | 401 | Not authenticated |
| `user:banned` | 403 | User is banned |
| `user:permission_insufficient` | 403 | Insufficient permissions |
| `user:handle_already_exists` | 409 | Handle is taken |
| `user:email_already_exists` | 409 | Email is already registered |
| `user:token_expired` | 401 | Auth token has expired |
| `user:no_refresh_token` | 401 | No refresh token available |
| `user:invalid_token` | 401 | Auth token is invalid |

## Session Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `session:invalid_user_id` | 401 | Session user ID is malformed |
| `session:expired` | 401 | Session has expired (max lifetime) |
| `session:not_found` | 401 | Session not found in Redis |

## OAuth Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `oauth:invalid_auth_url` | 400 | Failed to build authorization URL |
| `oauth:invalid_token_url` | 400 | Failed to build token exchange URL |
| `oauth:invalid_redirect_url` | 400 | Invalid redirect URI |
| `oauth:token_exchange_failed` | 400 | Code-to-token exchange failed |
| `oauth:user_info_fetch_failed` | 400 | Failed to fetch user info from provider |
| `oauth:user_info_parse_failed` | 500 | Failed to parse provider user info |
| `oauth:account_already_linked` | 409 | OAuth account already linked to another user |
| `oauth:connection_not_found` | 404 | OAuth connection not found |
| `oauth:cannot_unlink_last_connection` | 400 | Cannot unlink the last OAuth connection (no password) |
| `oauth:invalid_image_url` | 400 | Invalid avatar URL from provider |
| `oauth:invalid_state` | 400 | OAuth state parameter mismatch |
| `oauth:state_expired` | 400 | OAuth state has expired |
| `oauth:handle_required` | 400 | Handle required for new OAuth signup |
| `oauth:email_already_exists` | 409 | Email already exists |
| `oauth:email_not_verified` | 400 | Provider email not verified |

## Password Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `password:required_for_update` | 400 | Current password required |
| `password:incorrect` | 400 | Current password is wrong |
| `password:cannot_update_oauth_only` | 400 | Cannot set password on OAuth-only account |
| `password:new_password_missing` | 400 | New password not provided |
| `password:already_set` | 400 | Password is already set |

## Token Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `token:invalid_verification` | 400 | Email verification token is invalid |
| `token:expired_verification` | 400 | Email verification token has expired |
| `token:email_mismatch` | 400 | Token email doesn't match |
| `token:invalid_reset` | 400 | Password reset token is invalid |
| `token:expired_reset` | 400 | Password reset token has expired |
| `token:invalid_email_change` | 400 | Email change token is invalid |

## Email Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `email:already_verified` | 400 | Email is already verified |

## TOTP Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `totp:already_enabled` | 409 | TOTP is already enabled |
| `totp:not_enabled` | 400 | TOTP is not enabled |
| `totp:invalid_code` | 400 | Invalid TOTP or backup code |
| `totp:temp_token_invalid` | 400 | Temporary login token is invalid |
| `totp:temp_token_expired` | 400 | Temporary login token has expired |
| `totp:backup_code_exhausted` | 401 | All backup codes used |
| `totp:secret_generation_failed` | 500 | Failed to generate TOTP secret |
| `totp:qr_generation_failed` | 500 | Failed to generate QR code |

## Post Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `post:not_found` | 404 | Post not found |

## File Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `file:upload_error` | 400 | File upload failed |
| `file:not_found` | 400 | File not found |
| `file:read_error` | 400 | File read failed |

## General Errors

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `general:bad_request` | 400 | Bad request |
| `general:validation_error` | 400 | Validation failed |
| `general:invalid_ip_address` | 400 | Invalid IP address |
| `FORBIDDEN` | 403 | Access forbidden |
| `FILE_TOO_LARGE` | 413 | Payload too large |

## Rate Limiting

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `rate_limit:exceeded` | 429 | Too many requests |

## Turnstile (Bot Protection)

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `turnstile:token_missing` | 400 | Turnstile token not provided |
| `turnstile:verification_failed` | 403 | Turnstile verification failed |
| `turnstile:service_error` | 503 | Turnstile service unavailable |

## Search

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `meilisearch:query_failed` | 500 | Search query failed |

## EventStream

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `eventstream:publish_failed` | 503 | Event publish failed |

## Worker / System

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `worker:connection_failed` | 503 | Worker service connection failed |
| `worker:response_invalid` | 502 | Worker returned invalid response |
| `worker:verification_email_send_failed` | 502 | Verification email failed to send |
| `worker:password_reset_email_send_failed` | 502 | Password reset email failed to send |
| `system:internal_error` | 400 | Internal server error |
| `system:hashing_error` | 500 | Password hashing failed |
| `system:not_found` | 404 | Resource not found |
| `system:transaction_error` | 500 | Database transaction failed |
| `system:database_error` | 500 | Database error |
| `system:token_creation_error` | 500 | Token creation failed |
