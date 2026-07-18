import { defineSiteConfig } from '@levish0/lily-pad';

export const site = defineSiteConfig({
	title: 'AxumKit',
	description: 'A production-ready Rust web API template built on Axum.',
	github: 'https://github.com/levish0/AxumKit',
	nav: [
		{ label: { en: 'Home', ko: '홈' }, href: '/' },
		{ label: { en: 'Docs', ko: '문서' }, href: '/docs' }
	],
	rootSection: { en: 'Guide', ko: '가이드' },
	home: {
		features: [
			{
				icon: 'heroicons:shield-check-solid',
				title: { en: 'Hardened auth', ko: '강화된 인증' },
				description: {
					en: 'Redis sessions, TOTP 2FA, new-device verification, OAuth2 with PKCE, and an auth audit log — OWASP ASVS-aligned out of the box.',
					ko: 'Redis 세션, TOTP 2FA, 새 기기 인증, PKCE 기반 OAuth2, 인증 감사 로그까지 — OWASP ASVS 기준을 기본으로 충족합니다.'
				},
				href: '/docs/authentication'
			},
			{
				icon: 'heroicons:key-solid',
				title: { en: 'Django-style RBAC', ko: 'Django 스타일 RBAC' },
				description: {
					en: 'Roles, ACL groups, and grantable permission codenames with an admin API — has_perm semantics for community apps.',
					ko: '역할, ACL 그룹, 부여 가능한 권한 codename과 관리 API — 커뮤니티 서비스를 위한 has_perm 시맨틱.'
				},
				href: '/docs/authorization'
			},
			{
				icon: 'heroicons:queue-list-solid',
				title: { en: 'Background worker', ko: '백그라운드 워커' },
				description: {
					en: 'NATS JetStream job queue with retries, dedup, DLQ, and cron — email, search indexing, and cleanups run outside the request path.',
					ko: '재시도·중복 제거·DLQ·크론을 갖춘 NATS JetStream 잡 큐 — 이메일, 검색 인덱싱, 정리 작업이 요청 경로 밖에서 돌아갑니다.'
				},
				href: '/docs/background-jobs'
			},
			{
				icon: 'heroicons:beaker-solid',
				title: { en: 'Real e2e testing', ko: '진짜 e2e 테스트' },
				description: {
					en: 'A disposable Docker stack (tmpfs Postgres, Mailpit, S3 stand-in) drives black-box HTTP suites with pinned security regressions.',
					ko: '일회용 Docker 스택(tmpfs Postgres, Mailpit, S3 대체재)으로 블랙박스 HTTP 스위트와 고정된 보안 회귀 테스트를 돌립니다.'
				},
				href: '/docs/testing'
			}
		]
	}
});
