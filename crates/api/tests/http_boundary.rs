use anansi_api::{ApiState, router};
use anansi_core::{Core, ErrorCode};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;
fn state() -> Arc<ApiState> {
    Arc::new(ApiState {
        core: Core::default(),
        token: "synthetic-test-session-only".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    })
}
fn request() -> axum::http::request::Builder {
    Request::builder()
        .method("POST")
        .uri("/v1/readiness")
        .header("Host", "127.0.0.1:1451")
        .header("Origin", "http://127.0.0.1:1420")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer synthetic-test-session-only")
}
#[tokio::test]
async fn actual_http_and_core_command_share_response_and_errors() {
    let s = state();
    let app = router(s.clone());
    let r = app
        .clone()
        .oneshot(
            request()
                .body(Body::from(r#"{"contractVersion":1}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let result: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 4096).await.unwrap()).unwrap();
    let direct = serde_json::to_value(
        s.core
            .readiness(serde_json::json!({"contractVersion":1}))
            .unwrap(),
    )
    .unwrap();
    for field in [
        "contractVersion",
        "applicationName",
        "applicationVersion",
        "coreVersion",
        "operatingSystem",
    ] {
        assert_eq!(result[field], direct[field]);
    }
    assert_eq!(result["apiVersion"], env!("CARGO_PKG_VERSION"));
    assert!(direct["apiVersion"].is_null());
    for input in [
        serde_json::json!({}),
        serde_json::json!({"contractVersion":2}),
        serde_json::json!({"contractVersion":1,"path":"outside"}),
    ] {
        let expected = s.core.readiness(input.clone()).unwrap_err();
        let response = app
            .clone()
            .oneshot(request().body(Body::from(input.to_string())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let actual: anansi_core::CommandError =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(actual, expected);
    }
}
#[tokio::test]
async fn denies_wrong_and_missing_credentials_host_origin_and_malformed_body() {
    for (header, value, status) in [
        ("Authorization", "Bearer wrong", 401),
        ("Host", "evil.test:1451", 403),
        ("Origin", "https://evil.test", 403),
        ("Content-Type", "text/plain", 400),
    ] {
        let mut req = request()
            .body(Body::from(r#"{"contractVersion":1}"#))
            .unwrap();
        req.headers_mut().insert(
            header.parse::<axum::http::HeaderName>().unwrap(),
            value.parse().unwrap(),
        );
        assert_eq!(
            router(state())
                .oneshot(req)
                .await
                .unwrap()
                .status()
                .as_u16(),
            status
        );
    }
    for header in ["Authorization", "Host", "Origin"] {
        let mut req = request().body(Body::empty()).unwrap();
        req.headers_mut().remove(header);
        assert!(
            !router(state())
                .oneshot(req)
                .await
                .unwrap()
                .status()
                .is_success()
        );
    }
    for body in ["{", &"x".repeat(2000)] {
        let response = router(state())
            .oneshot(request().body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
#[tokio::test]
async fn preflight_and_queries_are_strict() {
    let preflight = Request::builder()
        .method("OPTIONS")
        .uri("/v1/readiness")
        .header("host", "127.0.0.1:1451")
        .header("origin", "http://127.0.0.1:1420")
        .header("access-control-request-method", "POST")
        .header(
            "access-control-request-headers",
            "authorization,content-type",
        )
        .body(Body::empty())
        .unwrap();
    let r = router(state()).oneshot(preflight).await.unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        r.headers()["access-control-allow-origin"],
        "http://127.0.0.1:1420"
    );
    let req = request()
        .uri("/v1/readiness?token=not-accepted")
        .body(Body::from(r#"{"contractVersion":1}"#))
        .unwrap();
    let r = router(state()).oneshot(req).await.unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    let error: anansi_core::CommandError =
        serde_json::from_slice(&to_bytes(r.into_body(), 4096).await.unwrap()).unwrap();
    assert_eq!(error.code, ErrorCode::InvalidRequest);
}

#[tokio::test]
async fn ambiguous_headers_and_old_sessions_are_rejected() {
    for header in ["host", "origin", "authorization"] {
        let mut req = request()
            .body(Body::from(r#"{"contractVersion":1}"#))
            .unwrap();
        let value = req.headers().get(header).unwrap().clone();
        req.headers_mut().append(header, value);
        assert!(
            !router(state())
                .oneshot(req)
                .await
                .unwrap()
                .status()
                .is_success()
        );
    }
    let replacement = Arc::new(ApiState {
        core: Core::default(),
        token: "replacement-session".into(),
        host: "127.0.0.1:1451".into(),
        origin: "http://127.0.0.1:1420".into(),
    });
    let response = router(replacement)
        .oneshot(
            request()
                .body(Body::from(r#"{"contractVersion":1}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.headers()["cache-control"], "no-store");
}

#[tokio::test]
async fn unsupported_preflights_and_routes_do_not_expose_data() {
    for (uri, method, headers) in [
        ("/v1/readiness?token=ignored", "POST", "authorization"),
        ("/v1/readiness", "DELETE", "authorization"),
        ("/v1/readiness", "POST", "x-unapproved"),
        ("/filesystem", "POST", "authorization"),
    ] {
        let req = Request::builder()
            .method("OPTIONS")
            .uri(uri)
            .header("host", "127.0.0.1:1451")
            .header("origin", "http://127.0.0.1:1420")
            .header("access-control-request-method", method)
            .header("access-control-request-headers", headers)
            .body(Body::empty())
            .unwrap();
        let response = router(state()).oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(
            response
                .headers()
                .get("access-control-allow-origin")
                .is_none()
        );
    }
    let response = router(state())
        .oneshot(request().uri("/filesystem").body(Body::from("{}")).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
