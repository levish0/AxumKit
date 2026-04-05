use chrono::{DateTime, Utc};
use validator::ValidationError;

/// Validates that expires_at is in the future (rejects past datetimes).
/// When used on an Option field, the validator crate only calls this for Some values.
pub fn validate_future_datetime(dt: &DateTime<Utc>) -> Result<(), ValidationError> {
    if *dt <= Utc::now() {
        return Err(ValidationError::new("expires_at_must_be_in_future"));
    }
    Ok(())
}
