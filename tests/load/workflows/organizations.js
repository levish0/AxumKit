import { json, jsonRequest, request, requireCheck } from '../helpers.js';

export function verifyOrganizationPagination(firstApi, secondApi, account, token) {
  const firstPage = json(request(firstApi, 'GET', '/api/v1/organizations?limit=1',
    'organizations_first_page', token));
  requireCheck(firstPage.items.length === 1 && typeof firstPage.next_cursor === 'string',
    'organization first page has one item and a cursor');
  const secondPage = json(request(secondApi, 'GET',
    `/api/v1/organizations?limit=1&cursor=${encodeURIComponent(firstPage.next_cursor)}`,
    'organizations_second_page', token));
  requireCheck(secondPage.items.length === 1 && secondPage.next_cursor === null,
    'organization second page completes pagination');
  const identifiers = [firstPage.items[0].id, secondPage.items[0].id];
  requireCheck(new Set(identifiers).size === 2
    && account.organization_ids.every(identifier => identifiers.includes(identifier)),
  'pagination returns exactly the current account organizations');
}

export function renameOrganization(firstApi, secondApi, organizationPath, token) {
  const organization = json(request(secondApi, 'GET', organizationPath,
    'organization_read', token));
  const name = `Load account ${__VU} iteration ${__ITER}`;
  const renamed = json(jsonRequest(firstApi, 'PUT', organizationPath,
    'organization_rename', token, { name, expected_version: organization.version }));
  requireCheck(renamed.name === name && renamed.version === organization.version + 1,
    'organization write advances its version');
  const observed = json(request(secondApi, 'GET', organizationPath,
    'organization_read_after_write', token));
  requireCheck(observed.name === name && observed.version === renamed.version,
    'organization write is visible on the other API');
  jsonRequest(secondApi, 'PUT', organizationPath, 'organization_stale_write', token,
    { name: 'This stale write must fail', expected_version: organization.version }, 409);
}
