#[path = "target_kind.rs"]
mod notification_target_kind;
#[path = "type.rs"]
mod notification_type;

pub use notification_target_kind::NotificationTargetKind;
pub use notification_type::NotificationType;
