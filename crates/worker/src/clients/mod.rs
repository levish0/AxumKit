mod mail;
mod media_processor;
mod meilisearch;

pub use mail::create_mailer;
pub use media_processor::process_media;
pub use meilisearch::create_meili_client;
