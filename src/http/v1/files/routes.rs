use super::{
    complete_upload, create_download_url, create_upload_url, delete_file, get_file, list_files,
    reserve_file, search_files,
};
use crate::http::HttpState;

pub(in crate::http::v1) fn routes() -> axum::Router<HttpState> {
    axum::Router::new()
        .route(
            "/organizations/{organization_id}/files",
            axum::routing::get(list_files::list_files).post(reserve_file::reserve_file),
        )
        .route(
            "/organizations/{organization_id}/files/search",
            axum::routing::get(search_files::search_files),
        )
        .route(
            "/organizations/{organization_id}/files/{file_id}",
            axum::routing::get(get_file::get_file).delete(delete_file::delete_file),
        )
        .route(
            "/organizations/{organization_id}/files/{file_id}/upload-url",
            axum::routing::post(create_upload_url::create_upload_url),
        )
        .route(
            "/organizations/{organization_id}/files/{file_id}/complete-upload",
            axum::routing::post(complete_upload::complete_upload),
        )
        .route(
            "/organizations/{organization_id}/files/{file_id}/download-url",
            axum::routing::post(create_download_url::create_download_url),
        )
}
