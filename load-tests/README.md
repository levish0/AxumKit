# Load Tests

Load testing with k6.

## Installation

```bash
# Windows (Chocolatey)
choco install k6

# Windows (winget)
winget install k6

# macOS
brew install k6

# Linux
sudo apt install k6
```

## Running Tests

### Health Check Test

```bash
# Full scenario
k6 run load-tests/health-check.js

# With environment variables
k6 run -e BASE_URL=http://localhost:8000 load-tests/health-check.js

# Quick test (50 VUs, 10 seconds)
k6 run --vus 50 --duration 10s load-tests/health-check.js
```

### Environment Variables

| Variable   | Default                 | Description    |
|------------|-------------------------|----------------|
| `BASE_URL` | `http://localhost:8000` | Server address |

## Test Scenarios

| Scenario   | VUs         | Duration | Purpose                 |
|------------|-------------|----------|-------------------------|
| `normal`   | 100         | 30s      | Normal load             |
| `at_limit` | 500         | 30s      | Concurrency limit (500) |
| `buffer`   | 1000        | 30s      | Buffer queue (1024)     |
| `spike`    | 0->10000->0 | 75s      | Spike test              |

## Stability Layer Configuration

Server defaults (configurable via environment variables):

```
STABILITY_CONCURRENCY_LIMIT=500   # Concurrent request limit
STABILITY_BUFFER_SIZE=1024        # Queue size
STABILITY_TIMEOUT_SECS=30         # Request timeout (seconds)
```

## Expected Results

- **normal (100 VUs)**: Error rate < 1%, p95 < 3s
- **at_limit (500 VUs)**: Response time increases, minimal errors
- **buffer (1000 VUs)**: Some requests queued, response time increases
- **spike (10000 VUs)**: Many 503/408 errors, graceful degradation
