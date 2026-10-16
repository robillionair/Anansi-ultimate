pub mod auth;
pub mod pairing;
use anansi_core::{CommandError, Core, ErrorCode};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{Request, StatusCode},
    middleware::{self},
    response::{IntoResponse, Response},
    routing::post,
};
use std::sync::Arc;

pub struct ApiState {
    pub core: Core,
    pub token: String,
    pub host: String,
    pub origin: String,
}
pub fn router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/v1/readiness", post(readiness))
        .fallback(|| async {
            error(
                StatusCode::NOT_FOUND,
                ErrorCode::NotFound,
                "Command not found.",
            )
        })
        .layer(DefaultBodyLimit::max(1024))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::boundary,
        ))
        .with_state(state)
}
pub fn error(status: StatusCode, code: ErrorCode, message: &str) -> Response {
    (status, Json(CommandError::new(code, message))).into_response()
}
async fn readiness(State(state): State<Arc<ApiState>>, request: Request<Body>) -> Response {
    let bytes = match axum::body::to_bytes(request.into_body(), 1024).await {
        Ok(b) => b,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "Use the supported readiness request.",
            );
        }
    };
    let input = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "Use the supported readiness request.",
            );
        }
    };
    match state.core.readiness(input) {
        Ok(mut value) => {
            value.api_version = Some(env!("CARGO_PKG_VERSION").into());
            Json(value).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(e)).into_response(),
    }
}
