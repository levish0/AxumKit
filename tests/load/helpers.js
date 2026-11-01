import http from 'k6/http';
import { check, fail } from 'k6';
import { Counter, Trend } from 'k6/metrics';

export const diagnosticStatuses = ['0', '400', '401', '403', '404', '408', '409', '412', '429', '500', '502', '503', '504', 'other'];
const unexpectedResponses = new Counter('unexpected_http_responses');
const clientErrorCodes = new Trend('unexpected_client_error_codes');
const clientErrorIterations = new Trend('unexpected_client_error_iterations');
const clientEof = new Counter('unexpected_client_eof');

export function requireCheck(condition, name) {
  if (!check(condition, { [name]: value => value })) {
    fail(`Mixed workload check failed: ${name}`);
  }
}

export function request(baseUrl, method, path, name, token, body = null, expectedStatus = 200, extraHeaders = {}) {
  const response = http.request(method, `${baseUrl}${path}`, body, {
    headers: { ...(__ENV.K3S_E2E === 'true' ? { 'CF-Connecting-IP': `198.51.100.${70 + __VU}` } : {}), Authorization: `Bearer ${token}`, ...extraHeaders },
    tags: { name },
    timeout: '15s',
    redirects: 0,
    responseCallback: http.expectedStatuses(expectedStatus),
  });
  if (response.status !== expectedStatus) {
    const status = String(response.status);
    unexpectedResponses.add(1, { status: diagnosticStatuses.includes(status) ? status : 'other' });
    if (response.status === 0) {
      clientErrorCodes.add(response.error_code || 0);
      clientErrorIterations.add(__ITER);
      if (/\bEOF\b/.test(response.error || '')) clientEof.add(1);
    }
    let errorCode = 'unparsed_response';
    try {
      const candidate = response.json('error.code');
      if (typeof candidate === 'string' && /^[a-z0-9_]{1,64}$/.test(candidate)) {
        errorCode = candidate;
      }
    } catch (_) {
      // A failed binary response need not contain a JSON error envelope.
    }
    console.error(`${name}: expected ${expectedStatus}, received ${response.status}, code=${errorCode}`);
  }
  requireCheck(response.status === expectedStatus, `${name}: expected HTTP status`);
  return response;
}

export function json(response) {
  try {
    return response.json();
  } catch (_) {
    requireCheck(false, 'response contains valid JSON');
  }
}

export function responseHeader(response, name) {
  const key = Object.keys(response.headers).find(key => key.toLowerCase() === name.toLowerCase());
  return response.headers[key];
}

export function jsonRequest(baseUrl, method, path, name, token, body, expectedStatus = 200) {
  return request(baseUrl, method, path, name, token, JSON.stringify(body), expectedStatus, {
    'Content-Type': 'application/json',
  });
}
