use axum::{
    extract::Path,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web-dist"]
struct WebAssets;

fn build_asset_response(path: &str) -> Option<Response> {
    let asset = WebAssets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let mut response = Response::new(axum::body::Body::from(asset.data.into_owned()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    Some(response)
}

pub async fn index() -> impl IntoResponse {
    build_asset_response("index.html")
        .unwrap_or_else(|| (StatusCode::NOT_FOUND, "Frontend bundle not found").into_response())
}

pub async fn assets(Path(path): Path<String>) -> impl IntoResponse {
    if path.starts_with("api/") {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    }

    if let Some(response) = build_asset_response(&path) {
        return response;
    }

    build_asset_response("index.html")
        .unwrap_or_else(|| (StatusCode::NOT_FOUND, "Frontend bundle not found").into_response())
}
