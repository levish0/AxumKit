# API Endpoints

All versioned endpoints are prefixed with `/v0`.

## Health

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health-check` | No | Health check |

## Auth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v0/auth/login` | No | Email/password login |
| POST | `/v0/auth/logout` | Yes | Destroy session |
| POST | `/v0/auth/verify-email` | No | Verify email token |
| POST | `/v0/auth/resend-verification-email` | Yes | Resend verification email |
| POST | `/v0/auth/forgot-password` | No | Request password reset |
| POST | `/v0/auth/reset-password` | No | Reset password with token |
| POST | `/v0/auth/change-password` | Yes | Change password |
| POST | `/v0/auth/change-email` | Yes | Request email change |
| POST | `/v0/auth/confirm-email-change` | No | Confirm email change token |
| POST | `/v0/auth/complete-signup` | No | Complete OAuth signup (set handle) |

## OAuth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v0/auth/oauth/google/authorize` | No | Get Google OAuth URL |
| POST | `/v0/auth/oauth/google/login` | No | Exchange Google code for session |
| POST | `/v0/auth/oauth/google/link` | Yes | Link Google account |
| GET | `/v0/auth/oauth/github/authorize` | No | Get GitHub OAuth URL |
| POST | `/v0/auth/oauth/github/login` | No | Exchange GitHub code for session |
| POST | `/v0/auth/oauth/github/link` | Yes | Link GitHub account |
| GET | `/v0/auth/oauth/connections` | Yes | List linked OAuth connections |
| POST | `/v0/auth/oauth/connections/unlink` | Yes | Unlink OAuth connection |

## TOTP 2FA

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v0/auth/totp/setup` | Yes | Generate TOTP secret + QR code |
| POST | `/v0/auth/totp/enable` | Yes | Enable TOTP |
| POST | `/v0/auth/totp/verify` | No | Verify TOTP during login |
| GET | `/v0/auth/totp/status` | Yes | Check TOTP status |
| POST | `/v0/auth/totp/disable` | Yes | Disable TOTP |
| POST | `/v0/auth/totp/backup-codes/regenerate` | Yes | Regenerate backup codes |

## Users

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v0/users` | No | Register user |
| GET | `/v0/users/profile` | No | Get user profile by handle |
| GET | `/v0/users/profile/id` | No | Get user profile by UUID |
| GET | `/v0/users/handle/{handle}/available` | No | Check handle availability |
| GET | `/v0/user/me` | Yes | Get my profile |
| PATCH | `/v0/user/me` | Yes | Update my profile |
| POST | `/v0/user/me/profile-image` | Yes | Upload profile image |
| DELETE | `/v0/user/me/profile-image` | Yes | Delete profile image |
| POST | `/v0/user/me/banner-image` | Yes | Upload banner image |
| DELETE | `/v0/user/me/banner-image` | Yes | Delete banner image |

## Search

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v0/search/users` | No | Search users |

## Action Logs

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v0/action-logs` | No | Get recent action logs |

## EventStream

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v0/eventstream/actions` | No | SSE stream of action log events |

## OpenAPI

| Path | Description |
|------|-------------|
| `/docs` | Swagger UI (debug builds only) |
| `/swagger.json` | OpenAPI JSON (debug builds only) |

## Error Response Format

```json
{
  "status": 400,
  "code": "user:not_found",
  "details": "User with ID ... not found"
}
```

- `status`: HTTP status code
- `code`: machine-readable code
- `details`: dev-only details
