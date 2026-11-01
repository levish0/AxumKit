# Profiles and search

## Profiles

`PUT /api/v1/account/profile` creates a profile with `expected_version: 0`, or
updates it with the current version. A missing own profile returns 404 on GET.
Display names contain 1–80 characters. Bio contains up to 1,000. Profiles have
their own UUIDv7 primary key and a unique account UUID.

Organization user lists, detail, and search expose account UUID, display name,
bio, profile version, and creation time. Only active, verified current members
with a profile appear. Email, credentials, MFA state, and administrator flags are
absent from profile DTOs and search documents. There is no global public directory.

## Derived indexes

The shared Meilisearch transport owns bounded HTTP responses, asynchronous task
completion, and index initialization. Each domain owns its projection:

| Index | Searchable fields | Server-derived filter | Returned engine fields |
| --- | --- | --- | --- |
| Files | filename | organization_id | id |
| Posts | title, body of published posts | organization_id | id, version |
| Profiles | display_name, bio | organization_ids | id, version |

Configure distinct `MEILISEARCH_INDEX` (files), `MEILISEARCH_POSTS_INDEX` (default
`posts`), and `MEILISEARCH_PROFILES_INDEX` (default `profiles`). Search uses
`MEILISEARCH_URL` and `MEILISEARCH_API_KEY`. Browser/native clients never receive
the engine credential. Run `just manage initialize-integrations` explicitly.
Startup does not provision or mutate integration schemas.

Post/profile edits enqueue jobs in the business transaction. Organization member
changes enqueue profile refresh when a profile exists. Workers wait for engine
tasks and compare current projection facts after network I/O. A changed revision,
membership, or account state causes another synchronization job. Periodic bounded
repair scans include unchanged and inactive/deleted rows so an empty or stale
index can converge after an outage.

## Numbered search pages

Post and user search accept `q` (1–200 characters), `page` (default 1), and
`per_page` (default 25, maximum 100). The requested starting offset must be less
than 1,000. Lists use cursors separately.

The engine returns at most 1,000 ranked candidate IDs/revisions. Before page
selection the API deduplicates IDs and hydrates current authorized rows in one
PostgreSQL query. Posts must still be published in the requested organization.
Profiles must still belong to an active, verified current member. The engine
revision must equal the current PostgreSQL revision. This check prevents an old
search term from revealing a renamed or edited row.

The response contains `items`, `page`, `per_page`, `total_in_window`,
`search_window_limit: 1000`, and `has_next_page`. Counts refer only to the
authorized, current rows in that bounded ranked window. They are not a global
exact total or raw Meilisearch count. There may be additional authorized matches
beyond the window even if filtering leaves fewer than 1,000 visible rows.
No engine snippets, highlights, or hidden-result counts are returned. Ranking
and pages may change between requests as indexing catches up.

Search returns 503 when unconfigured or unavailable. Profile/post CRUD and
notifications do not require search. Existing filename search retains its
separate maximum-20 response contract.

## References

- [Meilisearch search API](https://www.meilisearch.com/docs/reference/api/search/search-with-post)
- [Meilisearch asynchronous tasks](https://www.meilisearch.com/docs/capabilities/indexing/tasks_and_batches/async_operations)
