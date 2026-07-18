use crate::DbPool;
use crate::clients::process_media;
use crate::connection::R2AssetsClient;
use crate::jobs::WorkerContext;
use crate::jobs::index::user::{IndexUserJob, UserIndexAction};
use crate::nats::consumer::NatsConsumer;
use crate::nats::publisher::publish_job;
use crate::nats::streams::{OAUTH_PROFILE_IMAGE_CONSUMER, OAUTH_PROFILE_IMAGE_STREAM};
use crate::nats::{JetStreamContext, streams::INDEX_USER_SUBJECT};
use constants::{PROFILE_IMAGE_MAX_SIZE, user_image_key};
use entity::users::{Column as UserColumn, Entity as UserEntity};
use reqwest::Client as HttpClient;
use sea_orm::prelude::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub use job_queue::jobs::oauth::OAuthProfileImageJob;

async fn handle_oauth_profile_image(
    job: OAuthProfileImageJob,
    http_client: &HttpClient,
    r2_assets: &R2AssetsClient,
    db: &DbPool,
    jetstream: &JetStreamContext,
) -> Result<(), anyhow::Error> {
    tracing::info!(user_id = %job.user_id, "Processing OAuth profile image job");

    let response = http_client.get(&job.image_url).send().await?;
    if !response.status().is_success() {
        tracing::warn!(
            user_id = %job.user_id,
            status = %response.status(),
            "OAuth profile image download failed"
        );
        return Ok(());
    }

    let image_bytes = response.bytes().await?;
    let prepared_file = match prepare_oauth_profile_image(http_client, image_bytes.to_vec()).await {
        Ok(file) => file,
        Err(err) => {
            tracing::warn!(
                user_id = %job.user_id,
                error = ?err,
                "OAuth profile image preparation failed"
            );
            return Ok(());
        }
    };

    let storage_key = user_image_key(&prepared_file.hash, &prepared_file.extension);

    r2_assets
        .upload_with_content_type(&storage_key, prepared_file.bytes, &prepared_file.mime_type)
        .await?;

    let result = UserEntity::update_many()
        .col_expr(
            UserColumn::ProfileImage,
            Expr::value(Some(storage_key.clone())),
        )
        .filter(UserColumn::Id.eq(job.user_id))
        .filter(UserColumn::ProfileImage.is_null())
        .exec(db.as_ref())
        .await?;

    if result.rows_affected == 0 {
        if let Err(err) = r2_assets.delete(&storage_key).await {
            tracing::warn!(
                user_id = %job.user_id,
                storage_key = %storage_key,
                error = ?err,
                "Failed to cleanup unused OAuth profile image"
            );
        }
        return Ok(());
    }

    let index_job = IndexUserJob {
        user_id: job.user_id,
        action: UserIndexAction::Index,
    };
    publish_job(jetstream, INDEX_USER_SUBJECT, &index_job).await?;

    tracing::info!(
        user_id = %job.user_id,
        storage_key = %storage_key,
        "OAuth profile image uploaded"
    );

    Ok(())
}

struct PreparedOAuthProfileImage {
    bytes: Vec<u8>,
    mime_type: String,
    extension: String,
    hash: String,
}

async fn prepare_oauth_profile_image(
    http_client: &HttpClient,
    file: Vec<u8>,
) -> Result<PreparedOAuthProfileImage, anyhow::Error> {
    if file.is_empty() {
        anyhow::bail!("empty file");
    }

    if file.len() > PROFILE_IMAGE_MAX_SIZE {
        anyhow::bail!(
            "file too large: {} bytes (max: {} bytes)",
            file.len(),
            PROFILE_IMAGE_MAX_SIZE
        );
    }

    let processed = process_media(http_client, file).await?;
    if processed.bytes.is_empty() {
        anyhow::bail!("processed image is empty");
    }

    if processed.bytes.len() > PROFILE_IMAGE_MAX_SIZE {
        anyhow::bail!(
            "processed file too large: {} bytes (max: {} bytes)",
            processed.bytes.len(),
            PROFILE_IMAGE_MAX_SIZE
        );
    }

    if processed.mime_type != "image/webp" {
        anyhow::bail!("unexpected processed image type: {}", processed.mime_type);
    }

    if processed.extension != "webp" {
        anyhow::bail!(
            "unexpected processed image extension: {}",
            processed.extension
        );
    }

    let hash = blake3::hash(&processed.bytes).to_hex().to_string();

    Ok(PreparedOAuthProfileImage {
        bytes: processed.bytes,
        mime_type: processed.mime_type,
        extension: processed.extension,
        hash,
    })
}

pub async fn run_consumer(ctx: WorkerContext) -> anyhow::Result<()> {
    let http_client = HttpClient::builder()
        .user_agent("axumkit-worker/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let r2_assets = ctx.r2_assets.clone();
    let db_pool = ctx.db_pool.clone();
    let jetstream = ctx.jetstream.clone();

    let consumer = NatsConsumer::new(
        ctx.jetstream.clone(),
        OAUTH_PROFILE_IMAGE_STREAM,
        OAUTH_PROFILE_IMAGE_CONSUMER,
        2,
    )
    // Re-running uploads a duplicate avatar blob: skip redelivered messages already done.
    .with_dedup(ctx.lock_client.clone());

    consumer
        .run::<OAuthProfileImageJob, _, _>(move |job| {
            let http_client = http_client.clone();
            let r2_assets = r2_assets.clone();
            let db = db_pool.clone();
            let jetstream = jetstream.clone();
            async move {
                handle_oauth_profile_image(job, &http_client, &r2_assets, &db, &jetstream).await
            }
        })
        .await
}
