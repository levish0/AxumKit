pub mod user {
    pub const USER_INVALID_PASSWORD: &str = "user:invalid_password";
    pub const USER_NOT_FOUND: &str = "user:not_found";
}

pub mod general {
    pub const BAD_REQUEST: &str = "general:bad_request";
    pub const VALIDATION_ERROR: &str = "general:validation_error";
}

pub mod system {
    pub const SYS_HASHING_ERROR: &str = "system:hashing_error";
    pub const SYS_NOT_FOUND: &str = "system:not_found";
    pub const SYS_DATABASE_ERROR: &str = "system:database_error";
}
