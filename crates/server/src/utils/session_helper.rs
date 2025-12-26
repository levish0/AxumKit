use crate::dto::auth::internal::session::SessionContext;
use crate::errors::errors::Errors;
use sea_orm::prelude::IpNetwork;
use uuid::Uuid;

/// SessionContext에서 user_id와 IP를 추출
///
/// # Returns
/// - 로그인 사용자: `(Some(user_id), Some(ip_network))`
/// - 익명 사용자: `(None, Some(ip_network))`
///
/// IP는 항상 기록됨 (다중 계정 탐지, 차단 회피 추적용)
///
/// # Errors
/// - `Errors::InvalidIpAddress` - IP 주소 파싱 실패 시
pub fn extract_user_or_ip(
    session: Option<&SessionContext>,
    ip_address: &str,
) -> Result<(Option<Uuid>, Option<IpNetwork>), Errors> {
    let ip = ip_address
        .parse::<IpNetwork>()
        .map_err(|_| Errors::InvalidIpAddress)?;

    match session {
        Some(s) => Ok((Some(s.user_id), Some(ip))),
        None => Ok((None, Some(ip))),
    }
}
