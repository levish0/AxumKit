use url::Url;

/// Render a database URL for logging with the credentials stripped.
///
/// Connection URLs carry the password in their userinfo, so they must never reach a
/// log sink verbatim. Masking the password with a run of `*` still leaks its length,
/// so this drops the userinfo entirely and keeps only what is useful when debugging a
/// deployment: which host and which database we are pointed at.
///
/// An unparseable URL yields `"<unparseable database url>"` rather than the input —
/// falling back to the raw string would defeat the point on exactly the malformed
/// input most likely to be logged during a misconfiguration.
pub fn redact_database_url(database_url: &str) -> String {
    let Ok(url) = Url::parse(database_url) else {
        return "<unparseable database url>".to_string();
    };

    let host = url.host_str().unwrap_or("<no host>");
    let database = url.path().trim_start_matches('/');
    match url.port() {
        Some(port) => format!("{host}:{port}/{database}"),
        None => format!("{host}/{database}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_credentials_and_keeps_host_and_database() {
        assert_eq!(
            redact_database_url("postgres://user:hunter2@db.example.com:6432/axumkit"),
            "db.example.com:6432/axumkit"
        );
    }

    #[test]
    fn omits_the_port_when_the_url_does_not_carry_one() {
        // Managed endpoints (Neon) are addressed without an explicit port.
        assert_eq!(
            redact_database_url("postgresql://u:p@ep-x.eu-central-1.aws.neon.tech/axumkit"),
            "ep-x.eu-central-1.aws.neon.tech/axumkit"
        );
    }

    #[test]
    fn keeps_the_query_string_out_of_the_rendered_form() {
        // `?sslmode=require&options=...` is noise in a log line and can carry endpoint ids.
        assert_eq!(
            redact_database_url("postgres://u:p@host/db?sslmode=require"),
            "host/db"
        );
    }

    #[test]
    fn never_echoes_an_unparseable_url() {
        let redacted = redact_database_url("not a url: hunter2");
        assert_eq!(redacted, "<unparseable database url>");
        assert!(!redacted.contains("hunter2"));
    }
}
