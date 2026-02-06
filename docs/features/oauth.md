# OAuth2

AxumKit supports OAuth2 sign-in and account linking with **Google** and **GitHub**.

## OAuth Flow

### 1. Get Authorization URL

```
GET /v0/auth/oauth/google/authorize
GET /v0/auth/oauth/github/authorize
```

Returns a URL to redirect the user to the provider's consent screen. A random `state` parameter is stored in Redis for CSRF protection.

### 2. Exchange Code for Token

```
POST /v0/auth/oauth/google/login
POST /v0/auth/oauth/github/login
```

After the user authorizes, the frontend sends the `code` and `state` back:

1. Verify `state` matches the one stored in Redis
2. Exchange `code` for an access token with the provider
3. Fetch user info (email, name, avatar) from the provider's API
4. Find or create the user account

### 3. User Resolution

The system checks if a user with the provider's email already exists:

- **Existing user with this OAuth connection:** Sign in, create session
- **Existing user without this OAuth connection:** Automatically link the account
- **New user:** Return a pending signup token — the frontend must call `/auth/complete-signup` with a chosen `handle`

### 4. Complete Signup (New OAuth Users)

```
POST /v0/auth/complete-signup
```

New OAuth users must choose a handle before their account is created. The pending signup data is stored in Redis with a configurable TTL (`OAUTH_PENDING_SIGNUP_TTL_MINUTES`, default: 10 minutes).

## Account Linking

Authenticated users can link additional OAuth providers to their account:

```
POST /v0/auth/oauth/google/link
POST /v0/auth/oauth/github/link
```

Requires an active session. Exchanges the OAuth code and adds a `user_oauth_connections` record.

## List Connections

```
GET /v0/auth/oauth/connections
```

Returns all OAuth providers linked to the current user's account.

## Unlink Connection

```
POST /v0/auth/oauth/connections/unlink
```

Removes an OAuth connection. **Cannot unlink the last connection** if the user has no password set (they would be locked out).

## Configuration

| Variable | Required | Description |
|----------|----------|-------------|
| `GOOGLE_CLIENT_ID` | Yes | Google OAuth client ID |
| `GOOGLE_CLIENT_SECRET` | Yes | Google OAuth client secret |
| `GOOGLE_REDIRECT_URI` | Yes | Google callback URL |
| `GITHUB_CLIENT_ID` | Yes | GitHub OAuth client ID |
| `GITHUB_CLIENT_SECRET` | Yes | GitHub OAuth client secret |
| `GITHUB_REDIRECT_URI` | Yes | GitHub callback URL |

## Supported Providers

The `OAuthProvider` enum currently defines:

| Provider | Implemented |
|----------|-------------|
| Google | Yes |
| GitHub | Yes |
| Discord | Enum defined, not yet implemented |
| X (Twitter) | Enum defined, not yet implemented |
| Microsoft | Enum defined, not yet implemented |
