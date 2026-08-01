//! Integration tests for the worker's generic JetStream consumer, driven against
//! the test stack's real NATS. Unlike the rest of the e2e crate these are not HTTP
//! black-box tests: the redelivery timing they pin down cannot be reproduced
//! through the API (the stack's mailpit answers in milliseconds, and the bug needs
//! a handler that outlives the first backoff step), so they drive `NatsConsumer`
//! directly against a disposable stream.
//!
//! Regression context (duplicate transactional emails): setting `backoff` on the
//! JetStream consumer config makes the server use the backoff schedule as the ack
//! timeout, overriding `ack_wait` — so with a 1s first step, any handler slower
//! than 1s was redelivered (and re-run concurrently) while the original delivery
//! was still in flight. Retry backoff must therefore ride on explicit Nak delays,
//! never on the consumer config.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_nats::jetstream;
use serde::{Deserialize, Serialize};
use worker::nats::consumer::NatsConsumer;

#[derive(Serialize, Deserialize)]
struct TestJob;

fn nats_url() -> String {
    std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:54222".to_string())
}

/// Creates a disposable WorkQueue stream (mirroring the production job streams)
/// with a unique name so parallel tests never collide.
async fn disposable_stream() -> (Arc<jetstream::Context>, String, String) {
    let client = async_nats::connect(nats_url())
        .await
        .expect("connect to test NATS (is docker-compose.test.yml up?)");
    let js = Arc::new(jetstream::new(client));
    let id = uuid::Uuid::new_v4().simple().to_string();
    let stream = format!("e2e_consumer_{id}");
    let subject = format!("e2e.consumer.{id}");
    js.create_stream(jetstream::stream::Config {
        name: stream.clone(),
        subjects: vec![subject.clone()],
        retention: jetstream::stream::RetentionPolicy::WorkQueue,
        ..Default::default()
    })
    .await
    .expect("create disposable stream");
    (js, stream, subject)
}

async fn publish_one(js: &jetstream::Context, subject: &str) {
    js.publish(
        subject.to_string(),
        serde_json::to_vec(&TestJob).unwrap().into(),
    )
    .await
    .expect("publish test job")
    .await
    .expect("await publish ack");
}

#[tokio::test]
async fn slow_successful_handler_runs_exactly_once() {
    let (js, stream, subject) = disposable_stream().await;
    let runs = Arc::new(AtomicUsize::new(0));

    let handler_runs = runs.clone();
    // Concurrency 2 mirrors the email consumer: max_ack_pending 2 leaves room for
    // a premature redelivery to be delivered alongside the in-flight original.
    let consumer = NatsConsumer::new(js.clone(), &stream, "slow", 2);
    let task = tokio::spawn(consumer.run::<TestJob, _, _>(move |_job| {
        let runs = handler_runs.clone();
        async move {
            runs.fetch_add(1, Ordering::SeqCst);
            // Longer than the first retry-backoff step (1s). Under the buggy
            // config (backoff on the consumer, overriding ack_wait) JetStream
            // redelivered at +1s/+3s/+7s — all inside this handler's lifetime.
            tokio::time::sleep(Duration::from_secs(3)).await;
            Ok(())
        }
    }));

    publish_one(&js, &subject).await;

    // Wait out the handler (3s) plus the window in which every would-be
    // premature redelivery (1s/2s/4s steps) would land and be processed.
    tokio::time::sleep(Duration::from_secs(9)).await;

    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "a slow-but-successful handler must run exactly once (no redelivery while in flight)"
    );

    task.abort();
    js.delete_stream(&stream).await.expect("cleanup stream");
}

#[tokio::test]
async fn failed_handler_is_redelivered_then_succeeds() {
    let (js, stream, subject) = disposable_stream().await;
    let runs = Arc::new(AtomicUsize::new(0));

    let handler_runs = runs.clone();
    let consumer = NatsConsumer::new(js.clone(), &stream, "retry", 2);
    let task = tokio::spawn(consumer.run::<TestJob, _, _>(move |_job| {
        let runs = handler_runs.clone();
        async move {
            if runs.fetch_add(1, Ordering::SeqCst) == 0 {
                anyhow::bail!("first delivery fails");
            }
            Ok(())
        }
    }));

    publish_one(&js, &subject).await;

    // The first Nak delay is 1s; give the redelivery ample room.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while runs.load(Ordering::SeqCst) < 2 && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    assert_eq!(
        runs.load(Ordering::SeqCst),
        2,
        "a failed delivery must be redelivered (retry backoff still works without consumer-config backoff)"
    );

    task.abort();
    js.delete_stream(&stream).await.expect("cleanup stream");
}
