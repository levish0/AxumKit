# Suggested Commands

## Windows System Commands

Since this project is developed on Windows, use these commands:

### File Operations
- `dir` - List files in current directory
- `cd <path>` - Change directory
- `type <file>` - Display file contents
- `copy <src> <dest>` - Copy files
- `del <file>` - Delete files
- `mkdir <name>` - Create directory
- `rmdir <name>` - Remove directory

### Search
- `findstr <pattern> <files>` - Search for text in files (Windows equivalent of grep)
- Git Bash (if available) can use Unix commands: `grep`, `find`, etc.

### Version Control
- `git status` - Check repository status
- `git add .` - Stage all changes
- `git commit -m "message"` - Commit changes
- `git push` - Push to remote
- `git pull` - Pull from remote

## Development Commands

### Building & Running
```bash
cargo build                          # Build the project in debug mode
cargo build --release                # Build optimized release version
cargo run                            # Run the development server (localhost:8000)
cargo clean                          # Clean build artifacts
```

### Testing
```bash
cargo test                           # Run all tests
cargo test -- --nocapture            # Run tests with output visible
cargo test <test_name>               # Run specific test
```

### Code Quality
```bash
cargo fmt                            # Format code with rustfmt
cargo fmt -- --check                 # Check formatting without modifying files
cargo clippy                         # Run Clippy linter
cargo clippy -- -D warnings          # Run Clippy and treat warnings as errors
cargo check                          # Fast check without producing executables
```

### Database Migrations

Navigate to migration directory first:
```bash
cd migration
```

Then run migration commands:
```bash
cargo run                            # Apply all pending migrations
cargo run -- up                      # Same as above
cargo run -- down                    # Rollback last migration
cargo run -- fresh                   # Drop all tables and reapply migrations
cargo run -- refresh                 # Rollback all, then reapply all migrations
cargo run -- status                  # Check migration status
cargo run -- generate <NAME>         # Generate new migration file
```

After migrations, return to project root:
```bash
cd ..
```

### SeaORM Entity Generation
```bash
sea-orm-cli generate entity \
  -u postgres://user:password@localhost:5432/database \
  -o src/entity
```

### Dependency Management
```bash
cargo update                         # Update dependencies
cargo tree                           # Show dependency tree
cargo outdated                       # Check for outdated dependencies (requires cargo-outdated)
```

## Docker Commands (if using Docker)

```bash
docker-compose build                 # Build Docker images
docker-compose up                    # Start all services
docker-compose up -d                 # Start in detached mode
docker-compose up --build            # Rebuild and start
docker-compose down                  # Stop all services
docker-compose down -v               # Stop and remove volumes
docker-compose logs -f               # View logs
docker-compose ps                    # List running containers
```

## API Documentation Access

When server is running:
- Swagger UI: http://localhost:8000/docs (debug builds only)
- OpenAPI JSON: http://localhost:8000/swagger.json (debug builds only)

## Environment Setup

```bash
copy .env.example .env               # Create .env file from example (Windows)
# Then edit .env with your configuration
```

## Common Development Workflow

1. Make code changes
2. Format code: `cargo fmt`
3. Check for errors: `cargo check`
4. Run linter: `cargo clippy`
5. Run tests: `cargo test`
6. Run server: `cargo run`
7. Test API at http://localhost:8000

## CI/CD

The project uses GitHub Actions for CI/CD:
- **Release workflow**: Runs on push/PR to main branch
- Runs: `cargo build --release`
- Uses caching for Cargo dependencies
