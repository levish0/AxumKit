use validator::ValidationError;

pub fn email(value: &str) -> Result<(), ValidationError> {
    application::identity::validation::normalize_email(value)
        .map(|_| ())
        .map_err(|_| ValidationError::new("email"))
}

pub fn new_password(value: &str) -> Result<(), ValidationError> {
    application::identity::validation::validate_new_password(value)
        .map_err(|_| ValidationError::new("password_length"))
}

pub fn organization_name(value: &str) -> Result<(), ValidationError> {
    application::organizations::validation::normalize_name(value)
        .map(|_| ())
        .map_err(|_| ValidationError::new("organization_name"))
}

pub fn filename(value: &str) -> Result<(), ValidationError> {
    application::files::validation::validate_filename(value)
        .map_err(|_| ValidationError::new("filename"))
}

pub fn media_type(value: &str) -> Result<(), ValidationError> {
    application::files::validation::validate_media_type(value)
        .map_err(|_| ValidationError::new("media_type"))
}

pub fn api_access_name(value: &str) -> Result<(), ValidationError> {
    application::api_access::validation::name(value)
        .map(|_| ())
        .map_err(|_| ValidationError::new("api_name"))
}

pub fn post_title(value: &str) -> Result<(), ValidationError> {
    application::posts::validation::title(value).map_err(|_| ValidationError::new("post_title"))
}
pub fn post_body(value: &str) -> Result<(), ValidationError> {
    application::posts::validation::body(value).map_err(|_| ValidationError::new("post_body"))
}
pub fn comment_body(value: &str) -> Result<(), ValidationError> {
    application::posts::validation::comment(value).map_err(|_| ValidationError::new("comment_body"))
}
pub fn post_attachments(values: &[uuid::Uuid]) -> Result<(), ValidationError> {
    application::posts::validation::attachments(values)
        .map_err(|_| ValidationError::new("post_attachments"))
}
pub fn display_name(value: &str) -> Result<(), ValidationError> {
    application::profiles::validation::display_name(value)
        .map_err(|_| ValidationError::new("display_name"))
}
pub fn profile_bio(value: &str) -> Result<(), ValidationError> {
    application::profiles::validation::bio(value).map_err(|_| ValidationError::new("profile_bio"))
}
pub fn search_query(value: &str) -> Result<(), ValidationError> {
    application::search::validate_query(value).map_err(|_| ValidationError::new("search_query"))
}
pub fn push_token(value: &str) -> Result<(), ValidationError> {
    application::notifications::validation::device_token(value)
        .map_err(|_| ValidationError::new("push_token"))
}
