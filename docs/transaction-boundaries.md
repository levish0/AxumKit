# Transaction boundaries

An application service starts and commits its transaction. Repository writes
accept that transaction, not a database pool. A composed operation uses the
owning service's explicit `*_in_transaction` operation, as demonstrated by
organization creation. Returning an error drops and rolls back the transaction.

## Lock order

Lock the acting account first, then the domain resource. Re-read the current
session, token hash, account flags, security version, and required recent
authentication under that account lock before sensitive operations. Organization
mutations then lock the organization and check membership, policy, and version.
Never reverse this order in another endpoint or worker.

Canonical account and organization locks use `FOR NO KEY UPDATE`. They serialize
business mutations while allowing the `FOR KEY SHARE` existence checks taken by
child foreign keys. File workers can therefore finish lifecycle updates while
an authorized request holds its parent locks and waits for the file row. Using
`FOR UPDATE` for those parent reads creates a cycle with the child's implicit
foreign-key locks, including checks from successive updates in one transaction.
Keep file lifecycle locks and final authority checks in place.

Current parent mutations change flags, credentials, names, and versions. Adding
parent deletion or changes to a foreign-key-eligible unique key, including the
unique account email, requires a new lock-order review. PostgreSQL can acquire
`FOR UPDATE` for those writes even when all actual references use UUIDs. See
[PostgreSQL row locks](https://www.postgresql.org/docs/current/explicit-locking.html#LOCKING-ROWS).

Find an email or MFA challenge before locking its account. Under the account
lock, re-read the challenge, check expiry, and consume it conditionally. TOTP
replay state belongs to the account, not an individual challenge. Recovery code
consumption is conditional and single use. Bind OIDC callback state to a browser.
Consume that state before exchanging the code with the provider.
An ambiguous exchange failure requires a new authorization flow. Native ID token
login deletes and returns its flow in one committed statement before token
verification or key refresh. The account transaction starts only after the
token is verified.

Password verification and hashing run on bounded blocking tasks before database
locks. After locking, compare the password snapshot and security version again.
Network requests, provider discovery, SMTP, and template rendering do not run
while holding an account or resource lock.

## Durable effects

Signup and recovery commit account/challenge state, an encrypted delivery, and a
durable job together. The job contains a delivery UUID, not a token, recipient,
or mail body. The worker renders and sends after the transaction commits.

SMTP acceptance and a database commit cannot be atomic. A crash between them can
send the same mail again. Preserve the delivery's Message-ID and describe this as
at-least-once delivery. A stable identifier does not prove recipient deduplication.
The action itself remains single use. Check authority when consuming it.

Rate-limit counters commit in their own short transaction. A later invalid
password, challenge, or rolled-back business change must not refund the attempt.
Job claims and completion have their own short transactions and use claim-token
fencing, database time, leases, and hard execution deadlines.

External API admission has a separate accounting contract. It first rejects an
incorrect secret without authority locks, then locks client and key in that
order and repeats mutable checks using database time. Both request counters and
one admission record commit together. A committed admission remains charged if
response delivery subsequently fails. Human management locks account,
organization, client, then key. External organization reads take no subsequent
organization lock. See [API access](behavior/api-access.md) before adding external
writes, organization deletion/suspension, or new billing semantics.

Content mutations lock actor account, organization, post, then comment or sorted
attachment files as needed. The caller's transaction includes post revisions,
attachment associations, search jobs, and comment notification/delivery jobs.
Comment notification registration reads recipient eligibility without taking a
second account lock after the organization. Push execution instead starts with
the recipient account, then organization and device, rechecks current access and
session/device generation, commits, and sends a generic provider payload outside
locks. Provider acceptance and durable completion have the same at-least-once
boundary as mail. See [content](behavior/posts-and-comments.md) and
[notifications](behavior/notifications-and-push.md).

Use `tests/content.rs` for content versions, membership, scoped search/inbox
visibility, and atomic notification registration,
`tests/identity.rs` for atomic composition and credential races,
`tests/jobs.rs` for lease and retry behavior, and `tests/process.rs` for the
separate-process SMTP boundary. Do not replace these with database mocks.
