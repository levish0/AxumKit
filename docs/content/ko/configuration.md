---
title: 설정
description: 환경 변수와 env 파일 구성.
order: 4
---

설정은 환경 변수 기반으로 동작합니다. `ServerConfig`와 `WorkerConfig`는 (`LazyLock`을
통해) 한 번만 로드되며 즉시 검증됩니다 — 누락되었거나 파싱할 수 없는 변수는 하나씩
따로 보고되는 대신 모두 수집되어 단일 패닉 메시지로 보고됩니다.

## Env 파일 구성

- **`.env`** (저장소 루트, gitignore 대상; 템플릿: `.env.example`) — 바이너리를 네이티브로
  실행할 때 사용됩니다 (`cargo run -p server`).
- **`.envs/`** — Docker Compose용으로 관심사별로 그룹화된 env 트리:
  - `.example/` — 커밋되는 템플릿 (`postgres.env`, `server.env`, `worker.env`, `r2.env`, …)
  - `.local/` — gitignore 대상인 개발용 값
  - `.test/` — 일회용 테스트 스택을 위한 커밋되는 값 (실제 시크릿 없음)
  - `.production/` — gitignore 대상인 프로덕션 값

## 데이터베이스

애플리케이션은 단일 **`DATABASE_URL`** 로 표준화되어 있습니다 — 서버, 워커,
마이그레이션 바이너리가 모두 동일하게 읽는 전체 연결 URL입니다:

```
DATABASE_URL=postgres://axumkit:secret@localhost:6432/axumkit
```

host/port/user 조각을 조합하는 방식 대신 의도적으로 URL을 사용하여 배포 환경이
쿼리 스트링을 제어할 수 있게 했습니다 — Neon 같은 관리형 서비스에는
`?sslmode=require` 또는 `channel_binding=require`를, 로컬 Postgres에는 아무것도
붙이지 않으면 됩니다. 번들된 커넥션 풀러를 경유하려면 `pgdog:6432`를 가리키면
됩니다. 로그에는 URL이 그대로 출력되지 않습니다; `redact_database_url`이 먼저
자격 증명을 제거합니다.

`.envs/*/postgres.env`의 `POSTGRES_DB` / `POSTGRES_USER` / `POSTGRES_PASSWORD` 항목은
Postgres **컨테이너**의 initdb와 compose 헬스체크가 사용하는 값이며 — 애플리케이션이
사용하는 값이 아닙니다.

## 변수 레퍼런스 (개요)

| 그룹 | 변수 |
| --- | --- |
| 서버 바인드 | `HOST`, `PORT`, `ENVIRONMENT` (`dev`는 Swagger와 완화된 쿠키 설정을 활성화) |
| 데이터베이스 | `DATABASE_URL`, `POSTGRES_MAX_CONNECTION`, `POSTGRES_MIN_CONNECTION` |
| Redis | `REDIS_SESSION_HOST/PORT` (noeviction + AOF), `REDIS_CACHE_HOST/PORT` (LRU); 워커는 `REDIS_LOCK_HOST/PORT` (noeviction) 추가 |
| 인증 시크릿 | `TOTP_SECRET` (백업 코드 해싱), `TOTP_ENCRYPTION_KEY` (AES-GCM 키 파생) |
| 세션 튜닝 | `AUTH_SESSION_MAX_LIFETIME_HOURS`, `AUTH_SESSION_SLIDING_TTL_HOURS`, `AUTH_SESSION_REFRESH_THRESHOLD` |
| 토큰 만료 | 이메일 인증, 비밀번호 재설정, 이메일 변경, 계정 삭제, 기기 인증 (각각 분 단위) |
| OAuth | `GOOGLE_CLIENT_ID/SECRET/REDIRECT_URI`, `GITHUB_CLIENT_ID/SECRET/REDIRECT_URI` |
| 인프라 | `NATS_URL`, `MEILISEARCH_HOST` (+ 선택적 `MEILISEARCH_API_KEY`), `MEDIA_PROCESSOR_URL` |
| 스토리지 | `R2_ENDPOINT`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_ASSETS_BUCKET_NAME`, `R2_ASSETS_PUBLIC_DOMAIN` |
| 엣지 | `CORS_ALLOWED_ORIGINS` (**프로덕션에서 미설정 시 패닉**), `CORS_ALLOWED_HEADERS`, `COOKIE_DOMAIN`, `TURNSTILE_SECRET_KEY`, `INTERNAL_PROXY_SECRET` |
| 워커 이메일 | `SMTP_HOST/PORT/USER/PASSWORD/TLS`, `EMAILS_FROM_*`, `FRONTEND_HOST` + 플로우별 링크 경로 |

전체 주석 포함 목록은 `.env.example`과 `.envs/.example/`을 참고하십시오.

## 보안 관련 기본값

- 프로덕션 쿠키는 `__Host-` 접두사(`COOKIE_DOMAIN` 설정 시에는 `__Secure-`)와
  `HttpOnly`, `Secure`, `SameSite=Lax`를 사용합니다.
- 백엔드는 `CF-Connecting-IP`만 신뢰하며, 상수 시간 비교로 검증된
  `X-Internal-Secret`이 함께 오는 경우에 한해 `X-Real-Client-IP`를 신뢰합니다 —
  `X-Forwarded-For`는 절대 신뢰하지 않습니다.
- CORS는 와일드카드 대신 요청 헤더를 미러링하므로 자격 증명이 포함된 요청도
  유효하게 유지됩니다; 오리진 미설정은 열린 기본값이 아니라 프로덕션에서 시작 시
  오류로 처리됩니다.
