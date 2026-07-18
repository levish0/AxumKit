---
title: 인가 (RBAC)
description: 역할, 권한, ACL 그룹 — Django의 의미론을 Rust로 구현.
order: 6
---

인가는 세 개의 계층으로 구성되며, 모두 요청 단위의 `UserContext`로 해석됩니다.

## 역할(Roles)

`user_roles`는 `role` enum(`mod`, `admin`)에서 오는 거친 단위의 권한 부여를 담으며,
선택적으로 `expires_at`을 가집니다(읽기 시점에 필터링). 라우트 수준의 게이트
(`require_admin` / `require_mod`)는 grep으로 쉽게 찾을 수 있도록 라우터 경계에 위치하며,
`Admin`은 모든 역할 검사를 통과합니다.

역할 관리에는 내장된 메타 규칙이 있습니다. 자기 자신을 관리할 수 없고, 관리자는 다른
관리자를 차단하거나 강등할 수 없으며, `Admin` 역할 자체는 API를 통해 절대 부여하거나
회수할 수 없습니다 — 권한 상승과 마지막 관리자 잠금(lockout)을 모두 차단합니다.

## 권한(Permissions)

세분화된 기능 권한은 **permission codename**으로, `constants`의 `Permission` enum에
한 번만 정의되며 권한이 부여되는 모든 곳에 TEXT로 저장됩니다:

- `board:pin_post` — 게시글 고정/해제/고정 순서 변경
- `board:lock_post` — 게시글 댓글 스레드 잠금/해제
- `board:moderate` — 다른 사용자의 게시글과 댓글 삭제/숨김
- `board:manage` — 게시판 자체의 생성/수정/삭제

`UserContext::has_perm`은 다음 순서로 해석합니다: **차단(ban) 하드 게이트**(차단된
사용자는 어떤 권한도 갖지 않음), **Admin 우회**(관리자는 모든 검사를 통과 — 잠금 방지),
**Mod 기본 세트**(`pin`, `lock`, `moderate`), 마지막으로 그룹 멤버십을 통해 부여된
권한의 합집합. 거부 시에는 `acl:denied` 코드와 누락된 codename을 담아 `403`을
반환하므로, 클라이언트는 정확히 어떤 권한이 없었는지 알 수 있습니다.

권한 추가는 코드 변경만으로 끝납니다 — 새로운 enum variant 하나면 되고, 마이그레이션은
필요 없습니다. 더 이상 파싱되지 않는 저장된 codename은 어떤 검사와도 일치하지 않습니다
(fail closed, 로그 기록).

## ACL 그룹

그룹은 Django의 그룹 모델을 따릅니다: 멤버십 메타데이터를 가진 이름 있는 권한 묶음입니다.

| 테이블 | 의미 |
| --- | --- |
| `acl_groups` | 이름, 설명, `is_system` (시스템 그룹은 API를 통해 변경 불가) |
| `acl_group_members` | 행마다 사용자 한 명(또는 IP/CIDR), `reason`, `expires_at`, `created_by` 포함 |
| `acl_group_permissions` | 행마다 codename 하나, 그룹별로 유니크 |

멤버십 만료는 읽기 시점에 필터링되므로, 일시적인 권한 부여(예: "30일간
trusted-uploaders")에는 별도의 회수 작업이 필요 없습니다 — 주간 크론이 죽은 행을
정리할 뿐입니다.

## 관리자 API

모든 변경 작업은 관리자 전용이며, 필수 사유(reason)와 함께 모더레이션 로그에 기록되고,
읽기에는 최소 `Mod`가 필요합니다:

```
GET  /v0/acl/groups                       list groups
POST /v0/acl/groups                       create (name, description, reason)
POST /v0/acl/groups/delete                delete (system groups refused)
GET  /v0/acl/groups/members?group_id=…    list members (active only, paginated)
POST /v0/acl/groups/members               add member (user or ip, reason, expires_at)
POST /v0/acl/groups/members/remove        remove member
GET  /v0/acl/permissions                  every codename the app defines
GET  /v0/acl/groups/permissions?group_id=…
POST /v0/acl/groups/permissions/replace   whole-list replacement; unknown codenames rejected
```

권한 교체는 전체 목록 방식("원하는 최종 상태를 제출")이므로, 관리자 UI는
`GET /v0/acl/permissions`로 체크박스를 렌더링하고 diff 계산 없이 결과를 그대로
다시 제출할 수 있습니다.

## 도메인 정책 객체

서비스 코드에 역할 검사를 인라인으로 흩뿌리지 않습니다. 각 도메인은 `Rule` 트레이트를
구현하는 정책 enum을 정의합니다 — 게시판의 경우 `BoardPermission`:

```rust
BoardPermission::Write(facts).check(&ctx)?;          // authed + unbanned + board enabled
BoardPermission::EditContent { is_owner, facts }     // owner-only, held to the write bar
BoardPermission::DeleteContent { is_owner }          // owner, or board:moderate
BoardPermission::PinPost                             // board:pin_post
BoardPermission::ManageBoard                         // board:manage
```

이 규칙들은 지켜둘 가치가 있는 정책 결정을 담고 있습니다: 읽기는 공개이며 차단의
영향을 받지 않고(차단은 참여를 막을 뿐 읽기를 막지 않음), 비활성화된 게시판은 관리자를
제외한 모두에게 404를 반환하며, 모더레이터는 콘텐츠를 제재할 수는 있어도 다른 사람의
글을 고쳐 쓸 수는 없습니다.
