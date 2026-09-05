# AxumKit vNext 구조 검토 및 재설계안

검토일: 2026-09-05. AxumKit `ba89050`의 로컬 소스 기준.
첨부 피드백을 코드와 대조한 설계 제안이다. 아래 목표 구조는 아직 구현되지 않았다.
기존 AxumKit API/DB와의 backward compatibility는 요구하지 않는다.

## 1. 판단

전면적인 구조 개편이 타당하다. 범위는 crate 배치뿐 아니라 트랜잭션, 작업 복구,
인증 상태 저장, SQL 권한, 런타임 수명, 개발 명령, Docker, 테스트와 문서까지다.
설계의 중심은 명시적인 소유권과 장애 복구 계약이다. 제품별 분산 프로토콜은
AxumKit의 기본 구성으로 가져올 이유가 없다.

기존 community 기능은 검증된 완성 예제로 보존한다. 호환성을 없애는 것과 기능을
폐기하는 것은 별개의 결정이다. 새 core는 community를 컴파일하지 않고도 실행·검증되어야 한다.

## 2. 소스로 확인한 현 상태

| 영역 | 확인 결과 | 근거 |
| --- | --- | --- |
| 작업 내구성 | OAuth 가입 commit 이후 인덱싱·이미지 작업을 spawn한다. DB와 publish 사이에 복구할 작업 row가 없다. | `crates/server/src/service/oauth/complete_signup.rs:206`, `crates/server/src/service/user/utils/background_jobs.rs` |
| 이메일 | 모든 이메일이 spawn인 것은 아니다. signup은 Redis에 가입 대기 상태를 만든 다음 NATS publish를 await한다. 이 경로도 두 저장소 사이 원자성이 없다. | `crates/server/src/service/auth/signup.rs:75` |
| OAuth 이미지 | 전체 body를 받은 뒤 크기를 검사한다. 전용 client에 redirect/IP pinning 정책이 없다. 조건부 UPDATE 0건이면 content-hash key를 바로 삭제한다. | `crates/worker/src/jobs/oauth/profile_image.rs:27` |
| 이미지 재시도 | profile update 성공 뒤 index publish가 실패하면 재전달된다. 동일 이미지에서는 기존 key를 삭제하고 후속 index를 생략할 수 있다. 성공 후에만 기록하는 consumer dedup은 이 중간 실패를 막지 못한다. | 같은 파일의 `rows_affected == 0`, `publish_job`; `crates/worker/src/nats/consumer.rs:296` |
| 검색 | 이미 payload는 user ID이며 worker가 DB를 다시 읽는다. 그러나 generation/fence가 없고 `add_documents`/`delete_document`의 최종 task 결과를 확인하지 않는다. | `crates/worker/src/jobs/index/user.rs:67` |
| 재인덱싱 | operation별 독립 temp index가 아닌 고정 `_reindex`를 쓰고 다음 batch를 NATS로 발행한다. swap 결과 조회 실패는 성공 처리한다. | `crates/worker/src/jobs/reindex/users.rs`, `crates/worker/src/jobs/reindex/common.rs:14` |
| 런타임 | main이 startup 오류를 출력하고 정상 반환한다. HTTP graceful shutdown이 없고 SSE task handle을 보관하지 않는다. API가 JetStream stream도 생성한다. | `crates/server/src/main.rs` |
| 시작 의존성 | DB·Redis·NATS 연결은 실제 수행한다. R2와 Meili는 client 생성과 원격 health 검증을 구별해야 한다. Compose는 Meili/media health까지 필수로 연결한다. | `crates/server/src/connection/*`, `docker-compose.dev.yml` |
| SSE | broadcast 오류를 버리고 재연결 replay를 구현하지 않는다. subscriber가 종료돼도 API는 계속 동작한다. | `crates/server/src/service/eventstream/stream_actions.rs:54`, `crates/server/src/eventstream/subscriber.rs` |
| 설정 | 전역 LazyLock, raw String secret, 파생 Debug, 내부 dotenv 로딩이 있다. 이는 노출 가능성이지 실제 secret 유출을 확인한 것은 아니다. | `crates/config/src/server_config.rs` |
| DB 권한 | 예제와 테스트에서 migration/API/worker가 같은 DB 사용자 체계를 쓴다. migration에 runtime privilege 설치가 없다. | `.envs/.example/postgres.env`, `docker-compose.test.yml`, `crates/migration/src/lib.rs` |
| 개발 환경 | 기본 infra에 PgDog·Redis 3개·NATS·Meili가 들어간다. | `justfile:6`, `docker-compose.dev.yml` |
| E2E | HTTP 검증 외에도 entity/worker에 의존한 DB fixture와 NATS consumer 검증이 한 crate에 섞여 있다. | `crates/e2e/Cargo.toml`, `crates/e2e/src/lib.rs` |
| CI | fresh migration 검증은 이미 있다. runtime 권한·upgrade 경로 검증은 해당 workflow에 없다. 로컬 E2E는 4 threads, CI는 제한 없이 실행한다. | `.github/workflows/check.yml`, `.github/workflows/e2e.yml`, `justfile` |

이미지 삭제 경로는 코드상 재현 가능한 실패 순서를 확인했으며, 실제 외부 provider를 이용한
공격 가능성이나 운영 데이터 손상을 재현한 것은 아니다. provider URL을 얻는 경로까지 고려하지 않고
곧바로 외부 공격자에 의한 P0 취약점으로 확정하지 않는다.

## 3. 피드백에서 반드시 보완할 부분

### PostgreSQL queue가 보장하는 범위

- claim은 SELECT만으로 끝내지 않는다. 짧은 transaction의 `SKIP LOCKED` 선택과
  token/lease/attempt 갱신을 원자적으로 수행하고 commit 후 외부 I/O를 시작한다.
- pending → running → succeeded/retry/failed/cancelled 상태와 만료된 running 회수 규칙을 명시한다.
- heartbeat·완료·실패 갱신은 job ID뿐 아니라 현재 claim token과 lease로 조건부 실행한다.
- 일반 delivery는 개별 의무를 보존한다. 검색처럼 최신 상태만 필요한 작업에만 coalescing을 적용한다.
  generation 필드를 모든 job에 붙이는 것만으로 동일한 의미가 생기지 않는다.
- schema에 lease 필드의 일관성, attempts 범위, 완료/실패 timestamp 일관성, unique logical key,
  hot-row 조회용 partial index를 둔다. payload에는 version과 작은 크기 제한을 둔다.
- worker별 concurrency와 kind별 quota를 둬 느린 메일이 다른 작업을 전부 막지 않도록 한다.
- completed/failed retention, retry 감사 기록, poison job 격리, queue age 관측까지 core 계약이다.

DB queue도 외부 SMTP 전송과 PostgreSQL 완료 표시를 하나의 transaction으로 만들지는 못한다.
전송 성공 직후 worker가 죽으면 중복 메일 가능성이 있다. provider idempotency가 없다면
at-least-once와 응답 불명 상태를 명시하며, 중복 불가를 약속하지 않는다.

### 검색은 generation만 추가하면 끝나지 않는다

오래된 worker가 lease 검사 후 멈췄다가 새 worker보다 늦게 외부 요청을 보낼 수 있다.
DB fencing은 외부 Meili 쓰기를 자동으로 fence하지 않는다. resource별 실행 직렬화,
task UID/최종 status 추적, lost-write repair가 필요하다. 검색 응답의 권한 판정은 항상
DB 정본에 재검증하고, generation 불일치는 노출을 차단하거나 정본으로 대체한다.

재인덱싱은 operation별 index, snapshot/watermark 및 concurrent update catch-up,
cutover 상태, task 결과 불명 시 조회·조정 경로를 가진다. swap을 무작정 재시도하면
원래 index로 되돌릴 수 있으므로 durable operation에 확인 가능한 진행 상태를 남긴다.

Meili 쓰기는 접수와 성공이 다르다. [공식 task 문서](https://www.meilisearch.com/docs/capabilities/indexing/tasks_and_batches/monitor_tasks).

### SSE cursor는 commit 순서를 보장해야 한다

T1이 sequence 10을 할당받고 늦게 commit하고, T2의 11을 먼저 보낸 뒤 cursor를 11로
옮기면 `sequence > cursor` 조회는 10을 영구히 놓친다. BIGSERIAL/UUIDv7만으로 해결하지 않는다.

커밋 순서는 deferred trigger와
transaction advisory lock으로 짧은 sequence 할당 구간을 직렬화하여 보장한다. 구현 시
commit contention도 측정한다. 초기 LISTEN을 commit한 뒤 DB catch-up을 수행하고,
재구독·lag 시 재조회하며 주기 polling을 안전망으로 둔다.

전용 LISTEN 연결은 일반 transaction pool과 분리한다. retention 이전 cursor의 reset 계약,
페이지 처리 중 scan cursor, 현재 권한 필터, client dedup, shutdown 시 SSE 종료도 포함한다.
감사 로그와 외부 공개 event payload는 같은 노출 범위를 가진다고 가정하지 않는다.
[PostgreSQL LISTEN 초기화 규칙](https://www.postgresql.org/docs/current/sql-listen.html).

### Redis 제거는 인증 상태 전부의 이전이다

session 외에 pending signup, handle/email 예약, OAuth state/nonce, TOTP challenge·step replay,
비밀번호 재설정·기기 인증·이메일 변경 token, rate limit도 Redis를 사용한다.
PostgreSQL 기본 구성은 이 상태와 원자적 consume·만료·취소를 모두 대체해야 한다.
session TTL touch는 매 요청 UPDATE 대신 threshold를 두어 write amplification을 제한한다.
rate limit은 무제한 cardinality로 DB를 채우지 않게 TTL·행 수·키 구성을 제한한다.

이메일 delivery에는 암호화된 rendering data, key ID, expires/superseded 상태를 두고
job에는 delivery ID만 둔다. 암호화 키 주입·교체, 성공/만료 시 ciphertext 제거, 로그 redaction까지
동시에 설계한다. 인증 bearer의 검증용 저장값은 기존처럼 hash를 유지한다.

### scheduler와 blob GC도 명시적인 프로토콜이 필요하다

session advisory lock을 임의의 pooled connection으로 획득·해제하는 예시는 채택하지 않는다.
짧은 DB 작업은 transaction lock, 정기 작업은 `(schedule_name, scheduled_for)` unique occurrence와
job enqueue/next_run 갱신을 같은 transaction으로 처리한다. timezone·DST·missed-run의
skip/coalesce/catch-up 정책과 보충 실행 상한을 정의한다.

blob 삭제 전 reference 조회만 해도 조회 뒤 새 reference가 생기는 경쟁이 남는다.
asset row의 active/deleting 상태와 참조 생성 transaction의 공통 lock 규칙, grace period,
삭제 재시도·재업로드 규칙을 포함한 GC가 필요하다. 실패한 avatar job의 직접 DELETE는 없앤다.
조회수는 현재 코드가 손실 허용 best-effort임을 명시한다. 이를 중요한 전달 의무와 같은
수준으로 올릴지는 제품 계약으로 선택하며, 모든 telemetry를 durable job으로 만들지 않는다.

## 4. 목표 코드 구성

```text
apps/
  api/                   # core composition root, router, runtime, main
  worker/                # core handlers + scheduler + supervisor
crates/
  app/                   # identity, authorization, notifications
    src/<feature>/       # http, application, persistence, error
  jobs/                  # PostgreSQL queue/lease/scheduler kernel
  auth-core/             # 독립 암호화·토큰 primitive
  http-support/          # request ID, errors, network/client-IP policy
  storage/               # 필요 composition에만 연결하는 object adapter
  migration/             # SQL baseline, manifest, privilege policy
  test-support/          # disposable DB와 provider test doubles
examples/
  community/             # board/comment/group/moderation/search/media 및 API/worker composition
  distributed/           # outbox → JetStream → transactional inbox
xtask/                   # 개발·검증·bootstrap·운영 명령의 실제 구현
```

독립적인 재사용/의존성 경계가 생길 때만 network-safety 같은 module을 crate로 승격한다.
처음부터 feature마다 crate, repository trait, DTO crate를 만들지 않는다.
HTTP DTO·error·persistence는 feature가 소유하고 ORM entity를 외부 wire 계약으로 공개하지 않는다.
`AppState`는 concrete feature service의 composition이며 `FromRef`로 필요한 substate만 추출한다.
단일 workspace에서도 core manifest의 dependency graph에 community/NATS/Meili/AWS가 없어야 한다.

SeaORM을 제거해야만 explicit SQL이 되는 것은 아니다. CRUD는 유지 가능하다.
queue·권한·constraint·trigger 등 PostgreSQL 계약은 review 가능한 `.sql`로 작성하고 runner는 하나로 둔다.
ORM 교체는 이 구조 변경의 필수 조건이 아니다.

## 5. 인프라·운영·개발 계약

| 영역 | 목표 |
| --- | --- |
| Core | PostgreSQL + API + worker. 로컬 메일 확인은 Mailpit 같은 test/dev sink 사용. 외부 계정 없이 기동 가능 |
| Community | 별도 composition에서 Meili·S3 호환 저장소·media processor. Redis cache는 실제 필요한 경우만 |
| Distributed | NATS bootstrap 전용 명령/권한. app은 topology 확인과 publish/consume만 수행 |
| Outbox/inbox | domain+outbox atomic commit, publish ack 뒤 delivered 표시, crash 재발행 시 inbox unique ID와 business update를 같은 transaction에 처리 |
| SQL owner | 로컬 provisioner가 role을 만들고 migrator가 DDL 수행. runtime에는 명시적 DML/함수 실행만 부여. worker role은 실제 권한 차이가 있을 때 분리 |
| Migration | vNext 전용 새 baseline, 이 릴리스 이후 checksum으로 수정 감지. legacy DB를 발견하면 명확히 거절하며 자동 fresh 금지 |
| Runtime | typed Settings::load, secret redaction, 작은 main의 non-zero error exit, critical task 등록·restart/failure 정책, bounded drain |
| Health | livez는 생존, readyz는 DB/schema/종료 상태와 필요한 local task 상태. worker heartbeat·queue age는 별도 관측. SMTP 장애만으로 API 전체 제외 금지 |
| API | 새 `/api/v1` 계약, feature별 오류 → 공통 wire problem, deterministic cursor, 필요한 command에 idempotency scope/body hash/expiry와 concurrent replay 정책 |
| 초기 관리자 | owner CLI의 감사 가능한 1회 bootstrap. 테스트가 DB에 role을 직접 넣어야 하는 현재 상태를 기본 운영 경험으로 남기지 않음 |
| Docker | API/worker/migrator 분리 target, 단일 Dockerfile에서 build profile 공유, lockfile 고정 build, 명시적 OS/Rust/image 버전, non-root와 종료 유예 일치 |
| just/xtask | just는 얇은 명령 목록. setup/dev/migrate/check/e2e/jobs inspect/retry를 xtask에서 실제 구현하고 CI도 같은 명령 사용 |
| E2E lifecycle | unique compose project, 포트 격리, startup 실패부터 logs/teardown 보장, 원래 실패 exit 보존, 제한 시간. dev volume을 대상으로 down -v 하지 않음 |
| 문서 | root README, 환경 예제, deploy README, docs 사이트, OpenAPI, Docker publish 대상·release 안내를 같이 갱신 |

JetStream dedup은 PostgreSQL business transaction을 대체하지 않는다.
[NATS 전달 계약](https://github.com/nats-io/nats.docs/blob/master/nats-concepts/jetstream/README.md).

## 6. 검증 구성과 구현 순서

한 번의 vNext breaking release로 내보내되 내부 구현은 아래 완료 조건을 가진 묶음으로 진행한다.
구 API/신 API 이중 쓰기나 legacy migration bridge는 만들지 않는다.

| 순서 | 변경 묶음 | 완료 조건 |
| --- | --- | --- |
| 1 | workspace/composition, settings/runtime, SQL baseline/roles, Docker/xtask skeleton | core가 외부 서비스 설정 없이 fresh DB에서 기동, startup 실패 non-zero, runtime DDL 거절 |
| 2 | jobs/scheduler kernel과 PostgreSQL integration harness | rollback 시 job 없음, commit 후 crash 회수, concurrent claim, expired token 완료/heartbeat 거절, terminal retry 감사 |
| 3 | identity/session/challenge/email 이전 | 계정 변경·token consume·delivery enqueue 원자성, concurrent single-use, session revoke, enumeration 방지 회귀 |
| 4 | community avatar/search/reindex/notification 이전 | safe fetch·body cap, active blob 보호, provider 실패/응답 유실, stale writer repair, rebuild catch-up/cutover |
| 5 | realtime와 나머지 vertical module 이전 | 역순 commit, reconnect/lag, LISTEN 재접속, retention reset, 권한 변경 중 replay |
| 6 | 구 horizontal crate·기본 NATS/Redis 제거, 문서·CI·배포 예제 정리 | core/community 각각 build+DB test+E2E, OpenAPI drift, Docker stop/drain, 신규 clone quickstart |

전체 재설계보다 먼저 기존 버전에 수정을 내보내야 한다면 avatar 삭제/외부 fetch,
startup exit와 shutdown을 별도 좁은 변경으로 처리할 수 있다.

테스트는 pure unit, PostgreSQL contract, HTTP E2E, provider/worker failure integration으로 나눈다.
기존 TOTP backup-code 동시 소비, ACL fail-closed, auth enumeration, notification 동작 테스트의
의미는 유지한다. E2E crate에는 production worker를 import하지 않고 queue concurrency 테스트를
jobs integration으로 이동한다. fixture 생성용 관리자 권한과 검증 대상 runtime 권한은 분리한다.

vNext 최초 baseline에서는 fresh/runtime-privilege 검증을 실행한다. 후속 vNext release부터는
이전 지원 release schema+fixture → 현재 migration 검증을 추가한다. 구 v0.20 DB upgrade를
지원하지 않기로 한 결정과 모순되는 CI 의무를 만들지 않는다.

## 7. 구현 재사용 원칙

검증된 구현이 있으면 새로 작성하기보다 해당 코드와 동작 계약을 검증하는 테스트를 함께 재사용한다.
AxumKit의 의존성·설정·명명에 필요한 변경만 적용하며, 원래의 transaction·fencing·재시도 의미를 보존한다.
주석은 현재 코드의 목적과 invariant를 설명하고, 문서는 AxumKit만으로 이해할 수 있게 작성한다.
프로젝트 전용 서비스명·도메인·경로·운영 구성·비교 설명을 공개 코드와 문서에 포함하지 않는다.

## 8. 검증 범위

- AxumKit 소스·migration·Compose·CI·just·E2E harness를 대조했다.
- `docker compose -f docker-compose.test.yml config --quiet`: 성공.
- `just --show e2e`: 현재 recipe 파싱 및 확인 성공.
- Docker Desktop Linux engine named pipe가 없어 실제 컨테이너·PostgreSQL·E2E는 실행하지 않았다.
- 컴파일/전체 테스트/운영 검증은 수행하지 않았다. 구조 재설계는 아직 구현하지 않았다.
