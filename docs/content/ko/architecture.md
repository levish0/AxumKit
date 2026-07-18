---
title: 아키텍처
description: 워크스페이스 구성과 각 구성 요소 간의 통신 방식.
order: 3
---

AxumKit은 두 개의 바이너리와 공유 계약(contract) 크레이트들로 이루어진 Cargo 워크스페이스입니다.

## 두 개의 바이너리

- **`server`** — API 서버입니다. routes → services → repositories → entities로 계층화되어 있으며,
  권한, 미들웨어, extractor, 아웃바운드 클라이언트 등 횡단 관심사 모듈을 포함합니다.
- **`worker`** — 백그라운드 잡입니다. NATS JetStream 컨슈머와 크론 스케줄러로 구성됩니다.

두 바이너리는 서로를 절대 링크하지 않습니다. 경계를 넘나드는 모든 것은 전용 계약
크레이트에 위치하므로, 컴파일러가 불일치(drift)를 잡아냅니다:

| 크레이트 | 계약 |
| --- | --- |
| `job_queue` | 잡 페이로드, 스트림/서브젝트/컨슈머 이름, 멱등한 스트림 생성. 두 바이너리 모두 시작 시 `initialize_all_streams`를 호출하므로, 부팅 순서와 무관하게 새 NATS에서도 동작합니다. |
| `notification_repository` | 알림 이벤트와 수신자별 전달(delivery)을 기록하는 방식, 그리고 수신 설정 기반 필터링. |
| `search_index` | Meilisearch 인덱스 uid와 문서 스키마. 워커가 `SearchUser`를 직렬화해 넣고, 서버가 동일한 구조체로 역직렬화해 꺼냅니다. |
| `entity` / `migration` | SeaORM 엔티티와 스키마 자체. |
| `dto` | 요청/응답 타입과 검증기. |
| `errors` | `Errors` enum, 도메인별 핸들러 체인, `protocol.rs`의 와이어 코드. |
| `config` | `ServerConfig` / `WorkerConfig` (환경 변수 기반이며, 누락된 변수를 한 번에 모두 보고합니다). |
| `constants` | 캐시 키와 코드네임 enum (`Permission`, `NotificationAction`, `ModerationAction`). |
| `auth-core` | 프로젝트에 종속되지 않는 암호화 프리미티브 (AEAD, 상수 시간 비교, keyed hash, 토큰). |
| `storage` | 연산별 타임아웃이 설정된 R2/S3 클라이언트. |

## 서버 내부 구조

```
api/            route handlers (+ utoipa annotations, per-domain openapi.rs)
service/        business logic; owns transactions
repository/     queries; find_* → Option, get_* → Result, one function per file
permission/     UserContext, has_perm/require_perm, per-domain Rule objects
middleware/     anonymous user, CORS, require_role gates, stability layer
extractors/     session resolution (cookie or Bearer), Turnstile verification
bridge/         outbound clients: job publisher, media processor, Turnstile
eventstream/    SSE fan-out over core NATS (multi-replica safe)
connection/     one file per external dependency
```

코드를 작성하기 전에 알아둘 만한 컨벤션:

- 핸들러는 데이터베이스를 직접 다루지 않습니다. 서비스가 트랜잭션을 소유하고
  리포지토리를 호출합니다.
- 만료 시각이 있는 행(역할, 밴, 그룹 멤버십)은 **읽는 시점에 필터링**되며,
  만료된 행은 이후 크론 잡이 정리합니다.
- 실패해도 되는(best-effort) 부수 효과(인덱싱, 알림, 캐시 무효화)는 커밋 이후
  `tokio::spawn`으로 큐에 넣습니다 — 큐 장애가 사용자 요청을 실패시켜서는 안 됩니다.
- 코드네임 스타일 값(`board:pin_post`, `user_mentioned`)은 `constants`에 정의된
  Rust enum이며 Postgres에는 TEXT로 저장됩니다. 배리언트를 추가해도 마이그레이션이
  필요 없고, 더 이상 파싱되지 않는 저장 문자열은 안전하게 거부(fail closed)됩니다.

## 에러 처리

서비스와 핸들러는 `Errors`를 반환하며, `IntoResponse` 구현이 각 배리언트를
도메인별 핸들러 체인을 거쳐 `(status, "domain:code", details)`로 매핑합니다.
클라이언트 에러(4xx)는 항상 details를 포함하고, 서버 에러(5xx)는 개발 환경 밖에서는
이를 숨깁니다. `errors/src/protocol.rs`의 문자열 코드는 프런트엔드를 위한 안정적인
와이어 계약입니다.
