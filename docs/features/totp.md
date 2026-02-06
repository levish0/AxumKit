# TOTP 2FA

AxumKit supports Time-based One-Time Password (TOTP) as a second factor for authentication, with backup codes for recovery.

## Setup Flow

### 1. Generate TOTP Secret

```
POST /v0/auth/totp/setup
```

Requires an active session. Returns:
- TOTP secret (base32 encoded)
- QR code (data URI for authenticator apps)
- Temporary setup token

The user scans the QR code with their authenticator app (Google Authenticator, Authy, etc.).

### 2. Enable TOTP

```
POST /v0/auth/totp/enable
```

User submits a TOTP code from their authenticator app to verify they set it up correctly. On success:
- `totp_secret` is stored on the user
- `totp_enabled_at` is set
- 10 backup codes are generated and returned (one-time display)

### 3. Check Status

```
GET /v0/auth/totp/status
```

Returns whether TOTP is enabled for the current user.

## Login with TOTP

When a user with TOTP enabled tries to log in:

1. Password verification succeeds
2. Instead of creating a session, the server returns `totp_required: true` with a **temporary token**
3. The frontend prompts for the TOTP code
4. User submits the code:

```
POST /v0/auth/totp/verify
```

With the temporary token + TOTP code (or backup code). On success, a session is created.

## Backup Codes

When TOTP is enabled, 10 backup codes are generated. Each code can be used exactly once in place of a TOTP code during login.

### Regenerate Backup Codes

```
POST /v0/auth/totp/backup-codes/regenerate
```

Requires active session + TOTP code verification. Invalidates all existing backup codes and generates 10 new ones.

## Disable TOTP

```
POST /v0/auth/totp/disable
```

Requires active session + current TOTP code. Removes `totp_secret` and `totp_enabled_at` from the user.

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `totp:already_enabled` | 409 | TOTP is already enabled |
| `totp:not_enabled` | 400 | TOTP is not enabled |
| `totp:invalid_code` | 400 | Invalid TOTP or backup code |
| `totp:temp_token_invalid` | 400 | Temporary token is invalid |
| `totp:temp_token_expired` | 400 | Temporary token has expired |
| `totp:backup_code_exhausted` | 401 | All backup codes have been used |
| `totp:secret_generation_failed` | 500 | Failed to generate TOTP secret |
| `totp:qr_generation_failed` | 500 | Failed to generate QR code |
