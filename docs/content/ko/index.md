---
title: 소개
description: AxumKit이 무엇이고 어떤 기능을 기본 제공하는지 설명합니다.
order: 1
---

AxumKit은 **Axum**, **SeaORM**, **PostgreSQL**, **Redis**, **NATS JetStream**,
**Meilisearch** 위에 구축된 프로덕션 수준의 Rust 웹 API 템플릿입니다. 클론해서
실제 서비스로 키워 나가는 것을 목표로 하며, 진지한 백엔드라면 반드시 필요한
기반 — 인증, 인가, 백그라운드 작업, 이메일, 검색, 테스트, 배포 — 이 이미
갖춰져 있고 서로 연결되어 있습니다.

## 포함된 기능

- **세션 기반 인증** — Redis에 해시된 상태로 저장되는 불투명(opaque) 베어러 토큰,
  절대 수명 상한이 있는 슬라이딩 TTL, 세션 목록 조회/폐기, 암호화된 시크릿과
  일회용 백업 코드를 갖춘 TOTP 2FA, 새 기기 이메일 인증, 그리고 PKCE와 state
  바인딩을 적용한 OAuth2(Google, GitHub, Google One Tap).
- **Django 스타일 RBAC** — 큰 단위의 역할(`Mod`, `Admin`), 세분화된 권한
  코드네임(`board:pin_post`, …), 그리고 멤버에게 권한을 묶어 부여하는
  관리자 관리형 ACL 그룹.
- **데모 기능으로 제공되는 게시판 도메인** — 게시판, 게시글, 답글 깊이 제한이 있는
  댓글, 순서 변경이 가능한 고정 게시글, 게시글 잠금, 모더레이션, 버퍼링된
  조회수 집계, @handle 멘션.
- **인앱 알림** — 댓글 알림과 멘션으로 채워지는 사용자별 수신함, 액션별
  수신 거부(opt-out) 설정 지원.
- **백그라운드 워커** — 이메일(MJML 템플릿), Meilisearch 인덱싱, OAuth 아바타
  처리를 담당하는 NATS JetStream 컨슈머와 정리 작업 및 조회수 플러시를 위한
  크론 잡. 재시도, 메시지 단위 중복 제거, 데드레터 큐, 그레이스풀 셧다운이
  컨슈머 엔진에 내장되어 있습니다.
- **운영 도구** — 블랙박스 e2e 스위트를 갖춘 일회용 Docker 테스트 스택,
  CI의 OpenAPI 드리프트 게이트, dev/test/production용 compose 구성, 그리고
  콘텐츠 주소 기반 이미지 처리를 지원하는 Cloudflare R2 스토리지.

## 다음 단계

먼저 [시작하기](/docs/getting-started)를 읽고, 코드를 수정하기 전에
[아키텍처](/docs/architecture)를 훑어보며 워크스페이스 구조를 파악하십시오.
