---
title: 시작하기
description: 클론부터 API 실행까지 몇 분 안에 끝냅니다.
order: 2
---

## 사전 요구 사항

- Rust (stable, edition 2024)
- Docker와 Compose
- [`just`](https://github.com/casey/just)

## 첫 실행

```bash
git clone https://github.com/levish0/AxumKit
cd AxumKit
cp .env.example .env          # native dev env (server/worker read this)
cp -r .envs/.example .envs/.local   # compose env tree, fill real values

just dev                      # start infra containers + apply migrations
cargo run -p server           # API on http://localhost:8000
cargo run -p worker           # background worker (separate terminal)
```

`just dev`는 인프라 컨테이너만 띄웁니다 — Postgres(PgDog 풀러 뒤에 위치),
Redis 인스턴스 3개(세션 / 캐시 / 락), JetStream이 활성화된 NATS, Meilisearch —
따라서 두 바이너리는 네이티브로 실행되어 빠르게 반복 개발할 수 있습니다.

디버그 빌드에서는 Swagger UI가 `/docs`에서, OpenAPI 스펙이
`/swagger.json`에서 제공됩니다.

## 자주 쓰는 명령어

```bash
just check          # everything CI checks: fmt, clippy -D warnings, tests, OpenAPI drift
just test [filter]  # workspace tests, excluding e2e
just e2e            # full docker stack + black-box e2e suite (always tears down)
just openapi        # regenerate swagger.json — run after any route change
just migrate-fresh  # drop and reapply all migrations
```

## 첫 관리자 계정

애플리케이션에는 의도적으로 첫 관리자 부트스트랩 경로가 없습니다 — 역할은
오직 기존 관리자만 부여할 수 있습니다. 로컬 개발 시에는 역할을 직접 삽입하십시오:

```sql
INSERT INTO user_roles (user_id, role) VALUES ('<your-user-uuid>', 'admin');
```

서버는 요청마다 역할을 읽으므로 부여 즉시 적용됩니다.
