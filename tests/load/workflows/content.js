import { sleep } from 'k6';
import { Trend } from 'k6/metrics';
import { json, jsonRequest, request, requireCheck } from '../helpers.js';

const searchVisibility = new Trend('content_search_visibility_ms', true);
let profileVersion = 0;
let commenterProfileCreated = false;

function waitForSearch(baseUrl, path, name, token, isVisible) {
  const started = Date.now();
  let page;
  do {
    page = json(request(baseUrl, 'GET', path, name, token));
    if (isVisible(page)) break;
    sleep(0.25);
  } while (Date.now() - started < 10000);
  searchVisibility.add(Date.now() - started);
  requireCheck(isVisible(page) && Date.now() - started < 10000,
    name + ': current projection becomes visible within the bounded wait');
  return page;
}

export function runContentWorkflow(firstApi, secondApi, organizationPath, foreignOrganizationPath, account, token, attachmentId) {
  const marker = 'contentvu' + __VU + 'iteration' + __ITER;
  const profileMarker = 'profilevu' + __VU + 'iteration' + __ITER;
  const profile = json(jsonRequest(firstApi, 'PUT', '/api/v1/account/profile',
    'profile_save', token, {
      display_name: profileMarker, bio: 'Load test profile', expected_version: profileVersion,
    }));
  profileVersion = profile.version;
  const observed = json(request(secondApi, 'GET', '/api/v1/account/profile', 'profile_read', token));
  requireCheck(observed.account_id === profile.account_id && observed.version === profileVersion,
    'profile changes are visible on the other API');
  if (!commenterProfileCreated) {
    jsonRequest(secondApi, 'PUT', '/api/v1/account/profile', 'commenter_profile_save',
      account.commenter_token, {
        display_name: 'Load commenter ' + __VU, bio: '', expected_version: 0,
      });
    commenterProfileCreated = true;
  }
  const usersPath = organizationPath + '/users';
  const users = json(request(firstApi, 'GET', usersPath + '?limit=1', 'users_first_page', token));
  requireCheck(users.items.length === 1 && typeof users.next_cursor === 'string',
    'user cursor page has one item and a continuation');
  const nextUsers = json(request(secondApi, 'GET',
    usersPath + '?limit=1&cursor=' + encodeURIComponent(users.next_cursor), 'users_second_page', token));
  requireCheck(nextUsers.items.length === 1 && nextUsers.items[0].account_id !== users.items[0].account_id
    && nextUsers.next_cursor === null, 'user cursor pagination returns both members once');

  const postsPath = organizationPath + '/posts';
  const posts = ['published', 'published', 'draft'].map((state, index) =>
    json(jsonRequest(firstApi, 'POST', postsPath, 'post_create', token, {
      title: marker + ' item ' + index, body: 'Organization content',
      state, attachment_ids: index === 0 ? [attachmentId] : [], expected_version: 0,
    }, 201)));
  const postPath = postsPath + '/' + posts[0].id;
  posts[0] = json(jsonRequest(secondApi, 'PUT', postPath, 'post_update', token, {
    title: marker + ' revised', body: 'Updated organization content',
    state: 'published', attachment_ids: [attachmentId], expected_version: 1,
  }));
  const attached = json(request(firstApi, 'GET', postPath, 'post_attachment_read', token));
  requireCheck(attached.attachment_ids.length === 1 && attached.attachment_ids[0] === attachmentId,
    'deployed post hydrates its completed original attachment');
  requireCheck(posts[0].version === 2, 'post update advances its revision');
  jsonRequest(firstApi, 'PUT', postPath, 'post_stale_write', token, {
    title: marker, body: 'Stale change', state: 'published', attachment_ids: [], expected_version: 1,
  }, 409);
  request(secondApi, 'GET', postsPath + '/' + posts[2].id, 'draft_hidden_from_member',
    account.commenter_token, null, 404);
  request(firstApi, 'GET', foreignOrganizationPath + '/posts/' + posts[0].id,
    'foreign_post_hidden', token, null, 404);
  const firstPage = json(request(firstApi, 'GET', postsPath + '?limit=1', 'posts_first_page', token));
  const secondPage = json(request(secondApi, 'GET',
    postsPath + '?limit=1&cursor=' + encodeURIComponent(firstPage.next_cursor),
    'posts_second_page', token));
  requireCheck(firstPage.items.length === 1 && secondPage.items.length === 1
    && firstPage.items[0].id !== secondPage.items[0].id && secondPage.next_cursor === null,
  'post cursor pagination returns each published post once');

  const commentsPath = postPath + '/comments';
  const comments = [1, 2].map(index => json(jsonRequest(secondApi, 'POST', commentsPath,
    'comment_create', account.commenter_token, { body: 'Reply ' + index }, 201)));
  const commentsPage = json(request(firstApi, 'GET', commentsPath + '?limit=1',
    'comments_first_page', token));
  const nextComments = json(request(secondApi, 'GET',
    commentsPath + '?limit=1&cursor=' + encodeURIComponent(commentsPage.next_cursor),
    'comments_second_page', token));
  requireCheck(commentsPage.items[0].id !== nextComments.items[0].id
    && nextComments.next_cursor === null, 'comment cursors do not repeat rows');
  const unread = json(request(firstApi, 'GET', '/api/v1/notifications/unread-count',
    'notification_unread_count', token));
  requireCheck(unread.unread_count === 2, 'both comments atomically create author notifications');
  const inbox = json(request(secondApi, 'GET', '/api/v1/notifications?limit=1',
    'notifications_first_page', token));
  const nextInbox = json(request(firstApi, 'GET',
    '/api/v1/notifications?limit=1&cursor=' + encodeURIComponent(inbox.next_cursor),
    'notifications_second_page', token));
  requireCheck(inbox.items[0].id !== nextInbox.items[0].id && nextInbox.next_cursor === null
    && inbox.items.every(item => item.post_id === posts[0].id),
  'inbox pagination remains scoped to the recipient and visible post');
  request(firstApi, 'PUT', '/api/v1/notifications/' + inbox.items[0].id + '/read',
    'notification_mark_read', token, null, 204);
  const readBatch = json(request(secondApi, 'POST', '/api/v1/notifications/read-batch',
    'notification_read_batch', token));
  requireCheck(readBatch.updated === 1, 'batch read updates only remaining unread items');
  const readCount = json(request(firstApi, 'GET', '/api/v1/notifications/unread-count',
    'notification_read_count', token));
  requireCheck(readCount.unread_count === 0, 'read state is shared across API instances');

  const searchPath = postsPath + '/search?q=' + marker + '&per_page=1';
  const found = waitForSearch(secondApi, searchPath, 'post_search_poll', token,
    page => page.total_in_window === 2 && page.items.length === 1);
  const nextFound = json(request(firstApi, 'GET', searchPath + '&page=2', 'post_search_page_two', token));
  requireCheck(found.has_next_page && !nextFound.has_next_page
    && nextFound.items.length === 1 && found.items[0].id !== nextFound.items[0].id
    && found.items.every(post => post.state === 'published'),
  'numbered search pages hydrate only current authorized published rows');
  const profiles = waitForSearch(firstApi, usersPath + '/search?q=' + profileMarker,
    'profile_search_poll', token,
    page => page.total_in_window === 1 && page.items[0].version === profileVersion);
  requireCheck(profiles.items[0].account_id === account.account_id
    && profiles.items[0].email === undefined, 'profile search excludes email and authentication data');
  request(secondApi, 'GET', foreignOrganizationPath + '/posts/search?q=' + marker,
    'foreign_post_search_hidden', token, null, 404);

  for (let index = 0; index < 2; index += 1) {
    const external = json(request(index === 0 ? firstApi : secondApi, 'GET', '/api/v1/external/posts',
      'external_posts', account.api_keys[index]));
    requireCheck(external.items.length === 2
      && external.items.every(post => post.organization_id === account.organization_ids[0]
        && post.state === 'published' && post.id !== posts[2].id),
    'API keys share the published post scope across API instances');
  }
  request(secondApi, 'DELETE', commentsPath + '/' + comments[0].id + '?expected_version=1',
    'comment_delete', account.commenter_token, null, 204);
  for (const post of posts) {
    request(firstApi, 'DELETE', postsPath + '/' + post.id + '?expected_version=' + post.version,
      'post_delete', token, null, 204);
  }
  const afterDelete = json(request(secondApi, 'GET', searchPath, 'post_search_after_delete', token));
  requireCheck(afterDelete.total_in_window === 0 && afterDelete.items.length === 0,
    'database authority immediately hides deleted search projections');
  const hiddenInbox = json(request(firstApi, 'GET', '/api/v1/notifications',
    'inbox_after_post_delete', token));
  requireCheck(hiddenInbox.items.length === 0,
    'notifications for deleted parent posts disappear immediately');
}
