import { json, request, requireCheck, responseHeader } from '../helpers.js';

export function verifyExternalApi(firstApi, secondApi, account) {
  const primary = request(firstApi, 'GET', '/api/v1/external/organization',
    'external_organization_primary', account.api_keys[0]);
  const rotated = request(secondApi, 'GET', '/api/v1/external/organization',
    'external_organization_rotated', account.api_keys[1]);
  requireCheck(json(primary).id === account.organization_ids[0]
    && json(rotated).id === account.organization_ids[0],
  'both API keys resolve only their owning organization');
  requireCheck(responseHeader(primary, 'X-Quota-Limit') === '1000000'
    && responseHeader(rotated, 'X-RateLimit-Limit') === '100000',
  'external responses expose operator allocation');
  const previousRemaining = Number(responseHeader(primary, 'X-Quota-Remaining'));
  const nextRemaining = Number(responseHeader(rotated, 'X-Quota-Remaining'));
  const monthChanged = responseHeader(primary, 'X-Quota-Reset') !== responseHeader(rotated, 'X-Quota-Reset');
  requireCheck(nextRemaining === (monthChanged ? 999999 : previousRemaining - 1),
    'rotated keys across API instances share one monthly counter');
}
