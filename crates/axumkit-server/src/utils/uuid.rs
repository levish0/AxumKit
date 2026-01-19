use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Extract timestamp from UUIDv7 using uuid crate's built-in method.
/// Returns current time if extraction fails.
pub fn uuid7_to_datetime(uuid: Uuid) -> DateTime<Utc> {
    uuid.get_timestamp()
        .and_then(|ts| {
            let (secs, nanos) = ts.to_unix();
            DateTime::from_timestamp(secs as i64, nanos)
        })
        .unwrap_or_else(Utc::now)
}
