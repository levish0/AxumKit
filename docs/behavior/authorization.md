# Authorization

Bootstrap loads and validates the Cedar schema and policies. Invalid policy
prevents startup. Evaluation diagnostics are internal failures and deny the
operation. They do not permit access through a fallback.

An authenticated actor must be active and verified. Self-account operations use
the account policy after a fresh database check. Global administrator status does
not imply membership in every organization. The organization example permits
members to read and its owner to manage. Non-member detail access returns 404.
Organization creation includes the owner membership in the same transaction.

Organization lists apply membership scope inside the query before pagination.
Do not fetch an unrestricted page and filter it afterward. Mutation services lock
the actor before the organization, recheck access, and require the expected
resource version. Stale updates return conflict rather than overwriting changes.

Posts add concrete resource attributes and actions. Members create posts and
read published posts. Authors and organization owners can read drafts and
edit/delete posts. Single-level comment deletion permits the comment author,
post author, or organization owner while requiring a visible published parent.
Global administrators gain no implicit content access. External API clients are
separate Cedar principals restricted to their owning organization's published
posts, after key/scope/quota admission. See [posts and comments](posts-and-comments.md).

Owners may add/remove an existing account through scoped services with recent
authentication. They cannot remove the owner membership. The example bounds an
organization to 1,000 members. User discovery, search hydration, and notification
visibility repeat the current organization-membership scope in PostgreSQL.

The example deliberately has no general permissions editor, role hierarchy,
membership invitation flow, or administrator HTTP API. Add product operations through
their domain service, policy, scoped queries, and tests together. Never infer
authorization from UUID possession or from a cached session principal alone.
