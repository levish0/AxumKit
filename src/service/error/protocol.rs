pub mod user {
    pub const EMAIL_EXISTS: &str = "user:email_exists";
    pub const USERNAME_EXISTS: &str = "user:user_name_exists";
}

pub mod general {
    pub const BAD_REQUEST: &str = "general:bad_request";
    pub const VALIDATION_ERROR: &str = "general:validation_error";
}

pub mod system {
    pub const INTERNAL_ERROR: &str = "system:internal_error";
    pub const NOT_FOUND: &str = "system:not_found";
}
