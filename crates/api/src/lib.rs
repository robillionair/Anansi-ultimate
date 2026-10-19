pub mod auth;
pub mod pairing;
use anansi_core::{CommandError, Core, ErrorCode};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
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
        .route("/v1/workspaces/{operation}", post(workspace))
        .route("/v1/memories/{operation}", post(memory))
        .route("/v1/relationships/{operation}", post(relationship))
        .route("/v1/jobs/{operation}", post(job))
        .route("/v1/ai/{operation}", post(ai))
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
pub fn command_path(path: &str) -> bool {
    matches!(
        path,
        "/v1/memories/create"
            | "/v1/jobs/export"
            | "/v1/jobs/list"
            | "/v1/jobs/control"
            | "/v1/jobs/artifact"
            | "/v1/jobs/backup"
            | "/v1/jobs/preview"
            | "/v1/jobs/restore"
            | "/v1/jobs/schedule"
            | "/v1/jobs/schedule-read"
            | "/v1/jobs/upload-begin"
            | "/v1/jobs/upload-part"
            | "/v1/memories/read"
            | "/v1/memories/revise"
            | "/v1/memories/list"
            | "/v1/relationships/create"
            | "/v1/relationships/delete"
            | "/v1/relationships/restore"
            | "/v1/relationships/list"
            | "/v1/readiness"
            | "/v1/workspaces/list"
            | "/v1/workspaces/create"
            | "/v1/workspaces/open"
            | "/v1/workspaces/settings"
            | "/v1/ai/discovery"
            | "/v1/ai/ask"
            | "/v1/ai/hybrid-search"
            | "/v1/ai/context"
    )
}

async fn job(
    Path(operation): Path<String>,
    State(state): State<Arc<ApiState>>,
    request: Request<Body>,
) -> Response {
    let input = match axum::body::to_bytes(request.into_body(), 384 * 1024).await {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => {
                return error(
                    StatusCode::BAD_REQUEST,
                    ErrorCode::InvalidRequest,
                    "Use a supported job request.",
                );
            }
        },
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "Job request exceeds its limit.",
            );
        }
    };
    match tokio::task::spawn_blocking(move || state.core.job_command(&operation, input)).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(e)) => {
            let status = match e.code {
                ErrorCode::NotFound | ErrorCode::WorkspaceNotFound => StatusCode::NOT_FOUND,
                ErrorCode::RevisionConflict | ErrorCode::ContentIntegrity => StatusCode::CONFLICT,
                ErrorCode::StorageUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                ErrorCode::VaultUnavailable => StatusCode::FORBIDDEN,
                _ => StatusCode::BAD_REQUEST,
            };
            (status, Json(e)).into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(anansi_core::storage::unavailable()),
        )
            .into_response(),
    }
}
async fn workspace(
    Path(operation): Path<String>,
    State(state): State<Arc<ApiState>>,
    request: Request<Body>,
) -> Response {
    let bytes = match axum::body::to_bytes(request.into_body(), 1024).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "Use the supported workspace request.",
            );
        }
    };
    let input = match serde_json::from_slice(&bytes) {
        Ok(input) => input,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "Use the supported workspace request.",
            );
        }
    };
    match tokio::task::spawn_blocking(move || state.core.workspace_command(&operation, input)).await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(error)) => {
            let status = match error.code {
                ErrorCode::WorkspaceNotFound | ErrorCode::NotFound => StatusCode::NOT_FOUND,
                ErrorCode::WorkspaceExists
                | ErrorCode::UnsupportedSchema
                | ErrorCode::CorruptStorage => StatusCode::CONFLICT,
                ErrorCode::StorageUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::BAD_REQUEST,
            };
            (status, Json(error)).into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(anansi_core::storage::unavailable()),
        )
            .into_response(),
    }
}
pub fn error(status: StatusCode, code: ErrorCode, message: &str) -> Response {
    (status, Json(CommandError::new(code, message))).into_response()
}
async fn memory(
    Path(operation): Path<String>,
    State(state): State<Arc<ApiState>>,
    request: Request<Body>,
) -> Response {
    let bytes = match axum::body::to_bytes(request.into_body(), 320 * 1024).await {
        Ok(v) => v,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidMemory,
                "Memory request exceeds its limit.",
            );
        }
    };
    let input = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidMemory,
                "Use the supported memory request.",
            );
        }
    };
    match tokio::task::spawn_blocking(move || state.core.memory_command(&operation, input)).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(e)) => {
            let status = match e.code {
                ErrorCode::MemoryNotFound => StatusCode::NOT_FOUND,
                ErrorCode::RevisionConflict
                | ErrorCode::ContentIntegrity
                | ErrorCode::UnsupportedSchema => StatusCode::CONFLICT,
                ErrorCode::StorageUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                ErrorCode::VaultUnavailable => StatusCode::FORBIDDEN,
                _ => StatusCode::BAD_REQUEST,
            };
            (status, Json(e)).into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(anansi_core::storage::unavailable()),
        )
            .into_response(),
    }
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
async fn relationship(
    Path(operation): Path<String>,
    State(state): State<Arc<ApiState>>,
    request: Request<Body>,
) -> Response {
    let bytes = match axum::body::to_bytes(request.into_body(), 64 * 1024).await {
        Ok(v) => v,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "Relationship request exceeds its limit.",
            );
        }
    };
    let input = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "Use the supported relationship request.",
            );
        }
    };
    match tokio::task::spawn_blocking(move || state.core.relationship_command(&operation, input))
        .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(e)) => {
            let status = match e.code {
                ErrorCode::NotFound => StatusCode::NOT_FOUND,
                ErrorCode::RevisionConflict
                | ErrorCode::ContentIntegrity
                | ErrorCode::UnsupportedSchema => StatusCode::CONFLICT,
                ErrorCode::VaultUnavailable => StatusCode::FORBIDDEN,
                ErrorCode::StorageUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::BAD_REQUEST,
            };
            (status, Json(e)).into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(anansi_core::storage::unavailable()),
        )
            .into_response(),
    }
}

async fn ai(
    Path(operation): Path<String>,
    State(state): State<Arc<ApiState>>,
    request: Request<Body>,
) -> Response {
    let bytes = match axum::body::to_bytes(request.into_body(), 64 * 1024).await {
        Ok(v) => v,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "AI request exceeds its limit.",
            );
        }
    };
    let input = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                "Use the supported AI request.",
            );
        }
    };
    match tokio::task::spawn_blocking(move || state.core.ai_command(&operation, input)).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(e)) => {
            let status = match e.code {
                ErrorCode::NotFound => StatusCode::NOT_FOUND,
                ErrorCode::UnsupportedContract => StatusCode::BAD_REQUEST,
                ErrorCode::StorageUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::BAD_REQUEST,
            };
            (status, Json(e)).into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(anansi_core::storage::unavailable()),
        )
            .into_response(),
    }
}
