---
title: 게시판 및 알림
description: 데모 도메인 — 게시글, 댓글, 고정 게시글, 그리고 알림함.
order: 7
---

게시판 도메인은 템플릿의 모든 패턴이 처음부터 끝까지 동작하는 모습을 보여주기 위해
존재합니다. RBAC로 보호되는 쓰기 작업, 소유자 검사, 모더레이션, 카운터, 백그라운드
팬아웃, 알림이 여기에 포함됩니다. `notice`, `general`, `qna` 세 개의 게시판이 시드로
생성됩니다.

## 게시판, 게시글, 댓글

```
GET  /v0/board/list, /v0/board/by-slug?slug=…
POST /v0/board, /board/update, /board/delete        (board:manage)

POST /v0/board/post                                 create (authed, unbanned)
POST /v0/board/post/update                          owner-only
POST /v0/board/post/delete                          owner or board:moderate
POST /v0/board/post/pin | unpin | reorder-pins      board:pin_post
POST /v0/board/post/lock | unlock                   board:lock_post
GET  /v0/board/post/list, /v0/board/post            reads (public)

POST /v0/board/comment, /comment/update, /comment/delete, GET /comment/list
```

실제 프로젝트에도 그대로 적용할 수 있는 세부 사항:

- **콘텐츠는 원문 그대로 저장됩니다** — 서버 측 마크업 파이프라인이 없습니다.
  클라이언트에서 렌더링하거나, 응답 매퍼가 있는 위치에 자체 렌더러를 연결하면 됩니다.
- **답글 깊이는 2로 제한됩니다** (YouTube 방식): 답글에 대한 답글은 같은 스레드
  루트에 붙기 때문에 페이지네이션과 답글 카운트가 단순하게 유지됩니다.
- **고정된 게시글**은 명시적인 위치 값을 가집니다. `reorder-pins`는 원하는 전체
  순서를 받으며, 오래된 집합은 거부하므로(`board:pin_set_mismatch`) 두 명의
  모더레이터가 서로의 변경을 조용히 덮어쓰는 일이 발생하지 않습니다.
- **조회수는 버퍼링됩니다**: 각 조회자는 6시간 윈도우당 한 번만 집계되고(사용자 id
  또는 IP 기준으로 Redis에서 중복 제거), 증가분은 Redis 해시에 쌓이며, 워커 크론이
  1분마다 해시를 원자적으로 비워 `board_posts.view_count`에 반영합니다. Redis
  장애가 읽기 요청을 실패시키는 일은 절대 없습니다.
- 게시글과 댓글은 **액터**(user/anonymous/system)에 귀속됩니다. 덕분에 탈퇴한
  사용자의 콘텐츠도 마스킹된 신원으로 유지될 수 있습니다.

## 멘션

게시글/댓글 본문의 `@handle` 토큰은 사용자로 해석됩니다(본문당 최대 10개). 해석된
멘션마다 `user_mentioned` 알림이 생성되며, 작성자 본인은 제외됩니다. 수정 시에는
**새로 추가된** 멘션에만 알림이 발송됩니다 — 게시글을 다시 저장해도 이미 멘션된
사람들에게 알림이 재발송되지 않습니다.

## 알림함

알림은 두 개의 테이블로 구성됩니다. 이벤트당 하나의 `notification_events` 행과
수신자당 하나의 `notification_deliveries` 행이며, 읽음 상태는 delivery 쪽에
저장됩니다. 이벤트의 대상 컬럼(`board_id`/`post_id`/`comment_id`)은 `target_kind`에
따라 데이터베이스가 형태를 검증하므로, 잘못된 형태의 이벤트는 애초에 삽입될 수
없습니다.

현재 알림을 생성하는 이벤트는 `board_comment_created`(내 게시글에 댓글이 달림)와
`user_mentioned` 두 가지입니다. 모든 알림은 하나의 관문(`service_notify_user` /
`notify_mentions`)을 거치며, 이 관문에서 자기 자신에 대한 알림을 제거하고 액션별
수신 거부 설정을 반영합니다.

```
GET  /v0/notifications/list                cursor-paginated, filterable
GET  /v0/notifications/unread/count
POST /v0/notifications/mark-as-read, /mark-all-as-read, /delete
GET|PUT /v0/notifications/preferences      channel flags
GET|PUT /v0/notifications/preferences/actions   per-action opt-out
```

오래된 알림은 주간 정리 크론이 회수합니다(90일 보관).

## 검색

사용자 검색은 Meilisearch에서 제공됩니다(`GET /v0/search/users`). 인덱스 스키마는
공유 `search_index` 크레이트입니다. 워커가 가입 및 프로필 변경 시 `SearchUser`
문서를 기록하고, 서버는 동일한 구조체를 읽어오며, 전체 재인덱싱(관리자 트리거)은
임시 인덱스에 다시 빌드한 뒤 원자적으로 교체합니다.
