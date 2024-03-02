use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

pub const HTTP_STATUS_OK: i32 = 0;
pub const HTTP_STATUS_ERROR_SCOPE: i32 = 4000001;
pub const HTTP_STATUS_ERROR_UNKNOWN: i32 = 5000001;

#[derive(Debug)]
pub enum BackendError {
  CommonException { status: i32, msg: String },
  ScopeException(anyhow::Error),
  UnknownException(anyhow::Error),
}

impl IntoResponse for BackendError {
  fn into_response(self) -> axum::response::Response {
    let (status, msg) = match self {
      BackendError::CommonException { status, msg } => (status, msg),
      BackendError::ScopeException(err) => {
        tracing::error!("stacktrace: {}", err);
        (HTTP_STATUS_ERROR_SCOPE, "Scope Exception".to_string())
      }
      BackendError::UnknownException(err) => {
        tracing::error!("stacktrace: {}", err);
        (HTTP_STATUS_ERROR_UNKNOWN, "Unknown Exception".to_string())
      }
    };

    let body = Json(json!({
        "status": status,
        "msg": msg,
    }));

    (StatusCode::OK, body).into_response()
  }
}

impl From<anyhow::Error> for BackendError {
  fn from(value: anyhow::Error) -> Self {
    BackendError::UnknownException(value)
  }
}
