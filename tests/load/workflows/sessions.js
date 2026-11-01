import { Counter } from 'k6/metrics';
import { json, jsonRequest, request, requireCheck } from '../helpers.js';

const sessionRevocations = new Counter('session_revocations');
let revocationCompleted = false;

export function verifySessions(firstApi, secondApi, account, token) {
  if (!revocationCompleted) {
    request(secondApi, 'GET', '/api/v1/account', 'revocable_session_before_revoke', account.revocable_token);
    request(firstApi, 'DELETE', `/api/v1/sessions/${account.revocable_session_id}`,
      'session_revoke', token, null, 204);
    revocationCompleted = true;
    sessionRevocations.add(1);
  }
  request(secondApi, 'GET', '/api/v1/account', 'revoked_session_rejected', account.revocable_token, null, 401);
  const previousToken = token;
  const rotated = json(request(firstApi, 'POST', '/api/v1/sessions/current/rotate',
    'session_rotate', token));
  requireCheck(rotated.status === 'authenticated' && typeof rotated.token === 'string'
    && rotated.token !== previousToken, 'session rotation produces a new credential');
  token = rotated.token;
  request(secondApi, 'GET', '/api/v1/account', 'rotated_old_token_rejected', previousToken, null, 401);
  const authenticated = json(request(secondApi, 'GET', '/api/v1/account',
    'rotated_token_accepted', token));
  requireCheck(authenticated.id === account.account_id && authenticated.verified,
    'new session token retains the correct verified account');
  return token;
}
