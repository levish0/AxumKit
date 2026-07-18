use crate::utils::r2_url::build_r2_public_url;
use dto::user::PublicUserProfile;
use entity::common::Role;
use entity::user_bans::Model as UserBanModel;
use entity::users::Model as UserModel;
use uuid::Uuid;

/// A user's masked public identity — the single masking point for user→response conversion.
///
/// A deactivated (soft-deleted) user still exposes `handle`/`display_name` (permanently reserved,
/// preserves attribution) but has `profile_image` masked and `deactivated` set to true.
pub struct MaskedUserIdentity {
    pub id: Uuid,
    pub handle: String,
    pub display_name: String,
    pub profile_image: Option<String>,
    pub deactivated: bool,
}

/// Maps a user Model to its masked public identity.
///
/// Every public user representation (actor / brief / profile) goes through this function so the
/// masking policy lives in one place.
pub fn mask_user_identity(user: &UserModel) -> MaskedUserIdentity {
    let deactivated = user.deleted_at.is_some();

    MaskedUserIdentity {
        id: user.id,
        handle: user.handle.clone(),
        display_name: user.display_name.clone(),
        profile_image: if deactivated {
            None
        } else {
            user.profile_image.as_deref().map(build_r2_public_url)
        },
        deactivated,
    }
}

/// Maps a user Model to the public profile DTO.
///
/// Reuses [`mask_user_identity`] for the shared identity and masks the profile-only fields
/// (bio/banner) for deactivated users as well. The optional active ban (`ban`) populates the
/// ban status fields — `None` leaves the profile marked as not banned.
pub fn user_to_public_profile(
    user: UserModel,
    roles: Vec<Role>,
    ban: Option<UserBanModel>,
) -> PublicUserProfile {
    let identity = mask_user_identity(&user);

    PublicUserProfile {
        id: identity.id,
        handle: identity.handle,
        display_name: identity.display_name,
        bio: if identity.deactivated { None } else { user.bio },
        profile_image: identity.profile_image,
        banner_image: if identity.deactivated {
            None
        } else {
            user.banner_image.as_deref().map(build_r2_public_url)
        },
        roles,
        deactivated: identity.deactivated,
        is_banned: ban.is_some(),
        banned_until: ban.as_ref().and_then(|b| b.expires_at),
        ban_reason: ban.and_then(|b| b.reason),
        created_at: user.created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // Images are only attached when needed; the active-user cases keep them off so the masking
    // tests never reach `build_r2_public_url` (which requires a global ServerConfig).
    fn make_user(deleted: bool, with_image: bool) -> UserModel {
        let now = Utc::now();
        UserModel {
            id: Uuid::new_v4(),
            display_name: "Alice".to_string(),
            handle: "alice".to_string(),
            bio: Some("hi".to_string()),
            email: "alice@example.com".to_string(),
            password: None,
            profile_image: with_image.then(|| "avatar-key".to_string()),
            banner_image: with_image.then(|| "banner-key".to_string()),
            totp_secret: None,
            totp_enabled_at: None,
            totp_backup_codes: None,
            created_at: now,
            deleted_at: deleted.then_some(now),
        }
    }

    #[test]
    fn active_user_keeps_identity() {
        let identity = mask_user_identity(&make_user(false, false));
        assert!(!identity.deactivated);
        assert_eq!(identity.handle, "alice");
        assert_eq!(identity.display_name, "Alice");
        assert_eq!(identity.profile_image, None);
    }

    #[test]
    fn deactivated_user_keeps_names_but_masks_image() {
        let identity = mask_user_identity(&make_user(true, true));
        assert!(identity.deactivated);
        assert_eq!(identity.handle, "alice");
        assert_eq!(identity.display_name, "Alice");
        assert_eq!(identity.profile_image, None);
    }

    #[test]
    fn public_profile_masks_bio_and_media_when_deactivated() {
        let profile = user_to_public_profile(make_user(true, true), Vec::new(), None);
        assert!(profile.deactivated);
        assert_eq!(profile.handle, "alice");
        assert_eq!(profile.display_name, "Alice");
        assert_eq!(profile.bio, None);
        assert_eq!(profile.profile_image, None);
        assert_eq!(profile.banner_image, None);
    }

    #[test]
    fn public_profile_keeps_bio_when_active() {
        let profile = user_to_public_profile(make_user(false, false), Vec::new(), None);
        assert!(!profile.deactivated);
        assert_eq!(profile.bio.as_deref(), Some("hi"));
    }
}
