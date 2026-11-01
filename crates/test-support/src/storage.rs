//! Direct requests are confined to the disposable loopback storage fixture.

use anyhow::{Result, ensure};
use reqwest::{Client, Method, RequestBuilder, redirect::Policy};
use std::{net::SocketAddr, time::Duration};
use url::Url;

fn checked_url(endpoint: &str, target: &str) -> Result<Url> {
    let endpoint =
        Url::parse(endpoint).map_err(|_| anyhow::anyhow!("invalid local storage endpoint"))?;
    let target =
        Url::parse(target).map_err(|_| anyhow::anyhow!("invalid local storage request URL"))?;
    ensure!(
        matches!(endpoint.scheme(), "http" | "https")
            && matches!(
                endpoint.host_str(),
                Some("localhost" | "127.0.0.1" | "[::1]")
            )
            && endpoint.username().is_empty()
            && endpoint.password().is_none()
            && endpoint.query().is_none()
            && endpoint.fragment().is_none()
            && target.origin() == endpoint.origin()
            && target.username().is_empty()
            && target.password().is_none()
            && target.fragment().is_none(),
        "storage requests must use the configured loopback origin"
    );
    Ok(target)
}

fn client(target: &Url) -> Result<Client> {
    let mut builder = Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .referer(false)
        .timeout(Duration::from_secs(15));
    if target.host_str() == Some("localhost") {
        // Keep localhost on loopback even if the machine's resolver is overridden.
        let port = target.port_or_known_default().unwrap();
        builder = builder.resolve("localhost", SocketAddr::from(([127, 0, 0, 1], port)));
    }
    builder
        .build()
        .map_err(|_| anyhow::anyhow!("cannot build local storage client"))
}

pub fn request(method: Method, target: &str) -> Result<RequestBuilder> {
    let endpoint = std::env::var("TEST_S3_ENDPOINT")?;
    let target = checked_url(&endpoint, target)?;
    Ok(client(&target)?.request(method, target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };

    #[test]
    fn rejects_changed_origin_and_redacts_rejected_capabilities() {
        let endpoint = "http://localhost:8339/";
        for target in [
            "http://example.test/object?signature=private",
            "http://localhost:8340/object?signature=private",
            "https://localhost:8339/object?signature=private",
            "http://user:private@localhost:8339/object",
            "http://localhost:8339/object#private",
        ] {
            let error = checked_url(endpoint, target).unwrap_err().to_string();
            assert!(!error.contains("private"));
        }
        assert!(checked_url("http://example.test/", "http://example.test/object").is_err());
        assert!(checked_url(endpoint, "http://localhost:8339/object?signature=private").is_ok());
    }

    #[tokio::test]
    async fn does_not_follow_storage_redirects() -> Result<()> {
        let first = TcpListener::bind(("127.0.0.1", 0)).await?;
        let second = TcpListener::bind(("127.0.0.1", 0)).await?;
        let target: Url = format!(
            "http://localhost:{}/object?signature=test-only",
            first.local_addr()?.port()
        )
        .parse()?;
        let destination = second.local_addr()?;
        let server = tokio::spawn(async move {
            let (mut socket, _) = first.accept().await?;
            let mut reader = BufReader::new(&mut socket);
            let mut line = String::new();
            loop {
                ensure!(
                    reader.read_line(&mut line).await? > 0,
                    "incomplete local request"
                );
                if line == "\r\n" {
                    break;
                }
                line.clear();
            }
            socket.write_all(format!("HTTP/1.1 302 Found\r\nLocation: http://{destination}/object\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await?;
            anyhow::Ok(())
        });
        let response = client(&target)?
            .get(target)
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("local redirect fixture request failed"))?;
        assert_eq!(response.status(), 302);
        server.await??;
        assert!(
            tokio::time::timeout(Duration::from_millis(100), second.accept())
                .await
                .is_err()
        );
        Ok(())
    }
}
