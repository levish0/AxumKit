# Regression inventory

The table lists every reviewed reference E2E test. A replacement preserves the
stated invariant in the current domain. An exclusion records a scope decision.
It does not indicate a passing test or an implemented feature. Record test
execution and limitations in [milestones](../memory/README.md).

Current suites: [identity](../tests/identity.rs), [API](../tests/api.rs),
[OIDC](../tests/oidc.rs), [jobs](../tests/jobs.rs),
[files/search](../tests/files.rs), [connected content](../tests/content.rs), and
[process/SMTP/load](../tests/process.rs).
Identity includes [privileged authentication and retention](../tests/identity/hardening.rs);
jobs include [retention races](../tests/jobs/retention.rs) and
[dispatch capacity and idle polling](../tests/jobs/dispatch.rs) across independent worker pools.
Core policy, encryption, mail rendering,
pagination, proxy trust, and Sentry controls also have workspace unit tests.

[Schema consistency](../tests/schema_consistency.rs) compares migrated tables,
columns, types, nullability, primary keys, and registered string-enum CHECK values.
It also verifies failure detection with deliberate database changes.

| Reviewed test | Disposition |
| --- | --- |
| `account.rs::email_change_requires_password_and_confirmation` | Excluded: changing account email is outside the baseline; a future implementation must require reauthentication and confirmation. |
| `account.rs::account_deletion_kills_login_and_masks_profile` | Session/login invalidation: `session_ownership_password_change_and_deactivation_recheck_authority`; public profiles are excluded. |
| `account.rs::deletion_confirm_rejects_garbage_token` | Changed: deactivation is a recent-authenticated service operation; no deletion email challenge or public profile. |
| `auth.rs::password_reset_invalidates_sessions_and_old_password` | `password_reset_consumes_once_and_revokes_every_session`; separate-process reset also invalidates credentials on both APIs. |
| `auth.rs::app_login_enforces_new_device_verification` | Explicitly excluded by owner decision: use the common MFA gate; no default new-device email challenge. |
| `auth.rs::session_listing_and_revocation_are_per_user` | `session_ownership_password_change_and_deactivation_recheck_authority` and `session_rotation_is_single_use_and_revocation_is_shared`. |
| `auth.rs::change_password_requires_the_current_password` | `session_ownership_password_change_and_deactivation_recheck_authority`. |
| `auth.rs::logout_invalidates_the_current_session` | `two_applications_share_sessions_but_keep_http_configuration_separate` revokes the current session and checks the other instance. |
| `auth.rs::resend_verification_email_is_pending_only_and_enumeration_safe` | `verification_resend_is_pending_only_and_replaces_the_previous_challenge`; HTTP uses the same accepted response. |
| `board.rs::seeded_boards_are_publicly_listed` | Excluded: public boards and seeded catalogs. The current post example is organization-scoped. |
| `board.rs::board_management_is_admin_only` | Excluded: separate board administration. Cedar author/organization-owner rules govern the content example. |
| `board.rs::posting_requires_login` | Replaced by the production-router content contract test; unauthenticated posting is rejected. |
| `board.rs::post_update_and_delete_respect_ownership_and_moderation` | Replaced by content role/version/membership and concurrent-write tests; authors and organization owners manage posts. Moderator roles remain excluded. |
| `board.rs::post_requires_title` | Replaced by content HTTP/domain validation; blank titles are rejected. |
| `board.rs::comment_create_and_list` | Replaced by the content inbox lifecycle and k6 workflows through published posts. |
| `board.rs::comments_paginate_oldest_first_regardless_of_direction` | Changed: immutable descending (created_at, id) cursors. Timestamp ties and scope changes are covered by the content suite. |
| `board.rs::reply_depth_is_capped_at_two` | Changed: single-level comments only; nested reply inputs are not part of the contract. |
| `board.rs::comment_list_focus_anchor_returns_containing_window` | Excluded: focus anchors. The baseline provides scoped forward cursor traversal. |
| `board.rs::comment_count_tracks_creates_and_deletes` | Excluded: denormalized comment counts; comment rows are authoritative. |
| `board.rs::comment_update_and_delete_respect_ownership_and_moderation` | Deletion is covered by content role and concurrency tests. Comment editing and moderator roles are excluded. |
| `board.rs::moderators_cannot_edit_others_content` | Changed: there is no moderator role. Ordinary members cannot edit other authors' posts; organization-owner authority is explicit. |
| `board.rs::locked_post_blocks_new_comments` | Excluded: a separate locked-post state. Draft/deleted parents cannot accept comments. |
| `board.rs::pinned_posts_ride_along_on_every_page` | Excluded: pinning and pin-aware pagination. |
| `board.rs::pinned_posts_reorder_to_the_order_sent` | Excluded: pin ordering. |
| `board.rs::pin_reorder_requires_board_moderation` | Excluded: pin ordering and moderator roles. |
| `board.rs::pin_reorder_rejects_a_stale_pin_set` | Excluded: pin ordering. Current post/profile writes have explicit optimistic versions. |
| `moderation.rs::banned_user_loses_board_write_access_until_unbanned` | Excluded: content moderation is a product feature. Active-account denial and Cedar organization scope remain baseline security contracts. |
| `moderation.rs::admins_cannot_ban_themselves_or_other_admins` | Excluded: moderation API policy is a product feature. Global administrator changes are operator-only, with MFA eligibility and credential revocation covered by `privileged_accounts_require_mfa_at_login_session_and_final_authority`. |
| `moderation.rs::role_management_rejects_self_and_admin_targets` | Excluded: public role-management API policy. The baseline has an operator-only global administrator command, not user-facing role delegation. |
| `moderation.rs::moderation_logs_require_moderator_role` | Excluded: moderation log APIs are a product feature. Runtime audit access is append-only; age-based deletion is a restricted maintenance operation. |
| `notification.rs::inbox_requires_login` | Replaced by session extraction on all inbox routes and production-router/load requests. |
| `notification.rs::comment_notifies_post_author_and_inbox_lifecycle_works` | Replaced by inbox/preferences/read/visibility tests and the two-API k6 content workflow. |
| `notification.rs::mentions_in_posts_and_comments_notify_mentioned_users` | Excluded: mention parsing and mention notifications. |
| `notification.rs::sec_007_notifications_are_scoped_per_user` | Replaced by inbox owner-isolation and removed-member visibility/count tests. |
| `notification.rs::action_preference_opt_out_suppresses_new_notifications` | Replaced by transactional comment/inbox preference tests. |
| `notification.rs::notification_preferences_require_login_and_are_per_user` | Replaced by authenticated per-account preferences in the content inbox lifecycle test. |
| `rbac.rs::permissions_catalog_is_mod_readable_and_lists_board_codenames` | Excluded: generic permission catalog and group administration. The baseline uses validated Cedar policies with explicit organization membership scope. |
| `rbac.rs::group_management_is_admin_gated` | Excluded: generic permission catalog and group administration. The baseline uses validated Cedar policies with explicit organization membership scope. |
| `rbac.rs::group_permission_grant_gates_post_pinning` | Excluded: generic permission catalog and group administration. The baseline uses validated Cedar policies with explicit organization membership scope. |
| `rbac.rs::membership_removal_revokes_the_grant` | Excluded: generic permission catalog and group administration. The baseline uses validated Cedar policies with explicit organization membership scope. |
| `rbac.rs::membership_expiry_ends_the_grant` | Excluded: generic permission catalog and group administration. The baseline uses validated Cedar policies with explicit organization membership scope. |
| `rbac.rs::unknown_permission_codename_is_rejected` | Excluded: generic permission catalog and group administration. The baseline uses validated Cedar policies with explicit organization membership scope. |
| `rbac.rs::system_groups_are_immutable_via_the_generic_api` | Excluded: generic permission catalog and group administration. The baseline uses validated Cedar policies with explicit organization membership scope. |
| `search.rs::user_search_indexes_new_signups` | Changed: profiles are explicit and discoverable only within shared organization membership. Profile saves and membership changes enqueue projections. |
| `search.rs::profile_update_reindexes_user_search` | Replaced by stale-profile revision and membership-projection tests plus the two-API k6 workflow. |
| `search.rs::account_deletion_removes_user_from_search` | Replaced by current active/verified account filtering during scoped search hydration; stale indexed rows cannot reveal suspended profiles. |
| `smoke.rs::health_check_returns_no_content` | `storage_failure_is_not_treated_as_anonymous` and process readiness probes. |
| `smoke.rs::signup_verify_login_flow` | `signup_mail_login_cross_process_revocation_and_worker_restart`: actual SMTP and two API processes. |
| `smoke.rs::duplicate_handle_is_rejected` | Changed: no handles; normalized email uniqueness and generic duplicate acceptance in `registration_persists_encrypted_mail_and_durable_job`. |
| `smoke.rs::login_with_wrong_password_is_unauthorized` | `failed_authentication_attempts_survive_rollback_across_instances` and `session_ownership_password_change_and_deactivation_recheck_authority`. |
| `smoke.rs::profile_image_upload_is_processed_to_webp` | Excluded by owner decision: media processing is not in the baseline. Original attachments use private S3 storage without content conversion. |
| `totp.rs::login_with_totp_requires_second_factor` | `totp_and_recovery_codes_are_single_use_across_challenges`; OIDC gate in `explicit_linking_does_not_merge_email_and_oidc_cannot_bypass_mfa`. |
| `totp.rs::sec_002_backup_code_is_single_use_under_concurrency` | `totp_and_recovery_codes_are_single_use_across_challenges` races the same recovery code through different challenges. |
| `totp.rs::status_backup_code_rotation_and_disable_flow` | Enrollment, recovery consumption, status, and removal: `totp_and_recovery_codes_are_single_use_across_challenges`. Replacement and credential revocation: `concurrent_recovery_regeneration_returns_one_generation_and_revokes_credentials`. TOTP replay across replacement: `recovery_regeneration_keeps_the_consumed_totp_step_after_version_change`. |
| `user_public.rs::handle_availability_reflects_signups` | Excluded: handles and public profiles are outside the baseline. Private account DTOs require owner authentication. |
| `user_public.rs::public_profile_never_exposes_the_email` | Excluded: handles and public profiles are outside the baseline. Private account DTOs require owner authentication. |
| `user_public.rs::public_profile_by_id_matches_handle_lookup` | Excluded: handles and public profiles are outside the baseline. Private account DTOs require owner authentication. |
| `user_public.rs::banned_user_profile_reports_ban_status` | Excluded: handles and public profiles are outside the baseline. Private account DTOs require owner authentication. |
| `user_public.rs::deactivated_profile_masks_private_fields_but_keeps_identity` | Excluded: handles and public profiles are outside the baseline. Private account DTOs require owner authentication. |
| `worker_consumer.rs::slow_successful_handler_runs_exactly_once` | `heartbeat_keeps_a_slow_successful_handler_exclusive`; proves claim exclusivity under the tested lease, not exactly-once external effects. |
| `worker_consumer.rs::failed_handler_is_redelivered_then_succeeds` | `retry_delay_and_success_retention_preserve_failed_jobs` and `final_attempt_crash_becomes_retained_failure_and_can_be_retried`. |

## Additional baseline regressions

- Cloudflare client IP remains stable across proxy Pod replacement. Only a
  matching shared proxy secret permits SSR client-IP overrides. Selected IP
  headers reject invalid addresses, lists, and duplicates; IPv4-mapped addresses
  share the IPv4 budget. SSR headers without a matching secret cannot override it.
- Atomic caller transaction rollback includes domain rows and durable jobs.
- Rotation/reset/recovery races cannot reuse credentials or challenges.
- Authentication cleanup preserves an idle extension committed while deletion
  waits on its row lock, while still deleting sessions past absolute expiry.
- Client and identity limits survive failures and are shared across instances.
- Configurable limits above the former saturation ceiling remain effective, and
  an instance with a lower limit cannot truncate shared attempts.
- Global administrator promotion requires confirmed MFA and invalidates old
  credentials; primary login, session authentication, and final service checks
  deny an administrator without MFA. Administrator MFA removal is forbidden.
- Recovery-code regeneration requires recent authentication and a fresh proof;
  concurrent replacements leave one returned generation and revoke old credentials.
- An OIDC-only account adds a first password only with recent authentication.
  The change advances the security version, revokes every session and challenge,
  and registers the security notification. An existing password returns a
  conflict, and concurrent requests store one password.
- JSON/query validation reuses domain rules, rejects invalid input before the
  handler, and does not expose submitted secrets in error responses.
- Optional Turnstile verification uses a local fake siteverify server and no live
  network call. It covers acceptance, missing and rejected tokens, hostname and
  action mismatches, a retry that reuses its idempotency key, provider timeouts
  and 5xx responses, and unchanged behavior when it is not configured.
- Audit retention deletes bounded old batches while preserving recent events.
- Finished-job retention preserves rows retried or failed again while deletion
  waits, and removes attempts only with an eligible retained job.
- File cleanup's successive updates can finish while an authorized deletion
  holds canonical parent locks and waits for the file row. A synchronized
  PostgreSQL regression covers the foreign-key lock cycle found by mixed load.
- OIDC checks browser binding, state replay, nonce, signed key rotation, provider outages, explicit linking, and the common MFA gate.
- Native OIDC ID token login checks opt-in providers, Android and iOS audience
  shapes, unconfigured `aud`/`azp`, missing or different nonces, expiry, issuer,
  unpublished keys, malformed tokens, flow consumption on rejection, provider
  mismatch, browser/native flow separation, the common MFA gate, expired-flow
  rejection and cleanup, and bearer-only HTTP responses without an Origin or
  cookie (`tests/oidc/native.rs`).
- Worker crash recovery preserves attempt history, honors retry delay, and fences stale completion; a hung heartbeat/handler cannot hold a lease forever.
- Workers cannot observe an uncommitted business change or its job. Rollback
  removes both. Saturated handlers release claim locks and leave excess jobs
  pending without prefetch. Finished-history cleanup drains successive batches
  while preserving ready, running, and recent jobs (`tests/jobs/pressure.rs`).
- PostgreSQL job load verifies source/job/attempt/effect counts across 1/2/4 worker
  pools and retained history. Separate bulk/notification wait and foreground
  database latency measurements are diagnostic evidence, not production capacity
  gates (`tests/job_load.rs`).
- Database outages return unavailable; HTTP configuration is isolated across application instances.
- Real Mailpit delivery checks HTML/text content, worker replacement, cross-process revocation, and absence of secrets in captured process logs.
- k6 uses distinct accounts and organizations across two API processes for scoped
  pagination, optimistic writes and stale-write rejection, cross-tenant denial,
  session rotation/revocation, and original-file upload/download/ranges/search/
  deletion. Every virtual user must complete a workflow; unexpected HTTP failures,
  failed checks, and workflow failures are forbidden. HTTP p95 must be below two
  seconds and filename visibility must remain below its ten-second bound.
  Post-load checks require completed deletion and no failed background attempts,
  so retryable worker defects cannot hide behind successful HTTP responses.
