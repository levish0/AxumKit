import http from 'k6/http';
import { sleep } from 'k6';
import { Trend } from 'k6/metrics';
import { json, jsonRequest, request, requireCheck } from '../helpers.js';

const searchVisibility = new Trend('search_visibility_ms', true);

export function transferOriginal(firstApi, secondApi, organizationPath, foreignOrganizationPath, token) {
  const payload = `untrusted-original-vu-${__VU}-iteration-${__ITER}\0`.repeat(512);
  const searchTerm = `vu${__VU}iteration${__ITER}original`;
  const filesPath = `${organizationPath}/files`;
  const reserved = json(jsonRequest(firstApi, 'POST', filesPath, 'file_reserve', token,
    { filename: `${searchTerm}.bin`, media_type: 'application/octet-stream', byte_size: payload.length }, 201));
  requireCheck(reserved.state === 'uploading' && typeof reserved.id === 'string',
    'file reservation starts in uploading state');
  const filePath = `${filesPath}/${reserved.id}`;
  const upload = json(request(firstApi, 'POST', `${filePath}/upload-url`, 'file_upload_url', token));
  const uploadHeaders = Object.fromEntries(upload.headers.map(header => [header.name, header.value]));
  const sent = http.put(upload.url, payload, { headers: uploadHeaders, tags: { name: 'storage_upload', url: 'storage_upload' }, redirects: 0 });
  requireCheck(sent.status === 200, `direct upload completes (HTTP ${sent.status})`);
  const uploaded = json(request(firstApi, 'POST', `${filePath}/complete-upload`, 'file_complete', token));
  requireCheck(uploaded.state === 'ready' && uploaded.byte_size === payload.length,
    'completion verifies the declared size');
  const download = json(jsonRequest(secondApi, 'POST', `${filePath}/download-url`, 'file_download_url', token, { method: 'GET' }));
  const downloaded = http.get(download.url, { tags: { name: 'storage_download', url: 'storage_download' }, redirects: 0 });
  requireCheck(downloaded.status === 200 && downloaded.body === payload, 'download preserves original bytes');
  requireCheck(downloaded.headers['Content-Type'] === 'application/octet-stream'
    && downloaded.headers['Content-Disposition'].startsWith('attachment;'), 'original download retains attachment policy');
  const head = json(jsonRequest(secondApi, 'POST', `${filePath}/download-url`, 'file_head_url', token, { method: 'HEAD' }));
  const metadata = http.request('HEAD', head.url, null, { tags: { name: 'storage_head', url: 'storage_head' }, redirects: 0 });
  requireCheck(Number(metadata.headers['Content-Length']) === payload.length, 'HEAD exposes original length');
  const ranged = http.get(download.url, { headers: { Range: 'bytes=2-9' }, tags: { name: 'storage_range', url: 'storage_range' }, redirects: 0 });
  requireCheck(ranged.status === 206 && ranged.body === payload.slice(2, 10), 'range download preserves bytes');
  jsonRequest(secondApi, 'POST', `${foreignOrganizationPath}/files/${reserved.id}/download-url`,
    'foreign_organization_file_rejected', token, { method: 'GET' }, 404);

  const searchPath = `${filesPath}/search?q=${searchTerm}`;
  request(secondApi, 'GET', `${foreignOrganizationPath}/files/search?q=${searchTerm}`,
    'foreign_organization_search_rejected', token, null, 404);
  const started = Date.now();
  let visible = false;
  do {
    const results = json(request(secondApi, 'GET', searchPath, 'file_search_poll', token));
    requireCheck(Array.isArray(results.items) && results.items.every(file => file.organization_id === reserved.organization_id),
      'search results remain within their authorized organization');
    visible = results.items.some(file => file.id === reserved.id);
    if (!visible) sleep(0.25);
  } while (!visible && Date.now() - started < 10000);
  const visibilityMilliseconds = Date.now() - started;
  searchVisibility.add(visibilityMilliseconds);
  requireCheck(visible && visibilityMilliseconds < 10000,
    'worker makes the filename searchable within the bounded wait');

  return { id: reserved.id, filePath, searchPath };
}

export function deleteOriginal(firstApi, secondApi, original, token) {
  const { id, filePath, searchPath } = original;
  request(firstApi, 'DELETE', filePath, 'file_delete', token, null, 204);
  jsonRequest(secondApi, 'POST', `${filePath}/download-url`, 'deleted_file_rejected', token, { method: 'GET' }, 404);
  const afterDelete = json(request(secondApi, 'GET', searchPath, 'file_search_after_delete', token));
  requireCheck(afterDelete.items.every(file => file.id !== id),
    'deleted file disappears immediately even before index reconciliation');
}
