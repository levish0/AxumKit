use crate::config::ObservabilitySettings;
use sentry::integrations::tracing::{ErrorLayer, SpanLayer};
use tracing_subscriber::{EnvFilter, filter::filter_fn, prelude::*};

pub fn initialize(settings: &ObservabilitySettings) -> anyhow::Result<sentry::ClientInitGuard> {
    let guard = sentry::init(client_options(settings)?);
    tracing_subscriber::registry()
        .with(EnvFilter::try_new(&settings.log_filter)?)
        .with(tracing_subscriber::fmt::layer().json())
        .with(sentry_layer())
        .try_init()
        .map_err(|error| anyhow::anyhow!("tracing initialization failed: {error}"))?;
    Ok(guard)
}

fn client_options(settings: &ObservabilitySettings) -> anyhow::Result<sentry::ClientOptions> {
    let mut options = sentry::ClientOptions::new()
        .send_default_pii(false)
        .traces_sample_rate(settings.sentry_sample_rate)
        .before_send(|mut event| {
            event.request = None;
            event.user = None;
            Some(event)
        });
    options.dsn = settings
        .sentry_dsn
        .as_ref()
        .map(|dsn| dsn.expose().parse())
        .transpose()
        .map_err(|_| anyhow::anyhow!("invalid SENTRY_DSN"))?;
    Ok(options)
}

fn sentry_layer<S>() -> impl tracing_subscriber::Layer<S>
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    SpanLayer::new()
        .with_filter(filter_fn(|metadata| {
            metadata.is_span() && application_target(metadata.target())
        }))
        .and_then(ErrorLayer::new().with_filter(filter_fn(|metadata| {
            metadata.is_event()
                && application_target(metadata.target())
                && *metadata.level() == tracing::Level::ERROR
        })))
}

fn application_target(target: &str) -> bool {
    ["kit", "application", "jobs", "mail"]
        .iter()
        .any(|prefix| target == *prefix || target.starts_with(&format!("{prefix}::")))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sentry_filters_external_events_removes_request_data_and_records_performance() {
        let options = client_options(&ObservabilitySettings {
            log_filter: "info".into(),
            sentry_dsn: None,
            sentry_sample_rate: 1.0,
        })
        .unwrap();
        let envelopes = sentry::test::with_captured_envelopes_options(
            || {
                let subscriber = tracing_subscriber::registry().with(sentry_layer());
                tracing::subscriber::with_default(subscriber, || {
                    let request = tracing::info_span!(target:"kit::http", "http_request", route="/api/v1/account", request_id="public-test-correlation");
                    let _entered = request.enter();
                    let worker = tracing::debug_span!(target:"jobs::worker", "poll_jobs");
                    let _worker = worker.enter();
                    let provider = tracing::info_span!(target:"reqwest", "provider_request", body="external-span-secret");
                    let _provider = provider.enter();
                    tracing::error!(target:"reqwest", body="external-secret", "provider error");
                    tracing::error!(target:"kitten::http", body="similar-target-secret", "unrelated error");
                    tracing::warn!(target:"kit::http", body="warning-secret", "request warning");
                    tracing::info!(target:"kit::http", body="info-secret", "request info");
                    tracing::debug!(target:"kit::http", body="debug-secret", "request debug");
                    tracing::trace!(target:"kit::http", body="trace-secret", "request trace");
                    tracing::error!(target:"kit::http", error_code="storage_unavailable", "request failed");
                    sentry::capture_event(sentry::protocol::Event {
                        message: Some("redaction check".into()),
                        request: Some(sentry::protocol::Request {
                            data: Some("request-secret".into()),
                            ..Default::default()
                        }),
                        user: Some(sentry::protocol::User {
                            email: Some("private@example.test".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    });
                });
            },
            options,
        );
        let mut captured = Vec::new();
        let mut events = 0;
        let mut transactions = 0;
        for envelope in &envelopes {
            envelope.to_writer(&mut captured).unwrap();
            for item in envelope.items() {
                match item {
                    sentry::protocol::EnvelopeItem::Event(_) => events += 1,
                    sentry::protocol::EnvelopeItem::Transaction(_) => transactions += 1,
                    _ => {}
                }
            }
        }
        let captured = String::from_utf8(captured).unwrap();
        assert_eq!(events, 2);
        assert!(transactions > 0);
        for secret in [
            "external-secret",
            "external-span-secret",
            "similar-target-secret",
            "warning-secret",
            "info-secret",
            "debug-secret",
            "trace-secret",
            "request-secret",
            "private@example.test",
        ] {
            assert!(!captured.contains(secret));
        }
        assert!(captured.contains("public-test-correlation"));
        assert!(captured.contains("request failed"));
        assert!(captured.contains("poll_jobs"));
    }
}
