# OIDC test keys

These two RSA private keys are public, disposable test fixtures generated for the
local mock identity provider. They are not secrets and must never be used by a
deployed provider or application. Tests verify signature validation and key rotation
using the same OpenID Connect library as the application.
