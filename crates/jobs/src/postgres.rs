use crate::{
    AttemptOutcome, JobState,
    entities::{job, job_attempt},
};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection,
    DatabaseTransaction, DbErr, EntityTrait, ExprTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Set, TransactionTrait,
    sea_query::{Expr, Func, LockBehavior, LockType, OnConflict, Query},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

const APPLICATION_QUEUE: &str = "application";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TraceContext {
    pub request_id: Uuid,
}
impl Default for TraceContext {
    fn default() -> Self {
        Self {
            request_id: Uuid::now_v7(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("job storage unavailable")]
    Storage(#[from] DbErr),
    #[error("job lease lost")]
    LeaseLost,
    #[error("invalid job settings or payload")]
    Invalid,
}

#[derive(Clone)]
pub struct Claim {
    pub id: Uuid,
    pub kind: String,
    pub payload: Value,
    pub request_id: Uuid,
    pub attempts: i32,
    pub max_attempts: i32,
    pub claim_token: Uuid,
    pub deadline_at: DateTime<Utc>,
    pub execution_deadline: tokio::time::Instant,
}

#[tracing::instrument(skip_all, fields(job_id = %job_id, job_kind, request_id = %trace.request_id))]
pub async fn enqueue<T: Serialize>(
    transaction: &DatabaseTransaction,
    job_id: Uuid,
    job_kind: &str,
    payload: &T,
    trace: TraceContext,
) -> Result<(), JobError> {
    let payload = serde_json::to_value(payload).map_err(|_| JobError::Invalid)?;
    if job_kind.is_empty() || job_kind.len() > 100 || payload.to_string().len() > 16_384 {
        return Err(JobError::Invalid);
    }
    job::ActiveModel {
        id: Set(job_id),
        kind: Set(job_kind.into()),
        queue: Set(APPLICATION_QUEUE.into()),
        payload: Set(payload),
        request_id: Set(trace.request_id),
        max_attempts: Set(5),
        ..Default::default()
    }
    .insert(transaction)
    .await?;
    Ok(())
}

#[derive(Clone)]
pub struct PostgresQueue {
    database: DatabaseConnection,
    queue: String,
}
impl PostgresQueue {
    pub fn new(database: DatabaseConnection) -> Self {
        Self {
            database,
            queue: APPLICATION_QUEUE.into(),
        }
    }

    pub fn for_queue(&self, queue: &str) -> Result<Self, JobError> {
        if queue.is_empty()
            || queue.len() > 64
            || !queue
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(JobError::Invalid);
        }
        Ok(Self {
            database: self.database.clone(),
            queue: queue.into(),
        })
    }

    pub async fn schedule_periodic(
        &self,
        job_kind: &str,
        interval_seconds: i64,
        trace: TraceContext,
    ) -> Result<bool, JobError> {
        // Retention is at least one day, so the current bucket keeps its deduplication row.
        if !(1..=86400).contains(&interval_seconds) || job_kind.is_empty() || job_kind.len() > 100 {
            return Err(JobError::Invalid);
        }
        let transaction = self.database.begin().await?;
        let now = database_time(&transaction).await?;
        let bucket_start = now.timestamp() - now.timestamp().rem_euclid(interval_seconds);
        let inserted = job::Entity::insert(job::ActiveModel {
            id: Set(Uuid::now_v7()),
            kind: Set(job_kind.into()),
            queue: Set(self.queue.clone()),
            schedule_key: Set(Some(format!("{job_kind}:{bucket_start}"))),
            payload: Set(serde_json::json!({})),
            request_id: Set(trace.request_id),
            max_attempts: Set(5),
            ..Default::default()
        })
        .on_conflict(
            OnConflict::columns([job::Column::Queue, job::Column::ScheduleKey])
                .do_nothing()
                .to_owned(),
        )
        .exec_without_returning(&transaction)
        .await?;
        transaction.commit().await?;
        Ok(inserted == 1)
    }

    #[tracing::instrument(skip_all)]
    pub async fn claim(
        &self,
        lease_seconds: i32,
        execution_seconds: i32,
    ) -> Result<Option<Claim>, JobError> {
        if lease_seconds < 1 || execution_seconds < lease_seconds {
            return Err(JobError::Invalid);
        }
        loop {
            let transaction = self.database.begin().await?;
            let candidate = job::Entity::find()
                .filter(job::Column::Queue.eq(&self.queue))
                .filter(
                    Condition::any()
                        .add(
                            Condition::all()
                                .add(job::Column::State.eq(JobState::Pending))
                                .add(
                                    Expr::col(job::Column::AvailableAt)
                                        .lte(Func::cust("clock_timestamp")),
                                ),
                        )
                        .add(
                            Condition::all()
                                .add(job::Column::State.eq(JobState::Running))
                                .add(
                                    Expr::col(job::Column::LeaseUntil)
                                        .lte(Func::cust("clock_timestamp")),
                                ),
                        ),
                )
                .order_by_asc(job::Column::AvailableAt)
                .order_by_asc(job::Column::Id)
                .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked)
                .one(&transaction)
                .await?;
            let Some(candidate) = candidate else {
                transaction.commit().await?;
                return Ok(None);
            };
            let execution_started = tokio::time::Instant::now();
            let database_now = database_time(&transaction).await?;
            if let Some(expired_attempt_id) = candidate.claim_token {
                let error_code = if candidate
                    .deadline_at
                    .is_some_and(|deadline| deadline <= database_now)
                {
                    "execution_timeout"
                } else {
                    "lease_expired"
                };
                job_attempt::Entity::update_many()
                    .col_expr(job_attempt::Column::FinishedAt, Expr::value(database_now))
                    .col_expr(
                        job_attempt::Column::Outcome,
                        Expr::value(AttemptOutcome::LeaseExpired),
                    )
                    .col_expr(job_attempt::Column::ErrorCode, Expr::value(error_code))
                    .filter(job_attempt::Column::Id.eq(expired_attempt_id))
                    .filter(job_attempt::Column::FinishedAt.is_null())
                    .exec(&transaction)
                    .await?;
            }
            let mut next_state = candidate.clone().into_active_model();
            if candidate.attempts >= candidate.max_attempts {
                next_state.state = Set(JobState::Failed);
                next_state.finished_at = Set(Some(database_now));
                next_state.last_error = Set(Some("attempts_exhausted".into()));
                next_state.claim_token = Set(None);
                next_state.lease_until = Set(None);
                next_state.deadline_at = Set(None);
                next_state.update(&transaction).await?;
                transaction.commit().await?;
                continue;
            }
            if candidate.state == JobState::Running {
                next_state.state = Set(JobState::Pending);
                next_state.available_at = Set(std::cmp::max(
                    candidate.available_at,
                    database_now + Duration::seconds(retry_delay_seconds(candidate.attempts)),
                ));
                next_state.last_error = Set(Some(
                    if candidate
                        .deadline_at
                        .is_some_and(|deadline| deadline <= database_now)
                    {
                        "execution_timeout"
                    } else {
                        "lease_expired"
                    }
                    .into(),
                ));
                next_state.claim_token = Set(None);
                next_state.lease_until = Set(None);
                next_state.deadline_at = Set(None);
                next_state.update(&transaction).await?;
                transaction.commit().await?;
                continue;
            }
            let attempt_id = Uuid::now_v7();
            let deadline_at = database_now + Duration::seconds(i64::from(execution_seconds));
            next_state.state = Set(JobState::Running);
            next_state.attempts = Set(candidate.attempts + 1);
            next_state.claim_token = Set(Some(attempt_id));
            next_state.lease_until = Set(Some(
                database_now + Duration::seconds(i64::from(lease_seconds)),
            ));
            next_state.deadline_at = Set(Some(deadline_at));
            next_state.update(&transaction).await?;
            job_attempt::ActiveModel {
                id: Set(attempt_id),
                job_id: Set(candidate.id),
                attempt: Set(candidate.attempts + 1),
                ..Default::default()
            }
            .insert(&transaction)
            .await?;
            transaction.commit().await?;
            return Ok(Some(Claim {
                id: candidate.id,
                kind: candidate.kind,
                payload: candidate.payload,
                request_id: candidate.request_id,
                attempts: candidate.attempts + 1,
                max_attempts: candidate.max_attempts,
                claim_token: attempt_id,
                deadline_at,
                execution_deadline: execution_started
                    + std::time::Duration::from_secs(execution_seconds as u64),
            }));
        }
    }

    pub async fn heartbeat(&self, claim: &Claim, lease_seconds: i32) -> Result<(), JobError> {
        if lease_seconds < 1 {
            return Err(JobError::Invalid);
        }
        let lease_until = std::cmp::min(
            database_time(&self.database).await? + Duration::seconds(i64::from(lease_seconds)),
            claim.deadline_at,
        );
        let updated = job::Entity::update_many()
            .col_expr(job::Column::LeaseUntil, Expr::value(lease_until))
            .filter(valid_claim(claim))
            .filter(Expr::col(job::Column::DeadlineAt).gt(Func::cust("clock_timestamp")))
            .exec(&self.database)
            .await?;
        if updated.rows_affected != 1 {
            return Err(JobError::LeaseLost);
        }
        Ok(())
    }

    pub async fn finish(
        &self,
        claim: &Claim,
        failure: Option<(&str, bool)>,
    ) -> Result<(), JobError> {
        let (next_state, outcome, error_code, retry_delay) = match failure {
            None => (JobState::Succeeded, AttemptOutcome::Succeeded, None, 0),
            Some((code, true)) if claim.attempts < claim.max_attempts => (
                JobState::Pending,
                AttemptOutcome::Retry,
                Some(code),
                retry_delay_seconds(claim.attempts),
            ),
            Some((code, _)) => (JobState::Failed, AttemptOutcome::Failed, Some(code), 0),
        };
        if error_code.is_some_and(|code| code.len() > 100) {
            return Err(JobError::Invalid);
        }
        let transaction = self.database.begin().await?;
        let database_now = database_time(&transaction).await?;
        let finished_at = (next_state != JobState::Pending).then_some(database_now);
        let updated = job::Entity::update_many()
            .col_expr(job::Column::State, Expr::value(next_state))
            .col_expr(job::Column::LastError, Expr::value(error_code))
            .col_expr(
                job::Column::AvailableAt,
                Func::cust("greatest")
                    .arg(Expr::col(job::Column::AvailableAt))
                    .arg(database_now + Duration::seconds(retry_delay))
                    .into(),
            )
            .col_expr(job::Column::FinishedAt, Expr::value(finished_at))
            .col_expr(job::Column::ClaimToken, Expr::value(Option::<Uuid>::None))
            .col_expr(
                job::Column::LeaseUntil,
                Expr::value(Option::<DateTime<Utc>>::None),
            )
            .col_expr(
                job::Column::DeadlineAt,
                Expr::value(Option::<DateTime<Utc>>::None),
            )
            .filter(valid_claim(claim))
            .exec(&transaction)
            .await?;
        if updated.rows_affected != 1 {
            return Err(JobError::LeaseLost);
        }
        job_attempt::Entity::update_many()
            .col_expr(job_attempt::Column::FinishedAt, Expr::value(database_now))
            .col_expr(job_attempt::Column::Outcome, Expr::value(outcome))
            .col_expr(job_attempt::Column::ErrorCode, Expr::value(error_code))
            .filter(job_attempt::Column::Id.eq(claim.claim_token))
            .exec(&transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn defer_retry(&self, claim: &Claim, seconds: u64) -> Result<(), JobError> {
        if !(1..=86400).contains(&seconds) {
            return Err(JobError::Invalid);
        }
        let now = database_time(&self.database).await?;
        let updated = job::Entity::update_many()
            .col_expr(
                job::Column::AvailableAt,
                Func::cust("greatest")
                    .arg(Expr::col(job::Column::AvailableAt))
                    .arg(now + Duration::seconds(seconds as i64))
                    .into(),
            )
            .filter(valid_claim(claim))
            .exec(&self.database)
            .await?;
        if updated.rows_affected != 1 {
            return Err(JobError::LeaseLost);
        }
        Ok(())
    }

    pub async fn cleanup_finished(
        &self,
        succeeded_days: i64,
        failed_days: i64,
    ) -> Result<u64, JobError> {
        if !(1..=36500).contains(&succeeded_days) || !(1..=36500).contains(&failed_days) {
            return Err(JobError::Invalid);
        }
        let transaction = self.database.begin().await?;
        let now = database_time(&transaction).await?;
        let expired = Condition::any()
            .add(
                Condition::all()
                    .add(job::Column::State.eq(JobState::Succeeded))
                    .add(job::Column::FinishedAt.lt(now - Duration::days(succeeded_days))),
            )
            .add(
                Condition::all()
                    .add(job::Column::State.eq(JobState::Failed))
                    .add(job::Column::FinishedAt.lt(now - Duration::days(failed_days))),
            );
        let eligible = job::Entity::find()
            .select_only()
            .column(job::Column::Id)
            .filter(expired.clone())
            .order_by_asc(job::Column::FinishedAt)
            .order_by_asc(job::Column::Id)
            .limit(100)
            .into_query();
        let deleted = job::Entity::delete_many()
            .filter(job::Column::Id.in_subquery(eligible))
            // Retry or completion may change both state and timestamp while deletion waits.
            .filter(expired)
            .exec(&transaction)
            .await?;
        transaction.commit().await?;
        Ok(deleted.rows_affected)
    }

    pub async fn retry_failed(&self, job_id: Uuid) -> Result<bool, JobError> {
        Ok(job::Entity::update_many()
            .col_expr(job::Column::State, Expr::value(JobState::Pending))
            .col_expr(
                job::Column::MaxAttempts,
                Expr::col(job::Column::Attempts).add(5),
            )
            .col_expr(
                job::Column::AvailableAt,
                Expr::expr(Func::cust("clock_timestamp")),
            )
            .col_expr(
                job::Column::FinishedAt,
                Expr::value(Option::<DateTime<Utc>>::None),
            )
            .filter(job::Column::Id.eq(job_id))
            .filter(job::Column::State.eq(JobState::Failed))
            .filter(job::Column::Attempts.lte(995))
            .exec(&self.database)
            .await?
            .rows_affected
            == 1)
    }
}

fn valid_claim(claim: &Claim) -> Condition {
    Condition::all()
        .add(job::Column::Id.eq(claim.id))
        .add(job::Column::ClaimToken.eq(claim.claim_token))
        .add(job::Column::State.eq(JobState::Running))
        .add(Expr::col(job::Column::LeaseUntil).gt(Func::cust("clock_timestamp")))
}

fn retry_delay_seconds(attempt: i32) -> i64 {
    2_i64.pow(std::cmp::min(attempt, 8) as u32)
}

async fn database_time(connection: &impl ConnectionTrait) -> Result<DateTime<Utc>, DbErr> {
    connection
        .query_one(
            &Query::select()
                .expr_as(Func::cust("clock_timestamp"), "database_time")
                .to_owned(),
        )
        .await?
        .ok_or_else(|| DbErr::Custom("database clock unavailable".into()))?
        .try_get("", "database_time")
}
