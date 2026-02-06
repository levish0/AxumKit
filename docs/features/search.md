# Search

AxumKit integrates [MeiliSearch](https://www.meilisearch.com/) for full-text search across posts and users.

## How It Works

```
User/Post Created or Updated
    │
    └─ Server publishes index job to NATS JetStream
       │
       └─ Worker consumer picks up the job
          │
          └─ Upserts document in MeiliSearch index
```

Indexing is **asynchronous** — the API response returns immediately, and the search index is updated in the background.

## Search Endpoints

### Search Posts

```
GET /v0/search/posts?q=keyword
```

Public. Searches posts by title and content.

### Search Users

```
GET /v0/search/users?q=keyword
```

Public. Searches users by handle and display name.

## Indexes

The worker initializes MeiliSearch indexes on startup, ensuring they exist before any queries are made.

| Index | Searchable Fields |
|-------|-------------------|
| Posts | title, content |
| Users | handle, display_name |

## Bulk Reindexing

For recovery or schema changes, the worker supports bulk reindexing:

- **Reindex all posts:** Reads all posts from PostgreSQL and indexes them in MeiliSearch
- **Reindex all users:** Same for users

These are triggered via NATS JetStream jobs (`axumkit.jobs.reindex.posts`, `axumkit.jobs.reindex.users`).

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MEILISEARCH_HOST` | No | `http://localhost:7700` | MeiliSearch URL |
| `MEILISEARCH_API_KEY` | No | — | MeiliSearch API key (if auth enabled) |

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `meilisearch:query_failed` | 500 | MeiliSearch query execution failed |
