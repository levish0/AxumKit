# Files and search

Files are organization attachments. Original bytes live in a private SeaweedFS
S3 bucket. PostgreSQL owns UUIDv7 metadata, uploader identity, organization scope,
quota reservations, and state. A filename is display metadata, never an object
key or filesystem path. Uploads do not execute, unpack, sanitize, scan, or convert
content. `ready` means storage completed. It does not establish that the content is harmless.

## Upload and access

Reserve metadata through the authenticated organization API. Request an upload
URL, PUT raw bytes directly to S3, then request completion. The API never receives
or proxies the original bytes. These operations use the file UUID:

1. `POST /api/v1/organizations/{organization_id}/files`: reserve declared bytes.
2. `POST .../files/{file_id}/upload-url`: receive `url`, `method`, `headers`, and
   `expires_at`. Send each returned header unchanged. Send no API cookie or bearer
   token to storage. Browser clients use an exact File/Blob body; the browser sets
   its signed Content-Length. Do not manually set that forbidden header.
   Native clients must send the exact reserved Content-Length, including zero.
3. `POST .../files/{file_id}/complete-upload`: verify stored metadata and mark Ready.
4. `POST .../files/{file_id}/download-url` with `{"method":"GET"}` or
   `{"method":"HEAD"}`: receive a method-specific direct storage request.

Current membership and session authority are required for URL issuance,
completion, metadata, and deletion. Only the uploader may obtain an upload URL or
complete it. The uploader or organization owner may delete an attachment while
still a member. A file UUID does not grant access. Reissuing an upload URL does not
extend the reservation deadline. Completed-file retries are idempotent, but still
check the actor and current membership. Parallel completion publishes once.

The service reserves declared bytes under the organization lock. The PUT signature
binds its generated key, exact Content-Length, If-None-Match `*`, octet-stream type,
identity encoding, attachment disposition, and no-store. Storage must reject
changed signed headers, unsigned public ACLs, and concurrent overwrites. Completion
checks HEAD metadata outside locks, then repeats authority, deadline, and state
checks under account -> organization -> file locks. It does not download the file.
`content_hash` remains null: HEAD does not provide a verified BLAKE3 digest.
Originals remain unchanged and unprocessed.

Defaults are 100 MiB per file, 1 GiB/10,000 live files per organization, 100 pending
reservations, and 30 reservations per uploader per minute. Zero-byte files count
against metadata and reservation-rate limits. Upload URLs last 300 seconds;
download URLs last 60 seconds. Reservations last at least 15 minutes. These limits
are configurable. States remain `uploading -> transferring -> ready -> deleting
-> deleted`; Transferring means an upload URL has been issued.

Presigned URLs are bearer credentials. Anyone holding one can use it until expiry.
Revocation, membership removal, and deletion deny new URL issuance and completion.
They do not individually revoke an issued URL. Expiry prevents new requests; an
already started transfer can continue. A loaded image does not disappear at URL
expiry. Clients request a fresh authorized URL before a later read or after an
expired request fails. Do not log URLs or include their queries in access logs,
error reporting, analytics, or durable records.

Deletion immediately denies new API access and enqueues durable byte removal.
Keep Deleting rows and their quota reserved until the upload reservation deadline
plus `FILE_UPLOAD_GRACE_SECONDS` has passed. Its default is one hour. The storage
ingress must impose a total PUT duration no greater than this grace. Delete again
before marking Deleted. This prevents late writes from escaping quota accounting.
Retained tombstones repeat cleanup at most once per hour. Default retention is
30 days; purge requires the writer horizon to have passed, recent successful
storage deletion, and no outstanding index-deletion obligation. Disabling search
does not erase that obligation.

Storage GET/HEAD uses application/octet-stream, attachment disposition, identity
encoding, and no-store. GET supports the provider's byte-range behavior. The API
no longer parses Range or If-Range and does not enforce its previous single-range
policy. HEAD and GET require separate signatures. Set `X-Content-Type-Options:
nosniff` on the storage ingress: the API's JSON response headers cannot protect
a separate storage response. The baseline is original retrieval; a consuming app
must define a separate safe inline-image policy. Do not trust a declared media
type to enable HTML/SVG execution.

`S3_ENDPOINT` is the API's internal storage endpoint. Optional `S3_PUBLIC_ENDPOINT`
is the client-reachable endpoint used for signatures; it defaults to S3_ENDPOINT.
Do not rewrite the signed host, path, or query after issuance. Keep the bucket
private and anonymous requests denied. `manage initialize-integrations` replaces
the bucket CORS rules using ALLOWED_ORIGINS, allowing PUT/GET/HEAD and required
headers. This is an explicit provisioning operation, not startup behavior.

## Derived filename search

Meilisearch indexes file IDs, organization IDs, and filenames, not file contents.
Keys stay server-side. Organization filters come from the authorized route scope.
Search returns candidate IDs. The service hydrates current ready rows from
PostgreSQL and checks membership again before returning metadata. Index totals
and snippets are not exposed. Membership removal and file deletion take effect
even when the index is stale.

Ready/deleting transitions enqueue indexing jobs in their DB transaction. Workers
wait for each asynchronous Meilisearch task to complete, then recheck current
state/revision and repeat reconciliation when necessary. An ambiguous network
failure is retryable. Maintenance can enqueue a bounded repair batch to correct
late writes or an index rebuilt after data loss. Searches are eventually
consistent. PostgreSQL is always authoritative for access.

## Integration boundary

The example scope is an organization. A messenger must authorize attachments
through current conversation/message access in its owning service. Original
upload ownership must not become a permanent bypass for parent permissions.

- [OWASP file upload guidance](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html)
- [AWS Rust S3 examples](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/rust_s3_code_examples.html)
- [AWS presigned URL behavior](https://docs.aws.amazon.com/AmazonS3/latest/userguide/using-presigned-url.html)
- [Fetch request body length](https://fetch.spec.whatwg.org/#http-network-or-cache-fetch)
- [SeaweedFS S3 API](https://github.com/seaweedfs/seaweedfs/wiki/Amazon-S3-API)
- [Meilisearch asynchronous tasks](https://www.meilisearch.com/docs/capabilities/indexing/tasks_and_batches/async_operations)
