#[path = "job_load/fixture.rs"]
mod fixture;

use anyhow::{Context, Result, ensure};
use fixture::{BULK_JOB, NOTIFICATION_JOB, record};
use jobs::{
    HandlerFailure, PostgresQueue, WorkerSettings,
    entities::{job, job_attempt},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use test_support::TestDatabase;
use tokio::{sync::mpsc, task::JoinSet};
use tokio_util::sync::CancellationToken;

#[derive(Serialize)]
struct Latency {
    samples: usize,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    max_ms: f64,
}

impl Latency {
    fn from_samples(mut values: Vec<f64>) -> Result<Self> {
        ensure!(!values.is_empty(), "latency samples missing");
        values.sort_by(f64::total_cmp);
        let percentile = |percent: usize| values[(values.len() * percent).div_ceil(100) - 1];
        Ok(Self {
            samples: values.len(),
            p50_ms: percentile(50),
            p95_ms: percentile(95),
            p99_ms: percentile(99),
            max_ms: *values.last().unwrap(),
        })
    }
}

#[derive(Serialize)]
struct LoadReport {
    worker_pools: usize,
    concurrency_per_pool: usize,
    producers: usize,
    retained_history_rows: usize,
    registered_jobs: usize,
    succeeded_jobs: usize,
    attempts: usize,
    verified_effects: u64,
    elapsed_seconds: f64,
    scenario_jobs_per_second: f64,
    registration: Latency,
    bulk_registration_to_handler: Latency,
    notification_registration_to_handler: Latency,
    foreground_baseline: Latency,
    foreground_during_load: Latency,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires PostgreSQL; run just test-job-load"]
async fn postgres_queue_load_preserves_jobs_and_records_foreground_database_latency() -> Result<()>
{
    let profile = std::env::var("JOB_LOAD_PROFILE").unwrap_or_else(|_| "smoke".into());
    let (jobs, history) = match profile.as_str() {
        "smoke" => (2_000, 10_000),
        "extended" => (20_000, 100_000),
        _ => anyhow::bail!("JOB_LOAD_PROFILE must be smoke or extended"),
    };
    let mut reports = Vec::new();
    let report_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/job-load-report.json");
    std::fs::create_dir_all(report_path.parent().unwrap())?;
    if report_path.exists() {
        std::fs::remove_file(&report_path)?;
    }
    for worker_pools in [1, 2, 4] {
        reports.push(run_load(worker_pools, jobs, history).await?);
        let document = serde_json::json!({
            "profile": profile,
            "debug_assertions": cfg!(debug_assertions),
            "completed_scenarios": reports.len(),
            "expected_scenarios": 3,
            "scope": "PostgreSQL queue and synthetic database effects; excludes HTTP and external providers",
            "bulk_handler_delay_ms": 2,
            "notification_frequency": "one in sixteen per producer",
            "scenarios": reports,
        });
        let json = serde_json::to_string_pretty(&document)?;
        std::fs::write(&report_path, format!("{json}\n"))?;
        eprintln!("{json}");
    }
    eprintln!("Job load report: {}", report_path.display());
    Ok(())
}

async fn run_load(
    worker_pools: usize,
    job_count: usize,
    history_rows: usize,
) -> Result<LoadReport> {
    let database = TestDatabase::create().await?;
    let result = run_scenario(&database, worker_pools, job_count, history_rows).await;
    let cleanup = database.finish().await;
    let report = result?;
    cleanup?;
    Ok(report)
}

async fn run_scenario(
    database: &TestDatabase,
    worker_pools: usize,
    job_count: usize,
    history_rows: usize,
) -> Result<LoadReport> {
    let probe_id = fixture::prepare(&database.connection, history_rows).await?;
    let foreground = fixture::connect(&database.url).await?;
    let mut baseline = Vec::new();
    for _ in 0..100 {
        baseline.push(fixture::foreground_transaction(&foreground, probe_id).await?);
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    let shutdown = CancellationToken::new();
    let probe_shutdown = CancellationToken::new();
    let epoch = Instant::now();
    let (handler_started, mut handler_starts) = mpsc::unbounded_channel();
    let _cancel_workers = shutdown.clone().drop_guard();
    let _cancel_probe = probe_shutdown.clone().drop_guard();
    let probe = tokio::spawn({
        let foreground = foreground.clone();
        let probe_shutdown = probe_shutdown.clone();
        async move {
            let mut latencies = Vec::new();
            while !probe_shutdown.is_cancelled() {
                latencies.push(fixture::foreground_transaction(&foreground, probe_id).await?);
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            Ok::<_, anyhow::Error>(latencies)
        }
    });

    let mut workers = JoinSet::new();
    let mut worker_connections = Vec::new();
    let settings = WorkerSettings {
        concurrency: 4,
        ..Default::default()
    };
    for _ in 0..worker_pools {
        let connection = fixture::connect(&database.url).await?;
        worker_connections.push(connection.clone());
        let handler_started = handler_started.clone();
        workers.spawn(jobs::run(
            PostgresQueue::new(connection.clone()),
            settings,
            shutdown.clone(),
            move |claim| {
                let connection = connection.clone();
                let handler_started = handler_started.clone();
                async move {
                    let result = async {
                        let wait = fixture::handle(&connection, &claim, epoch).await?;
                        handler_started
                            .send((claim.id, claim.kind, wait))
                            .context("handler measurement receiver closed")?;
                        Ok::<(), anyhow::Error>(())
                    }
                    .await;
                    result.map_err(|_| HandlerFailure {
                        code: "load_database_effect_failed",
                        retryable: false,
                    })
                }
            },
        ));
    }

    let started = Instant::now();
    let workload = tokio::time::timeout(Duration::from_secs(180), async {
        let mut producers = JoinSet::new();
        for producer in 0..4 {
            let connection = fixture::connect(&database.url).await?;
            producers.spawn(async move {
                let result = fixture::produce(&connection, job_count / 4, producer, epoch).await;
                connection.close().await?;
                result
            });
        }
        let mut registration = Vec::with_capacity(job_count);
        while let Some(produced) = producers.join_next().await {
            registration.extend(produced??);
        }
        loop {
            ensure!(!probe.is_finished(), "foreground database probe stopped");
            ensure!(
                workers.try_join_next().is_none(),
                "worker exited before draining the queue"
            );
            ensure!(
                job::Entity::find()
                    .filter(job::Column::State.eq(jobs::JobState::Failed))
                    .count(&database.connection)
                    .await?
                    == 0,
                "a loaded job failed"
            );
            let unfinished = job::Entity::find()
                .filter(
                    job::Column::State.is_in([jobs::JobState::Pending, jobs::JobState::Running]),
                )
                .count(&database.connection)
                .await?;
            if unfinished == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Ok::<_, anyhow::Error>(registration)
    })
    .await;
    let elapsed_seconds = started.elapsed().as_secs_f64();
    shutdown.cancel();
    probe_shutdown.cancel();
    let mut worker_error = None;
    while let Some(stopped) = workers.join_next().await {
        if let Err(error) = stopped
            .context("worker task failed")
            .and_then(|result| result.map_err(Into::into))
        {
            worker_error = Some(error);
        }
    }
    let foreground_samples = probe.await.context("foreground probe panicked")?;
    for connection in worker_connections {
        connection.close().await?;
    }
    foreground.close().await?;

    async {
        let foreground_samples = foreground_samples?;
        let registration = workload.context("job load exceeded 180 seconds")??;
        if let Some(error) = worker_error {
            return Err(error);
        }
        ensure!(registration.len() == job_count);
        let completed = job::Entity::find()
            .filter(job::Column::Kind.is_in([BULK_JOB, NOTIFICATION_JOB]))
            .all(&database.connection)
            .await?;
        let attempts = job_attempt::Entity::find()
            .all(&database.connection)
            .await?;
        ensure!(
            completed.len() == job_count && attempts.len() == job_count,
            "jobs were lost or retried unexpectedly"
        );
        ensure!(
            completed
                .iter()
                .all(|job| job.state == jobs::JobState::Succeeded && job.attempts == 1)
        );
        ensure!(
            attempts
                .iter()
                .all(|attempt| attempt.outcome == Some(jobs::AttemptOutcome::Succeeded))
        );
        let effects = record::Entity::find()
            .filter(record::Column::Id.ne(probe_id))
            .count(&database.connection)
            .await?;
        ensure!(effects == job_count as u64, "a business record was lost");
        ensure!(
            record::Entity::find()
                .filter(record::Column::Id.ne(probe_id))
                .filter(record::Column::Version.ne(1))
                .one(&database.connection)
                .await?
                .is_none(),
            "a job effect was lost or duplicated"
        );
        let jobs_by_id = completed
            .iter()
            .map(|job| (job.id, job))
            .collect::<BTreeMap<_, _>>();
        let mut bulk_wait = Vec::new();
        let mut notification_wait = Vec::new();
        let mut measured_jobs = std::collections::BTreeSet::new();
        while let Ok((job_id, kind, wait)) = handler_starts.try_recv() {
            let job = jobs_by_id
                .get(&job_id)
                .context("handler has no source job")?;
            ensure!(
                measured_jobs.insert(job_id) && kind == job.kind,
                "duplicate or mismatched handler measurement"
            );
            if job.kind == BULK_JOB {
                bulk_wait.push(wait);
            } else {
                notification_wait.push(wait);
            }
        }
        ensure!(
            measured_jobs.len() == job_count,
            "handler timing measurements missing"
        );
        ensure!(
            foreground_samples.len() >= 10,
            "too few foreground database samples"
        );
        Ok(LoadReport {
            worker_pools,
            concurrency_per_pool: settings.concurrency,
            producers: 4,
            retained_history_rows: history_rows,
            registered_jobs: job_count,
            succeeded_jobs: completed.len(),
            attempts: attempts.len(),
            verified_effects: effects,
            elapsed_seconds,
            scenario_jobs_per_second: job_count as f64 / elapsed_seconds,
            registration: Latency::from_samples(registration)?,
            bulk_registration_to_handler: Latency::from_samples(bulk_wait)?,
            notification_registration_to_handler: Latency::from_samples(notification_wait)?,
            foreground_baseline: Latency::from_samples(baseline)?,
            foreground_during_load: Latency::from_samples(foreground_samples)?,
        })
    }
    .await
}
