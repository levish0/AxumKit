//! R2 storage key prefixes and image size limits

/// Prefix for post content in SeaweedFS
pub const POST_CONTENT_PREFIX: &str = "posts";

/// Prefix for user profile/banner images
pub const USER_IMAGES_PREFIX: &str = "user-images";

/// Maximum size for profile images (4MB)
pub const PROFILE_IMAGE_MAX_SIZE: usize = 4 * 1024 * 1024;

/// Maximum size for banner images (8MB)
pub const BANNER_IMAGE_MAX_SIZE: usize = 8 * 1024 * 1024;

/// Generate storage key for user images (profile/banner)
pub fn user_image_key(hash: &str, extension: &str) -> String {
    format!("{}/{}.{}", USER_IMAGES_PREFIX, hash, extension)
}
