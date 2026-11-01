use super::{api_access, files, identity, notifications, organizations, posts, profiles};
use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
};

#[derive(OpenApi)]
#[openapi(paths(
    identity::signup::signup, identity::request_verification::request_verification, identity::verify_email::verify_email, identity::login::login, identity::complete_mfa::complete_mfa,
    identity::request_reset::request_reset, identity::reset_password::reset_password, identity::get_account::get_account, identity::deactivate_account::deactivate_account, identity::change_password::change_password, identity::set_initial_password::set_initial_password,
    identity::list_sessions::list_sessions, identity::revoke_session::revoke_session, identity::revoke_all_sessions::revoke_all_sessions, identity::rotate_session::rotate_session,
    identity::begin_totp::begin_totp, identity::confirm_totp::confirm_totp, identity::disable_totp::disable_totp, identity::regenerate_recovery_codes::regenerate_recovery_codes,
    organizations::create_organization::create_organization, organizations::get_organization::get_organization, organizations::list_organizations::list_organizations, organizations::rename_organization::rename_organization, organizations::add_member::add_member, organizations::remove_member::remove_member,
    identity::oidc::begin_login::begin_login, identity::oidc::begin_link::begin_link, identity::oidc::callback::callback, identity::oidc::begin_native_login::begin_native_login, identity::oidc::complete_native_login::complete_native_login, identity::oidc::list_identities::list_identities, identity::oidc::unlink_identity::unlink_identity,
    files::reserve_file::reserve_file, files::list_files::list_files, files::get_file::get_file, files::create_upload_url::create_upload_url, files::delete_file::delete_file,
    files::search_files::search_files, files::create_download_url::create_download_url, files::complete_upload::complete_upload,
    api_access::create_client::create_client, api_access::list_clients::list_clients, api_access::get_client::get_client,
    api_access::update_client::update_client, api_access::revoke_client::revoke_client, api_access::issue_key::issue_key,
    api_access::list_keys::list_keys, api_access::revoke_key::revoke_key, api_access::external_organization::external_organization,
    posts::create_post::create_post,posts::update_post::update_post,posts::get_post::get_post,posts::list_posts::list_posts,posts::delete_post::delete_post,posts::search_posts::search_posts,
    posts::comments::create_comment::create_comment,posts::comments::list_comments::list_comments,posts::comments::delete_comment::delete_comment,api_access::external_posts::external_posts,
    profiles::get_own_profile::get_own_profile,profiles::save_profile::save_profile,profiles::list_users::list_users,profiles::get_user::get_user,profiles::search_users::search_users,
    notifications::list_notifications::list_notifications,notifications::get_notification::get_notification,notifications::unread_count::unread_count,
    notifications::mark_read::mark_read,notifications::mark_read_batch::mark_read_batch,notifications::get_preferences::get_preferences,notifications::save_preferences::save_preferences,
    notifications::register_device::register_device,notifications::list_devices::list_devices,notifications::remove_device::remove_device
), components(schemas(crate::http::error::ErrorOutput)), modifiers(&SecuritySchemes))]
struct ApiDocumentation;

struct SecuritySchemes;
impl Modify for SecuritySchemes {
    fn modify(&self, document: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = document.components.as_mut() {
            components.add_security_scheme(
                "apiKeyAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("API key: hak_v1_<uuid>.<secret>; external routes only")
                        .build(),
                ),
            );
            components.add_security_scheme("cookieAuth", SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description("__Host-session", "Development uses session; production uses __Host-session. Unsafe cookie requests require an allowed Origin."))));
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("Opaque session token")
                        .build(),
                ),
            );
        }
    }
}

pub fn document() -> utoipa::openapi::OpenApi {
    use utoipa::openapi::{Content, Ref, response::ResponseBuilder};
    let mut document = ApiDocumentation::openapi();
    for path in document.paths.paths.values_mut() {
        for operation in [
            &mut path.get,
            &mut path.post,
            &mut path.put,
            &mut path.delete,
            &mut path.patch,
            &mut path.head,
        ]
        .into_iter()
        .flatten()
        {
            for (status, description) in [
                ("400", "Invalid input"),
                ("401", "Authentication required"),
                ("403", "Access denied"),
                ("404", "Resource not found"),
                ("408", "Request deadline elapsed"),
                ("409", "Conflicting state"),
                ("413", "Body too large"),
                ("429", "Too many attempts"),
                ("500", "Internal error"),
                ("503", "Service unavailable"),
            ] {
                operation
                    .responses
                    .responses
                    .entry(status.into())
                    .or_insert_with(|| {
                        ResponseBuilder::new()
                            .description(description)
                            .content(
                                "application/json",
                                Content::new(Some(Ref::from_schema_name("ErrorOutput"))),
                            )
                            .build()
                            .into()
                    });
            }
        }
    }
    document
}
