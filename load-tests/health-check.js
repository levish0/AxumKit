import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const timeoutRate = new Rate('timeouts');
const overloadRate = new Rate('overloads');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8000';

export const options = {
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)'],

  scenarios: {
    // Scenario 1: Normal load
    normal_load: {
      executor: 'constant-vus',
      vus: 100,
      duration: '30s',
      startTime: '0s',
      tags: { scenario: 'normal' },
    },

    // Scenario 2: At concurrency limit (500)
    at_limit: {
      executor: 'constant-vus',
      vus: 500,
      duration: '30s',
      startTime: '35s',
      tags: { scenario: 'at_limit' },
    },

    // Scenario 3: Over limit, using buffer (1000)
    buffer_queue: {
      executor: 'constant-vus',
      vus: 1000,
      duration: '30s',
      startTime: '70s',
      tags: { scenario: 'buffer' },
    },

    // Scenario 4: Spike test (up to 10,000 VUs)
    spike: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '10s', target: 1000 },
        { duration: '10s', target: 5000 },
        { duration: '10s', target: 10000 },
        { duration: '30s', target: 10000 },
        { duration: '10s', target: 1000 },
        { duration: '5s', target: 0 },
      ],
      startTime: '105s',
      tags: { scenario: 'spike' },
    },
  },

  thresholds: {
    'http_req_duration{scenario:normal}': ['p(95)<3000'],
    'errors{scenario:normal}': ['rate<0.01'],
  },
};

export default function () {
  const url = `${BASE_URL}/health-check`;

  const res = http.get(url, {
    timeout: '35s',
    tags: { name: 'health_check' },
  });

  const success = check(res, {
    'status is 204': (r) => r.status === 204,
    'status is not 5xx': (r) => r.status < 500,
  });

  if (!success) {
    errorRate.add(1);

    if (res.status === 408) {
      timeoutRate.add(1);
    } else if (res.status === 503) {
      overloadRate.add(1);
    }
  } else {
    errorRate.add(0);
  }

  sleep(0.05);
}

export function handleSummary(data) {
  const duration = data.metrics.http_req_duration?.values || {};
  const errors = data.metrics.errors?.values || {};
  const reqs = data.metrics.http_reqs?.values || {};
  const timeouts = data.metrics.timeouts?.values || {};
  const overloads = data.metrics.overloads?.values || {};

  const totalRequests = reqs.count || 0;
  const timeoutCount = timeouts.passes || 0;
  const overloadCount = overloads.passes || 0;

  console.log('\n========== HEALTH CHECK LOAD TEST SUMMARY ==========');
  console.log(`Total Requests: ${totalRequests}`);
  console.log(`Success Rate: ${((1 - (errors.rate || 0)) * 100).toFixed(2)}%`);
  console.log(`Timeouts (408): ${timeoutCount} (${totalRequests > 0 ? ((timeoutCount / totalRequests) * 100).toFixed(2) : 0}%)`);
  console.log(`Overloads (503): ${overloadCount} (${totalRequests > 0 ? ((overloadCount / totalRequests) * 100).toFixed(2) : 0}%)`);
  console.log(`Avg Response Time: ${(duration.avg || 0).toFixed(2)}ms`);
  console.log(`P95 Response Time: ${(duration['p(95)'] || 0).toFixed(2)}ms`);
  console.log(`P99 Response Time: ${(duration['p(99)'] || 0).toFixed(2)}ms`);
  console.log(`Max Response Time: ${(duration.max || 0).toFixed(2)}ms`);
  console.log('=====================================================\n');

  return {};
}
