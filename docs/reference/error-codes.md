# Error Codes

Errors use `domain:reason` format. `details` is only included in development mode.

## Auth

| Code | HTTP | Description |
|------|------|-------------|
| `auth:invalid_credentials` | 401 | Invalid email/password combination |

## User

| Code | HTTP | Description |
|------|------|-------------|
| `user:invalid_password` | 401 | Wrong password |
| `user:password_not_set` | 401 | User has no password |
| `user:invalid_session` | 401 | Session is invalid |
| `user:not_verified` | 401 | Email not verified |
| `user:not_found` | 404 | User not found |
| `user:unauthorized` | 401 | Not authenticated |
| `user:banned` | 403 | User is banned |
| `user:permission_insufficient` | 403 | Insufficient permissions |
| `user:handle_already_exists` | 409 | Handle already taken |
| `user:email_already_exists` | 409 | Email already registered |
| `user:token_expired` | 401 | Token expired |
| `user:no_refresh_token` | 401 | No refresh token available |
| `user:invalid_token` | 401 | Invalid token |

## Session

| Code | HTTP | Description |
|------|------|-------------|
| `session:invalid_user_id` | 401 | Invalid user ID in session |
| `session:expired` | 401 | Session expired |
| `session:not_found` | 401 | Session not found |

## OAuth

| Code | HTTP | Description |
|------|------|-------------|
| `oauth:invalid_auth_url` | 400 | Invalid auth URL |
| `oauth:invalid_token_url` | 400 | Invalid token URL |
| `oauth:invalid_redirect_url` | 400 | Invalid redirect URL |
| `oauth:token_exchange_failed` | 400 | OAuth token exchange failed |
| `oauth:user_info_fetch_failed` | 400 | Failed to fetch provider user info |
| `oauth:user_info_parse_failed` | 500 | Failed to parse provider user info |
| `oauth:account_already_linked` | 409 | OAuth account already linked |
| `oauth:connection_not_found` | 404 | OAuth connection not found |
| `oauth:cannot_unlink_last_connection` | 400 | Cannot unlink last login method |
| `oauth:invalid_image_url` | 400 | Invalid provider image URL |
| `oauth:invalid_state` | 400 | Invalid OAuth state |
| `oauth:state_expired` | 400 | Expired OAuth state |
| `oauth:handle_required` | 400 | Handle required |
| `oauth:email_already_exists` | 409 | Email already exists |
| `oauth:email_not_verified` | 400 | Provider email not verified |

## Password

| Code | HTTP | Description |
|------|------|-------------|
| `password:required_for_update` | 400 | Current password required |
| `password:incorrect` | 400 | Current password incorrect |
| `password:cannot_update_oauth_only` | 400 | OAuth-only account cannot update password |
| `password:new_password_missing` | 400 | New password missing |
| `password:already_set` | 400 | Password already set |

## Token

| Code | HTTP | Description |
|------|------|-------------|
| `token:invalid_verification` | 400 | Invalid verification token |
| `token:expired_verification` | 400 | Expired verification token |
| `token:email_mismatch` | 400 | Token email mismatch |
| `token:invalid_reset` | 400 | Invalid reset token |
| `token:expired_reset` | 400 | Expired reset token |
| `token:invalid_email_change` | 400 | Invalid email-change token |

## Email

| Code | HTTP | Description |
|------|------|-------------|
| `email:already_verified` | 400 | Email already verified |

## TOTP

| Code | HTTP | Description |
|------|------|-------------|
| `totp:already_enabled` | 409 | TOTP already enabled |
| `totp:not_enabled` | 400 | TOTP not enabled |
| `totp:invalid_code` | 400 | Invalid TOTP/backup code |
| `totp:temp_token_invalid` | 400 | Invalid temporary login token |
| `totp:temp_token_expired` | 400 | Expired temporary login token |
| `totp:backup_code_exhausted` | 401 | Backup codes exhausted |
| `totp:secret_generation_failed` | 500 | TOTP secret generation failed |
| `totp:qr_generation_failed` | 500 | QR generation failed |

## File

| Code | HTTP | Description |
|------|------|-------------|
| `file:upload_error` | 400 | File upload failed |
| `file:not_found` | 400 | File not found |
| `file:read_error` | 400 | File read failed |

## General

| Code | HTTP | Description |
|------|------|-------------|
| `general:bad_request` | 400 | Bad request |
| `general:validation_error` | 400 | Validation error |
| `general:invalid_ip_address` | 400 | Invalid IP address |
| `FORBIDDEN` | 403 | Forbidden |
| `FILE_TOO_LARGE` | 413 | Payload too large |

## Rate Limit

| Code | HTTP | Description |
|------|------|-------------|
| `rate_limit:exceeded` | 429 | Too many requests |

## Turnstile

| Code | HTTP | Description |
|------|------|-------------|
| `turnstile:token_missing` | 400 | Turnstile token missing |
| `turnstile:verification_failed` | 403 | Turnstile verification failed |
| `turnstile:service_error` | 503 | Turnstile service error |

## Search

| Code | HTTP | Description |
|------|------|-------------|
| `meilisearch:query_failed` | 500 | Search query failed |

## EventStream

| Code | HTTP | Description |
|------|------|-------------|
| `eventstream:publish_failed` | 503 | Event publish failed |

## Worker/System

| Code | HTTP | Description |
|------|------|-------------|
| `worker:connection_failed` | 503 | Worker connection failed |
| `worker:response_invalid` | 502 | Invalid worker response |
| `worker:verification_email_send_failed` | 502 | Verification email send failed |
| `worker:password_reset_email_send_failed` | 502 | Password reset email send failed |
| `system:internal_error` | 400 | Internal error |
| `system:hashing_error` | 500 | Hashing error |
| `system:not_found` | 404 | Not found |
| `system:transaction_error` | 500 | Transaction error |
| `system:database_error` | 500 | Database error |
| `system:token_creation_error` | 500 | Token creation error |
