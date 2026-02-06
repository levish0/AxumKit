# Posts

AxumKit provides a complete CRUD API for posts with content stored in SeaweedFS.

## Data Model

```
posts
├── id          UUID v7 (PK)
├── author_id   UUID (FK → users, CASCADE)
├── title       String
├── storage_key String (SeaweedFS path)
├── created_at  TimestampTZ
└── updated_at  TimestampTZ
```

Post content (body) is not stored in PostgreSQL. Instead, the `storage_key` points to a blob in SeaweedFS, keeping the database lean.

## Endpoints

### Create Post

```
POST /v0/posts
```

Requires authentication. Creates a post and stores content in SeaweedFS.

### List Posts

```
GET /v0/posts
```

Public. Returns paginated posts.

### Get Post

```
GET /v0/posts/{id}
```

Public. Returns a single post by ID.

### Update Post

```
PATCH /v0/posts/{id}
```

Requires authentication. Only the post author can update. Updates content in SeaweedFS.

### Delete Post

```
DELETE /v0/posts/{id}
```

Requires authentication. Only the post author can delete. Publishes a storage cleanup job to NATS for asynchronous content deletion.

## Storage Architecture

```
Create Post
    │
    ├─ Save metadata (title, author) → PostgreSQL
    ├─ Save content → SeaweedFS (key: posts/{post_id})
    └─ Publish index job → NATS → Worker → MeiliSearch

Delete Post
    │
    ├─ Delete metadata → PostgreSQL
    ├─ Publish delete_content job → NATS → Worker → SeaweedFS
    └─ Post is removed from MeiliSearch index
```

## Search Integration

Posts are automatically indexed in MeiliSearch when created or updated. The server publishes an indexing job to NATS, and the worker consumer updates the MeiliSearch index.

See [Search](/features/search) for query details.

## Action Logging

Post operations are recorded in the `action_logs` table:

| Action | Description |
|--------|-------------|
| `post:create` | Post created |
| `post:edit` | Post updated |
| `post:delete` | Post deleted |
