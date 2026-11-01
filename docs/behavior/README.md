# Behavior contracts

These documents describe supported behavior and the invariants tests must preserve.
Update them with the implementation. Use milestone records for validation results.

- [Identity](identity.md): account, credential, session, OAuth, and MFA lifecycle.
- [Jobs and mail](jobs-and-mail.md): atomic registration, delivery, recovery, and retries.
- [Authorization](authorization.md): Cedar decisions, organization scope, and list filtering.
- [External API access](api-access.md): organization-owned keys, scopes, operator allocation, shared request accounting, and retention.
- [Files and search](files-and-search.md): private originals, upload limits, attachment access, deletion, and derived filename search.
- [Posts and comments](posts-and-comments.md): organization content, Cedar rules, versions, attachments, and cursor lists.
- [Profiles and search](profiles-and-search.md): scoped user discovery, derived projections, numbered search pages, and count semantics.
- [Notifications and push](notifications-and-push.md): durable comment inbox, preferences, devices, optional FCM, and delivery limits.
- [API](../api.md): validation, errors, cookies, and pagination.
- [Transaction boundaries](../transaction-boundaries.md): ownership and lock order.
- [Regression inventory](../regressions.md): disposition of prior security and feature tests.
