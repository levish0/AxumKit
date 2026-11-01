use kit::{bootstrap::bootstrap, config::Settings, observability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::from_environment()?;
    let _telemetry = observability::initialize(&settings.observability)?;
    let app = bootstrap(&settings).await?;
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let result = match arguments.as_slice() {
        [command] if command == "cleanup" => {
            kit::maintenance::cleanup(&app, &settings.retention).await?;
            Ok(())
        }
        [command] if command == "initialize-integrations" => {
            kit::integrations::initialize(&app).await?;
            Ok(())
        }
        [command] if command == "repair-search" => {
            kit::integrations::repair_search(&app).await?;
            Ok(())
        }
        [command, account_id, enabled] if command == "set-administrator" => {
            let account_id = uuid::Uuid::parse_str(account_id)?;
            let administrator = enabled.parse::<bool>()?;
            app.identity
                .set_administrator_by_operator(
                    account_id,
                    administrator,
                    jobs::TraceContext::default(),
                )
                .await?;
            tracing::info!(%account_id, administrator, "administrator access updated");
            Ok(())
        }
        [command, job_id] if command == "retry-job" => {
            let job_id = uuid::Uuid::parse_str(job_id)?;
            anyhow::ensure!(
                app.queue.retry_failed(job_id).await?,
                "job is not eligible for retry"
            );
            tracing::info!(%job_id, "failed job scheduled again");
            Ok(())
        }
        [command, client_id, minute_limit, month_limit] if command == "set-api-client-quota" => {
            app.api_access
                .set_quota_by_operator(
                    client_id.parse()?,
                    minute_limit.parse()?,
                    month_limit.parse()?,
                    jobs::TraceContext::default(),
                )
                .await?;
            tracing::info!(client_id, "API client allocation updated");
            Ok(())
        }
        _ => Err(anyhow::anyhow!(
            "usage: manage cleanup | initialize-integrations | repair-search | retry-job <uuid> | set-administrator <uuid> <true|false> | set-api-client-quota <uuid> <per-minute> <per-month>"
        )),
    };
    app.close().await?;
    result
}
