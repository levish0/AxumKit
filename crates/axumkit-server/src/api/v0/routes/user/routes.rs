use super::check_handle_available::check_handle_available;
use super::create_user::create_user;
use super::delete_banner_image::delete_banner_image;
use super::delete_profile_image::delete_profile_image;
use super::get_my_profile::get_my_profile;
use super::get_user_profile::get_user_profile;
use super::get_user_profile_by_id::get_user_profile_by_id;
use super::update_my_profile::update_my_profile;
use super::upload_banner_image::upload_banner_image;
use super::upload_profile_image::upload_profile_image;
use crate::state::AppState;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use axumkit_constants::{BANNER_IMAGE_MAX_SIZE, PROFILE_IMAGE_MAX_SIZE};

pub fn user_routes() -> Router<AppState> {
    // Profile image upload route with 4MB limit
    let profile_image_route = Router::new()
        .route(
            "/user/me/profile-image",
            post(upload_profile_image).delete(delete_profile_image),
        )
        .layer(DefaultBodyLimit::max(PROFILE_IMAGE_MAX_SIZE));

    // Banner image upload route with 8MB limit
    let banner_image_route = Router::new()
        .route(
            "/user/me/banner-image",
            post(upload_banner_image).delete(delete_banner_image),
        )
        .layer(DefaultBodyLimit::max(BANNER_IMAGE_MAX_SIZE));

    // Protected routes (authentication via extractors)
    let protected_routes =
        Router::new().route("/user/me", get(get_my_profile).patch(update_my_profile));

    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/users", post(create_user))
        .route("/users/profile", get(get_user_profile))
        .route("/users/profile/id", get(get_user_profile_by_id))
        .route(
            "/users/handle/{handle}/available",
            get(check_handle_available),
        );

    // Merge all routes
    profile_image_route
        .merge(banner_image_route)
        .merge(protected_routes)
        .merge(public_routes)
}
