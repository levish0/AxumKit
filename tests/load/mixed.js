import { sleep } from 'k6';
import { Counter } from 'k6/metrics';
import { diagnosticStatuses, request, requireCheck } from './helpers.js';
import { verifyOrganizationPagination, renameOrganization } from './workflows/organizations.js';
import { verifySessions } from './workflows/sessions.js';
import { verifyExternalApi } from './workflows/external_api.js';
import { transferOriginal, deleteOriginal } from './workflows/files.js';
import { runContentWorkflow } from './workflows/content.js';

const accounts = JSON.parse(__ENV.LOAD_ACCOUNTS);
const apiUrls = [__ENV.FIRST_API_URL, __ENV.SECOND_API_URL];
const extended = __ENV.K6_PROFILE === 'extended';
const virtualUsers = extended ? 20 : 2;
const completedWorkflows = new Counter('completed_workflows');
const workflowFailures = new Counter('workflow_failures');
const thresholds = {
  http_req_failed: ['rate==0'],
  http_req_duration: ['p(95)<2000'],
  checks: ['rate==1'],
  session_revocations: [`count==${virtualUsers}`],
  workflow_failures: ['count==0'],
  search_visibility_ms: ['p(95)<10000'],
  content_search_visibility_ms: ['p(95)<10000'],
};
for (let virtualUser = 1; virtualUser <= virtualUsers; virtualUser += 1) {
  thresholds[`completed_workflows{virtual_user:${virtualUser}}`] = ['count>0'];
}
for (const status of diagnosticStatuses) {
  thresholds[`unexpected_http_responses{status:${status}}`] = ['count==0'];
}

export const options = {
  scenarios: {
    mixed: {
      executor: 'constant-vus',
      vus: virtualUsers,
      duration: extended ? '2m' : '10s',
      gracefulStop: '30s',
    },
  },
  thresholds,
};

let currentToken;

function runWorkflow() {
  const account = accounts[__VU - 1];
  requireCheck(!!account, 'each virtual user has a dedicated account fixture');
  if (currentToken === undefined) {
    currentToken = account.token;
    sleep((__VU - 1) * 0.2);
  }
  const started = Date.now();
  const firstApi = apiUrls[(__VU + __ITER) % 2];
  const secondApi = apiUrls[(__VU + __ITER + 1) % 2];
  const organizationPath = `/api/v1/organizations/${account.organization_ids[0]}`;
  const foreignAccount = accounts[__VU % accounts.length];
  const foreignOrganizationPath = `/api/v1/organizations/${foreignAccount.organization_ids[0]}`;
  request(firstApi, 'GET', '/health/ready', 'readiness', currentToken, null, 204);
  verifyOrganizationPagination(firstApi, secondApi, account, currentToken);
  renameOrganization(firstApi, secondApi, organizationPath, currentToken);
  request(secondApi, 'GET', foreignOrganizationPath, 'foreign_organization_rejected', currentToken, null, 404);
  currentToken = verifySessions(firstApi, secondApi, account, currentToken);
  verifyExternalApi(firstApi, secondApi, account);
  const original = transferOriginal(firstApi, secondApi, organizationPath, foreignOrganizationPath, currentToken);
  runContentWorkflow(firstApi, secondApi, organizationPath, foreignOrganizationPath, account, currentToken, original.id);
  deleteOriginal(firstApi, secondApi, original, currentToken);
  completedWorkflows.add(1, { virtual_user: String(__VU) });
  // Pace reservations below the ordinary per-account limit without weakening it.
  sleep(Math.max(0, 5 - (Date.now() - started) / 1000));
}

export default function () {
  workflowFailures.add(0);
  try {
    runWorkflow();
  } catch (error) {
    workflowFailures.add(1);
    throw error;
  }
}

export function handleSummary(data) {
  const values = name => data.metrics[name]?.values || {};
  const result = {
    requests: values('http_reqs').count || 0,
    failed_requests: values('http_req_failed').rate || 0,
    failed_checks: values('checks').fails || 0,
    workflow_failures: values('workflow_failures').count || 0,
    completed_workflows: values('completed_workflows').count || 0,
    session_revocations: values('session_revocations').count || 0,
    p95_milliseconds: values('http_req_duration')['p(95)'] || 0,
    failed_assertions: data.root_group.checks.filter(check => check.fails > 0)
      .map(check => ({ name: check.name, failures: check.fails })),
    unexpected_statuses: diagnosticStatuses.map(status => ({
      status, count: values(`unexpected_http_responses{status:${status}}`).count || 0,
    })).filter(result => result.count > 0),
    client_errors: {
      minimum_code: values('unexpected_client_error_codes').min || 0,
      maximum_code: values('unexpected_client_error_codes').max || 0,
      first_iteration: values('unexpected_client_error_iterations').min || 0,
      last_iteration: values('unexpected_client_error_iterations').max || 0,
      eof_count: values('unexpected_client_eof').count || 0,
    },
  };
  return { stdout: `K3S_E2E_RESULT ${JSON.stringify(result)}\n` };
}
