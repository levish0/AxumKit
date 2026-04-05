use chrono::{DateTime, Utc};
use validator::ValidationError;

/// expires_at이 미래 시간인지 검증 (과거 시간 입력 방지)
/// Option 필드에 사용 시 validator crate가 Some일 때만 호출
pub fn validate_future_datetime(dt: &DateTime<Utc>) -> Result<(), ValidationError> {
    if *dt <= Utc::now() {
        return Err(ValidationError::new("expires_at_must_be_in_future"));
    }
    Ok(())
}
