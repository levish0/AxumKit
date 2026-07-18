---
title: 테스트
description: 단위 테스트, e2e 하니스, CI 게이트.
order: 9
---

## 단위 테스트

인라인 `#[cfg(test)]` 모듈로 작성하며, 회귀가 절대 발생해서는 안 되는 부분에 가장 집중되어 있습니다: 권한 엔진(`has_perm` 해석, 보드 정책 규칙), `auth-core`의 암호화 프리미티브, DTO 검증기, 알림 타깃 인코딩. `just test`로 실행합니다(워크스페이스 전체, e2e 제외).

## e2e: 설계상 블랙박스

`crates/e2e`는 서버 코드를 절대 임포트하지 않습니다. 테스트는 실제 클라이언트와 똑같이 HTTP를 통해 완전히 구동 중인 스택을 대상으로 실행되며, `docker-compose.test.yml`을 사용합니다:

- **tmpfs Postgres** — 호스트 포트 55432 사용(빠르고, 매번 초기화되며, 5432에서 실행 중인 네이티브 Postgres에 가려지지 않음)
- **SeaweedFS** — R2를 대체하는 S3 호환 스토리지
- **Mailpit** — SMTP를 캡처(테스트는 Mailpit의 REST API로 이메일 전송된 토큰을 읽음)
- **Turnstile 스텁** — 모든 요청에 성공을 응답
- `Dockerfile.dev`로 빌드된 실제 서버, 워커, 마이그레이션 컨테이너

`just e2e`로 실행합니다 — 헬스체크를 거쳐 스택을 띄우고, 병렬 실행 수를 제한한 상태로 스위트를 실행하며, 항상 스택을 정리하고, 스위트의 종료 코드를 보존합니다.

### 하니스

`TestClient`는 쿠키 저장소를 가진 HTTP 클라이언트로, 각 인스턴스가 독립적인 브라우저 역할을 합니다. `with_ip()`는 서로 다른 클라이언트 IP를 시뮬레이션합니다. `signup_and_login()`은 고유한 사용자를 등록하고 Mailpit 폴링을 통해 이메일 인증까지 완료합니다. 애플리케이션에 최초 관리자 생성 경로가 없기 때문에, 의도적으로 데이터베이스에 직접 접근하는 헬퍼 두 개가 존재합니다: `grant_role(handle, Role)`과 `backdate_user(handle, days)`.

### 스위트

`auth`, `totp`, `account`, `moderation`, `rbac`, `board`, `notification`, `search`, `user_public`, `smoke`. 보안 회귀 테스트는 `sec_NNN` 이름으로 고정되어 있어(예: 백업 코드 단일 사용 동시성, 알림 IDOR 탐지) 쉽게 식별되며 조용히 삭제되는 일이 없습니다.

## CI

| 워크플로 | 게이트 |
| --- | --- |
| `check.yml` | `fmt --check`, `clippy --all-targets -D warnings`, **OpenAPI 드리프트** (`cargo xtask openapi` + `git diff --exit-code swagger.json`), 그리고 새로 생성한 Postgres에 모든 마이그레이션을 적용하는 잡 |
| `test.yml` | 워크스페이스 단위/통합 테스트(check와 병렬 실행) |
| `e2e.yml` | 전체 docker 스택 e2e 스위트, 실패 시 컨테이너 로그 출력 |
| `build.yml` / `docker.yml` | 빌드 검사 및 버전 태그 시 GHCR 이미지 배포 |

OpenAPI 드리프트 게이트 덕분에 `swagger.json`은 항상 코드와 동기화됩니다 — 라우트를 변경한 후에는 `just openapi`를 실행하십시오.
