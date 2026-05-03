use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../client/"]
pub struct ClientAssets;

pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();
        match ClientAssets::get(path.as_str()) {
            Some(content) => {
                let mime = mime_guess::from_path(&path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            }
            None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        }
    }
}

/// Serve a static file by path segment with path traversal protection (T-04-01).
pub async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    if path.contains("..") || path.starts_with('/') {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }
    StaticFile(path).into_response()
}

/// Serve index.html at the root route.
pub async fn index_handler() -> impl IntoResponse {
    StaticFile("index.html".to_string())
}
