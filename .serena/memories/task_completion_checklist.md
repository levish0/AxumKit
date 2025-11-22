# Task Completion Checklist

When a coding task is completed, perform the following steps to ensure code quality and proper integration:

## 1. Code Formatting
```bash
cargo fmt
```
- Ensures consistent code style across the project
- Uses default rustfmt configuration
- Must pass before committing

## 2. Static Analysis
```bash
cargo clippy
```
- Checks for common mistakes and anti-patterns
- Follow Clippy suggestions to improve code quality
- Treat warnings seriously (CI may fail on warnings)

## 3. Compilation Check
```bash
cargo check
```
- Fast compilation check without producing executables
- Verifies that code compiles successfully
- Catches type errors and other compilation issues

## 4. Run Tests
```bash
cargo test
```
- Run all unit and integration tests
- Ensure no existing tests are broken
- Add tests for new functionality

## 5. Build Verification
```bash
cargo build
```
- Full debug build to ensure everything compiles
- For production readiness, also run: `cargo build --release`

## 6. Database Migrations (if schema changes were made)
```bash
cd migration
cargo run -- status                  # Check current status
cargo run                            # Apply new migrations
cd ..
```
- Apply any new migrations
- Test migration rollback if critical: `cargo run -- down`
- Regenerate entities if needed: `sea-orm-cli generate entity ...`

## 7. OpenAPI Documentation (if API changes were made)
- Verify `#[utoipa::path(...)]` annotations are correct
- Check that all DTOs have `#[derive(ToSchema)]`
- Register new schemas in `src/api/v0/routes/openapi.rs`
- Test Swagger UI: http://localhost:8000/docs (in debug mode)

## 8. Manual Testing
```bash
cargo run
```
- Start the development server
- Test new endpoints manually via:
  - Swagger UI: http://localhost:8000/docs
  - curl/Postman/HTTP client
- Verify error handling works correctly
- Test edge cases

## 9. Environment Configuration (if new env vars were added)
- Update `.env.example` with new variables
- Document the new variables in CLAUDE.md or README.md
- Ensure default values or validation for missing vars

## 10. Code Review Checklist

### Error Handling
- [ ] All errors properly handled (no unwrap() in production code)
- [ ] Appropriate error types used from `Errors` enum
- [ ] Error messages are helpful but don't leak sensitive info in production

### Security
- [ ] No hardcoded secrets or credentials
- [ ] Input validation using `validator` crate
- [ ] SQL injection prevention (SeaORM handles this)
- [ ] Authentication/authorization checks in place

### Performance
- [ ] Database queries are efficient (use proper indexes)
- [ ] No N+1 query problems
- [ ] Connection pooling used correctly
- [ ] Async/await used properly

### Code Quality
- [ ] Functions are focused and do one thing
- [ ] Code is readable and self-documenting
- [ ] Proper separation of concerns (API → Service → Repository → Entity)
- [ ] DTOs used correctly (request/response/internal separation)

### Documentation
- [ ] Public APIs have doc comments
- [ ] Complex logic is explained with comments
- [ ] OpenAPI documentation is complete
- [ ] CLAUDE.md updated if patterns changed

## 11. Git Workflow
```bash
git status                           # Check changed files
git add .                            # Stage changes
git commit -m "feat: description"   # Commit with descriptive message
```

Commit message format:
- `feat:` - New feature
- `fix:` - Bug fix
- `refactor:` - Code refactoring
- `docs:` - Documentation changes
- `test:` - Adding/updating tests
- `chore:` - Maintenance tasks

## 12. CI/CD Verification
- Push to branch and verify GitHub Actions pass
- Release workflow runs `cargo build --release`
- Ensure build succeeds before merging to main

## Quick Pre-Commit Command Chain (Windows)
```bash
cargo fmt && cargo clippy && cargo test && cargo build
```
- If all pass, code is ready to commit
- Fix any issues that arise before committing

## Optional: Performance Testing
If the task involves performance-critical code:
```bash
cd load_test
# Run load tests as defined in load_test directory
```

## Notes for Specific Task Types

### Adding New Endpoint
1. Handler in `src/api/v0/routes/{domain}/{handler}.rs`
2. Add `#[utoipa::path(...)]` annotation
3. Register route in `src/api/v0/routes/{domain}/routes.rs`
4. Add schemas to `src/api/v0/routes/{domain}/openapi.rs`
5. Create DTOs in `src/dto/{domain}/`
6. Implement service in `src/service/{domain}/`
7. Add repository functions in `src/repository/{domain}/`

### Database Schema Changes
1. Generate migration: `cd migration && cargo run -- generate <NAME>`
2. Edit migration file with up/down SQL
3. Apply migration: `cargo run`
4. Regenerate entities: `sea-orm-cli generate entity ...`
5. Update repository layer
6. Update service layer
7. Run tests

### Adding OAuth Provider
1. Add credentials to `.env` and `.env.example`
2. Create provider handler in `src/service/oauth/`
3. Add routes in `src/api/v0/routes/oauth/`
4. Update `OAuthProvider` enum if needed
5. Test OAuth flow manually

Remember: **Quality over speed** - take time to ensure each step passes before proceeding.
