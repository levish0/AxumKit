mod image_processor;
mod mail;
mod meilisearch;

pub use image_processor::process_image;
pub use mail::create_mailer;
pub use meilisearch::create_meili_client;
