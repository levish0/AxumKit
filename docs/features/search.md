# Search

AxumKit integrates [MeiliSearch](https://www.meilisearch.com/) for full-text search across users.

## How It Works

```
User Created or Updated
    |
    `- Server publishes index job to NATS JetStream
       |
       `- Worker consumer picks up the job
          |
          `- Upserts document in MeiliSearch index
```

Indexing is asynchronous. API handlers return immediately while indexing happens in the background.

## Search Endpoint

### Search Users

```
GET /v0/search/users?q=keyword
```

Public endpoint. Searches users by `handle` and `display_name`.

## Indexes

The worker initializes MeiliSearch indexes on startup.

| Index | Searchable Fields |
|-------|-------------------|
| Users | handle, display_name |

## Bulk Reindexing

For recovery or schema changes, the worker supports bulk reindexing:

- Reindex all users: reads all users from PostgreSQL and indexes them in MeiliSearch.

Triggered via NATS JetStream job subject: `axumkit.jobs.reindex.users`.

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MEILISEARCH_HOST` | No | `http://localhost:7700` | MeiliSearch URL |
| `MEILISEARCH_API_KEY` | No | - | MeiliSearch API key (if auth enabled) |

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `meilisearch:query_failed` | 500 | MeiliSearch query execution failed |
