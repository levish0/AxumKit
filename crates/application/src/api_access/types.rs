use super::entities::{api_client, api_key};
use crate::{ApplicationError, ApplicationResult, api_access::ApiScope, config::SecretString};
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use uuid::Uuid;

pub struct ApiClientSummary {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub scopes: Vec<ApiScope>,
    pub active: bool,
    pub quota: QuotaStatus,
}
pub struct ApiKeySummary {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}
impl From<api_key::Model> for ApiKeySummary {
    fn from(value: api_key::Model) -> Self {
        Self {
            id: value.id,
            name: value.name,
            created_at: value.created_at,
            expires_at: value.expires_at,
            revoked_at: value.revoked_at,
        }
    }
}
pub struct IssuedApiKey {
    pub key: ApiKeySummary,
    pub credential: SecretString,
}
pub struct ExternalOrganization {
    pub id: Uuid,
    pub name: String,
    pub version: i64,
    pub quota: QuotaStatus,
}
pub struct ApiAdmission {
    pub(super) organization: crate::organizations::OrganizationSummary,
    pub(super) client_id: Uuid,
    pub(super) scope: ApiScope,
    pub(super) quota: QuotaStatus,
}
impl ApiAdmission {
    pub fn organization_id(&self) -> Uuid {
        self.organization.id
    }
    pub(crate) fn organization_owner_id(&self) -> Uuid {
        self.organization.owner_id
    }
    pub fn client_id(&self) -> Uuid {
        self.client_id
    }
    pub fn scope(&self) -> ApiScope {
        self.scope
    }
    pub fn quota(&self) -> &QuotaStatus {
        &self.quota
    }
}
pub struct QuotaStatus {
    pub minute_limit: i64,
    pub minute_used: i64,
    pub minute_resets_at: DateTime<Utc>,
    pub month_limit: i64,
    pub month_used: i64,
    pub month_resets_at: DateTime<Utc>,
}
pub(super) struct QuotaPeriods {
    pub minute: DateTime<Utc>,
    pub next_minute: DateTime<Utc>,
    pub month: DateTime<Utc>,
    pub next_month: DateTime<Utc>,
}
impl QuotaPeriods {
    pub fn at(now: DateTime<Utc>) -> Self {
        let minute = now.with_second(0).unwrap().with_nanosecond(0).unwrap();
        let month = minute
            .with_day(1)
            .unwrap()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap();
        let (year, month_number) = if now.month() == 12 {
            (now.year() + 1, 1)
        } else {
            (now.year(), now.month() + 1)
        };
        Self {
            minute,
            next_minute: minute + Duration::minutes(1),
            month,
            next_month: month
                .with_year(year)
                .unwrap()
                .with_month(month_number)
                .unwrap(),
        }
    }
}
pub(super) fn quota_status(client: &api_client::Model, now: DateTime<Utc>) -> QuotaStatus {
    let periods = QuotaPeriods::at(now);
    QuotaStatus {
        minute_limit: client.minute_limit,
        minute_used: if client.minute_started_at == Some(periods.minute) {
            client.minute_used
        } else {
            0
        },
        minute_resets_at: periods.next_minute,
        month_limit: client.month_limit,
        month_used: if client.month_started_at == Some(periods.month) {
            client.month_used
        } else {
            0
        },
        month_resets_at: periods.next_month,
    }
}
pub(super) fn client_summary(
    client: api_client::Model,
    now: DateTime<Utc>,
) -> ApplicationResult<ApiClientSummary> {
    let quota = quota_status(&client, now);
    Ok(ApiClientSummary {
        id: client.id,
        organization_id: client.organization_id,
        name: client.name,
        scopes: serde_json::from_value(client.scopes).map_err(ApplicationError::internal)?,
        active: client.active,
        quota,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utc_calendar_periods_cover_leap_days_and_year_rollover() {
        for (date, next) in [
            ("2028-02-29T23:59:59.999Z", "2028-03-01T00:00:00Z"),
            ("2026-12-31T23:59:59Z", "2027-01-01T00:00:00Z"),
        ] {
            let now: DateTime<Utc> = date.parse().unwrap();
            let periods = QuotaPeriods::at(now);
            assert_eq!(periods.next_month, next.parse::<DateTime<Utc>>().unwrap());
            assert_eq!(periods.next_minute, periods.next_month);
            assert!(periods.month <= now && periods.minute <= now);
        }
    }
}
