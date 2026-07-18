---
title: 배포
description: Compose 토폴로지, 이미지, 백업.
order: 10
---

배포 관련 자산은 `deploy/`와 `compose/`에 있으며, 인프라와 애플리케이션의
라이프사이클을 독립적으로 관리할 수 있도록 분리되어 있습니다:

- **`deploy/docker-compose.infra.yml`** — Postgres(pgBackRest가 포함된 이미지),
  PgDog 커넥션 풀러, 세 개의 Redis 인스턴스(세션 / 캐시 / 락), NATS,
  Meilisearch. Postgres는 `127.0.0.1:${POSTGRES_HOST_PORT}`에만 바인딩됩니다.
- **`deploy/docker-compose.app.yml`** — 일회성 마이그레이션 잡, 서버,
  워커, 미디어 프로세서 사이드카. 모두 GHCR에서 가져옵니다.
- **`deploy/docker-compose.logging.yml`** — 선택적인 Loki + Grafana + Alloy 로그
  파이프라인.

환경 선택은 `deploy/<env>.env`와 gitignore된 `.envs/.production/` 트리를
통해 이루어지며, `COMPOSE_PROJECT_NAME`으로 한 호스트에서 여러 환경을
격리할 수 있습니다. Neon 변형(`deploy/neon/`)은 로컬 Postgres를 완전히 제거합니다 —
마이그레이션 서비스는 direct 엔드포인트를 사용하고, 앱은 풀러 URL을 거칩니다.

```bash
cd deploy
just up production          # infra + app
just up-logging production  # + log pipeline
just logs production server
```

## 이미지

`cargo xtask docker-publish --tag <version> [--latest]`는 서버와 워커 이미지를
빌드하여 GHCR에 푸시합니다. `docker.yml` 워크플로도 `v*` 태그에서 동일한 작업을 수행합니다.
`Dockerfile`은 릴리스 빌드이고, `Dockerfile.dev`는 테스트 스택에서 사용하는
빠른 디버그 빌드입니다.

## 백업

pgBackRest는 Postgres 이미지에 통합되어 있으며 `deploy/justfile`로 제어합니다:
`pgbackrest-init`, `-backup [full|diff|incr]`, `-info`, `-restore`(PITR을 지원하는
델타 복원), 그리고 주간 full/일간 diff 호스트 crontab을 설정하는 `-cron-install`이
있습니다. 저장소는 R2의 S3 API를 대상으로 합니다.

## 엣지 전제 조건

이 템플릿은 TLS를 종단하고 `CF-Connecting-IP`(Cloudflare)를 전달하는 엣지 —
또는 공유된 `X-Internal-Secret`과 함께 `X-Real-Client-IP`를 보내는 신뢰할 수 있는
프록시 — 뒤에 배치되는 것을 전제로 합니다. 레이트 리밋은 내장 미들웨어를
사용하거나 게이트웨이에 위임할 수 있습니다(신원 기반 레이트 리밋 키잉을 위한
APISIX compose 파일과 `/v0/auth/check` forward-auth 엔드포인트가 포함되어 있습니다).
