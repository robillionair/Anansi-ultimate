use crate::{ApiState, error};
use anansi_core::ErrorCode;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderValue, Method, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use subtle::ConstantTimeEq;

pub async fn boundary(
    State(state): State<Arc<ApiState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let headers = request.headers();
    let one = |name: &str| {
        let mut values = headers.get_all(name).iter();
        let value = values.next().and_then(|v| v.to_str().ok());
        if values.next().is_some() { None } else { value }
    };
    if one("host") != Some(state.host.as_str()) || one("origin") != Some(state.origin.as_str()) {
        return error(
            StatusCode::FORBIDDEN,
            ErrorCode::Forbidden,
            "This browser origin or host is not allowed.",
        );
    }
    // Preflight discloses no domain data and permits only approved domain commands.
    if request.method() == Method::OPTIONS {
        if !crate::command_path(request.uri().path())
            || request.uri().query().is_some()
            || one("access-control-request-method") != Some("POST")
            || !one("access-control-request-headers").is_some_and(|s| {
                s.split(',').map(str::trim).all(|h| {
                    h.eq_ignore_ascii_case("authorization")
                        || h.eq_ignore_ascii_case("content-type")
                })
            })
        {
            return error(
                StatusCode::FORBIDDEN,
                ErrorCode::Forbidden,
                "Preflight is not allowed.",
            );
        }
        let mut response = Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .unwrap();
        cors(&mut response, &state.origin);
        response.headers_mut().insert(
            "access-control-allow-methods",
            HeaderValue::from_static("POST"),
        );
        response.headers_mut().insert(
            "access-control-allow-headers",
            HeaderValue::from_static("authorization, content-type"),
        );
        return response;
    }
    let expected = format!("Bearer {}", state.token);
    let authorized = one("authorization")
        .is_some_and(|value| value.as_bytes().ct_eq(expected.as_bytes()).into());
    let mut response = if !authorized {
        error(
            StatusCode::UNAUTHORIZED,
            ErrorCode::Unauthorized,
            "Pair with the current local session to continue.",
        )
    } else if request.method() != Method::POST
        || request.uri().query().is_some()
        || one("content-type") != Some("application/json")
    {
        error(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            "Use the supported readiness request.",
        )
    } else {
        next.run(request).await
    };
    cors(&mut response, &state.origin);
    response
}
fn cors(response: &mut Response, origin: &str) {
    response.headers_mut().insert(
        "access-control-allow-origin",
        HeaderValue::from_str(origin).unwrap(),
    );
    response
        .headers_mut()
        .insert("vary", HeaderValue::from_static("Origin"));
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
}
