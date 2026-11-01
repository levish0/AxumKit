# Posts and comments

The example is an organization board with plain-text posts and single-level
comments. All entity IDs are generated with UUIDv7. This is ordinary application
code that a consuming project can replace while keeping the shared services.

## Membership and authority

Only current active, verified session actors can enter organization operations.
Owners can add an existing verified account by UUID through
`PUT /api/v1/organizations/{organization_id}/members/{account_id}` and remove a
member through DELETE. Both require recent authentication. Adding an existing
member is idempotent. Owners cannot remove themselves. Each organization allows
at most 1,000 members. This small direct membership API does not implement
email invitations, self-join, or ownership transfer.

The service locks actor account, organization, post, comment, then attachment
files in UUID order as needed. Parent locks use NO KEY UPDATE. Membership is
checked under the organization lock before Cedar evaluates content facts.

| Operation | Cedar rule |
| --- | --- |
| Create a post | Current organization member; author is the actor |
| Read published post | Current organization member |
| Read a draft | Current member who is its author or the organization owner |
| Update/delete a post | Current member who is its author or the organization owner |
| Create/list comments | Published, nondeleted parent post in the current organization |
| Delete a comment | Current member who is the comment author, post author, or organization owner; parent remains published |
| External posts:read | Admitted API client; same organization; published, nondeleted post only |

Global administrator status does not bypass organization membership. Hidden
resources return 404. An accessible resource with a forbidden mutation returns
403. Deleted posts and comments disappear from normal queries.

## Writes and attachments

Posts have title, text body, draft/published state, attachment UUIDs, and version.
Title is 1–200 characters, body 1–8,000 characters and at most 12,000 UTF-8 bytes.
Comments contain 1–2,000 characters and at most 8,000 UTF-8 bytes. The shared 16 KiB
HTTP body limit still applies. Clients render these fields as text.

Creating a post requires `expected_version: 0`. Full updates require the current
version. Post and comment deletion require `expected_version` in the query.
Conflicts return 409. Deletion leaves a tombstone and clears the deleted row's
text. The baseline does not provide wiki revision history or restoration.

Up to ten distinct ready files from the same organization may be attached.
The owning file service validates and locks them before replacing associations.
Files retain their independent organization-wide access: connecting an original
to a draft does not make the file private to the draft author. Missing/deleting
files disappear from returned attachment lists. Purging a file tombstone cascades
only its associations and cannot delete the post.

## Lists, search, and notifications

Post, comment, profile, and notification lists use descending immutable
`(created_at, id)` keyset ordering. Cursors bind the actor, organization/resource,
state filter, and ordering. Page size defaults to 25 and is limited to 100.
Queries fetch limit+1 rows to determine whether another page exists. New insertions
do not shift older pages. Visibility changes can still remove rows between requests.

The existing session/organization/file/API-client lists retain ascending UUID
cursors. See [API pagination](../api.md) for the separate contracts.

Published post changes commit a search job in the same transaction. Search never
grants access. See [profiles and search](profiles-and-search.md). A comment by
someone else commits the author's notification and eligible push jobs with the
comment. See [notifications and push](notifications-and-push.md).
