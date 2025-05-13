<div align="right">
  <strong>한국어</strong> | <a href="./README.md">English</a>
</div>

# Axum + SeaORM + PostgreSQL + JWT + REST API + OpenAPI 템플릿

[![Rust](https://img.shields.io/badge/rust-stable-blue.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Rust, Axum, SeaORM을 사용한 고성능 웹 API를 구축하기 위한 프로덕션 준비가 된 템플릿입니다.

## 주요 기능

- **고성능 웹 서버**: 빠르고 모듈화된 웹 프레임워크인 Axum 기반
- **타입 안전 ORM**: 컴파일 타임 보장을 통한 SeaORM 데이터베이스 연동
- **PostgreSQL 지원**: 강력한 관계형 데이터베이스 통합
- **JWT 인증**: 즉시 사용 가능한 보안 인증 시스템
- **RESTful API**: 모범 사례를 따르는 잘 구조화된 엔드포인트
- **OpenAPI 문서화**: Swagger UI를 통한 자동 생성 API 문서
- **모듈식 아키텍처**: 유지보수성을 위한 관심사 분리
- **환경 기반 설정**: 유연한 구성 관리
- **데이터베이스 커넥션 풀링**: 효율적인 연결 관리
- **비동기/대기**: 최대 성능을 위한 완전한 비동기 처리
- **에러 처리**: 일관된 에러 응답 및 로깅

## 시작하기

### 선행 조건

- Rust (최신 안정 버전 권장)
- PostgreSQL (12 이상)
- Cargo (Rust의 패키지 관리자)
- Docker 및 Docker Compose (선택사항, 컨테이너 기반 개발용)

### 설치

1. 저장소 클론:
   ```bash
   git clone https://github.com/yourusername/axum-seaorm-postgresql-template.git
   cd axum-seaorm-postgresql-template
   ```

2. 환경 변수 설정:
   ```bash
   cp .env.example .env
   ```
   `.env` 파일을 열어 데이터베이스 자격 증명 및 기타 설정을 업데이트하세요.

3. 의존성 설치 및 빌드:
   ```bash
   cargo build
   ```

4. 데이터베이스 마이그레이션 실행 (SeaORM CLI 필요):
   ```bash
   cargo install sea-orm-cli
   sea-orm-cli migrate up
   ```

5. 서버 시작:
   ```bash
   cargo run
   ```

6. API 문서에 접속:
   ```
   http://localhost:8000/docs
   ```

## Docker 설정

이 프로젝트는 쉬운 개발과 배포를 위한 Docker 구성을 포함하고 있습니다.

### 선행 조건

- Docker Engine 20.10.0 이상
- Docker Compose 2.0.0 이상

### Docker로 빌드 및 실행하기

#### 빌드만 하기

Docker 컨테이너를 실행하지 않고 이미지만 빌드하려면:

```bash
docker-compose build
```

이 명령은 `docker-compose.yml`에 정의된 모든 서비스를 빌드합니다.

#### 빌드 및 실행

1. 애플리케이션 빌드 및 시작:
   ```bash
   docker-compose up --build
   ```

2. 백그라운드에서 실행:
   ```bash
   docker-compose up -d --build
   ```

3. 로그 확인:
   ```bash
   docker-compose logs -f
   ```

4. 애플리케이션 중지:
   ```bash
   docker-compose down
   ```

5. 모든 컨테이너, 네트워크, 볼륨 제거:
   ```bash
   docker-compose down -v
   ```

### 서비스

- **app**: 메인 애플리케이션 서버 (포트 8000)
- **db**: PostgreSQL 데이터베이스 (포트 5432)
- **migrate**: 시작 시 데이터베이스 마이그레이션 실행

### 환경 변수

프로젝트 루트에 `.env` 파일을 생성하여 애플리케이션을 구성할 수 있습니다. 사용 가능한 옵션은 `.env.example` 파일을 참조하세요.

### 개발 워크플로우

- 소스 코드 변경 시 애플리케이션이 자동으로 재시작됩니다.
- 시작 시 데이터베이스 마이그레이션이 자동으로 실행됩니다.
- 데이터베이스 데이터는 Docker 볼륨에 유지됩니다.

## 프로젝트 구조

```
src/
├── api/               # API 라우트 및 핸들러
├── config/            # 애플리케이션 설정
├── database/          # 데이터베이스 연결 및 설정
├── dto/               # 데이터 전송 객체
├── entity/            # SeaORM 엔티티
├── middleware/        # Axum 미들웨어
├── service/           # 비즈니스 로직
├── main.rs            # 애플리케이션 진입점
└── state.rs           # 애플리케이션 상태
```

## API 문서화

이 프로젝트는 `utoipa`를 사용하여 OpenAPI 문서를 자동 생성하고 Swagger UI를 통해 제공합니다.

- **Swagger UI**: `http://localhost:8000/docs`
- **OpenAPI JSON**: `http://localhost:8000/api-doc/openapi.json`

### 예제 API 엔드포인트

```rust
#[utoipa::path(
    get,
    path = "/v0/user/{id}",
    params(
        ("id" = i32, Path, description = "사용자 ID")
    ),
    responses(
        (status = 200, description = "사용자 정보 조회 성공", body = UserInfoResponse),
        (status = 404, description = "사용자를 찾을 수 없음"),
        (status = 500, description = "서버 내부 오류")
    ),
    tag = "User"
)]
pub async fn get_user(
    state: State<AppState>,
    Path(id): Path<String>,
) -> Result<UserInfoResponse, Errors> {
    // 핸들러 구현
}
```

## 아키텍처

이 프로젝트는 다음과 같은 계층 구조를 따릅니다:

1. **API 계층**: HTTP 요청/응답 처리
2. **서비스 계층**: 비즈니스 로직 포함
3. **저장소 계층**: 데이터베이스 작업 관리
4. **도메인 계층**: 엔티티와 DTO 정의

## 환경 변수

`.env` 파일에서 다음을 구성하세요:

```env
DATABASE_URL=postgres://사용자이름:비밀번호@localhost:5432/데이터베이스이름
JWT_SECRET=JWT_비밀키
PORT=8000
RUST_LOG=info
```

## 라이선스

이 프로젝트는 MIT 라이선스 하에 있습니다. 자세한 내용은 [LICENSE](./LICENSE) 파일을 참조하세요.

---

<div align="center">
  <sub>Created by <a href="https://github.com/shiueo">Levi Lim</a> | <a href="https://github.com/shiueo/axum-seaorm-postgresql-jwt-rest-openapi-template">GitHub</a></sub>
</div>
