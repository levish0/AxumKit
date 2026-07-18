use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

/// Notification action: the concrete reason a notification was produced.
///
/// Stored as TEXT in `notification_events.action` (not a Postgres enum) so
/// the action set can grow without an enum migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum NotificationAction {
    // ==================== Board Actions ====================
    /// A comment was posted on one of your posts
    #[serde(rename = "board_comment_created")]
    BoardCommentCreated,

    // ==================== User Actions ====================
    /// You were mentioned
    #[serde(rename = "user_mentioned")]
    UserMentioned,

    // ==================== System Actions ====================
    /// System announcement
    #[serde(rename = "system_announcement")]
    SystemAnnouncement,
}

impl NotificationAction {
    /// Returns the canonical string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationAction::BoardCommentCreated => "board_comment_created",
            NotificationAction::UserMentioned => "user_mentioned",
            NotificationAction::SystemAnnouncement => "system_announcement",
        }
    }

    /// Every defined action — what the preferences API can enumerate.
    pub fn all() -> &'static [NotificationAction] {
        &[
            NotificationAction::BoardCommentCreated,
            NotificationAction::UserMentioned,
            NotificationAction::SystemAnnouncement,
        ]
    }
}

impl fmt::Display for NotificationAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for NotificationAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // Board
            "board_comment_created" => Ok(NotificationAction::BoardCommentCreated),
            // User
            "user_mentioned" => Ok(NotificationAction::UserMentioned),
            // System
            "system_announcement" => Ok(NotificationAction::SystemAnnouncement),
            _ => Err(format!("Unknown notification action: {}", s)),
        }
    }
}

/// Convert NotificationAction to String for DB storage.
pub fn notification_action_to_string(action: NotificationAction) -> String {
    action.as_str().to_string()
}

/// Convert String from DB to NotificationAction.
pub fn string_to_notification_action(s: &str) -> Option<NotificationAction> {
    s.parse().ok()
}
