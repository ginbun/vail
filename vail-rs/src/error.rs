use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("SSH error: {0}")]
    Ssh(String),

    #[error("SFTP error: {0}")]
    Sftp(String),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::Database(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Auth(e) => (axum::http::StatusCode::UNAUTHORIZED, e),
            AppError::NotFound(e) => (axum::http::StatusCode::NOT_FOUND, e),
            AppError::BadRequest(e) => (axum::http::StatusCode::BAD_REQUEST, e),
            AppError::Internal(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e),
            AppError::Ssh(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e),
            AppError::Sftp(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e),
        };

        axum::Json(serde_json::json!({
            "code": status.as_u16(),
            "message": message,
            "msg": message,
            "trace_id": null
        }))
        .into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
