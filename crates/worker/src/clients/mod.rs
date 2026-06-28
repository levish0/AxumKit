mod media_processor;
mod mail;
mod meilisearch;

pub use media_processor::process_media;
pub use mail::create_mailer;
pub use meilisearch::create_meili_client;
