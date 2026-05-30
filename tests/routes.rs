use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_symmetricdifference::{health, router, telemetry};
use tower::ServiceExt;

fn app() -> axum::Router {
    router(telemetry::metrics_handle_for_tests())
}

async fn status_of(uri: &str) -> StatusCode {
    app()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

/// POST `{ "a": <a>, "b": <b> }` to `/` and return (status, parsed JSON).
async fn eval(a: Value, b: Value) -> (StatusCode, Value) {
    let res = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "a": a, "b": b }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

// --- Standard srvcs service surface ---

#[tokio::test]
async fn index_ok() {
    assert_eq!(status_of("/").await, StatusCode::OK);
}

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

// --- Symmetric difference cases ---

#[tokio::test]
async fn asserted_case() {
    let (status, body) = eval(json!([1, 2, 3]), json!([2, 3, 4])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([1, 4]));
    assert_eq!(body["a"], json!([1, 2, 3]));
    assert_eq!(body["b"], json!([2, 3, 4]));
}

#[tokio::test]
async fn result_is_sorted() {
    let (status, body) = eval(json!([4, 1, 9]), json!([9, 2])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([1, 2, 4]));
}

#[tokio::test]
async fn duplicates_within_lists_are_collapsed() {
    let (status, body) = eval(json!([1, 1, 2]), json!([2, 2])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([1]));
}

#[tokio::test]
async fn identical_sets_yield_empty() {
    let (status, body) = eval(json!([1, 2, 3]), json!([3, 2, 1])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([]));
}

#[tokio::test]
async fn disjoint_lists_yield_all_elements() {
    let (status, body) = eval(json!([1, 2]), json!([3, 4])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([1, 2, 3, 4]));
}

#[tokio::test]
async fn empty_lists_yield_empty() {
    let (status, body) = eval(json!([]), json!([])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([]));
}

#[tokio::test]
async fn one_empty_list_yields_the_other() {
    let (status, body) = eval(json!([1, 2]), json!([])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([1, 2]));
}

#[tokio::test]
async fn negatives_are_ordered() {
    let (status, body) = eval(json!([-3, 0]), json!([0, 5])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!([-3, 5]));
}

// --- Error / edge cases ---

#[tokio::test]
async fn non_integer_in_a_is_422() {
    let (status, body) = eval(json!([1, "nope", 3]), json!([2])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "a and b must be lists of integers");
}

#[tokio::test]
async fn float_in_b_is_422() {
    let (status, body) = eval(json!([1]), json!([2, 1.5])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "a and b must be lists of integers");
}

#[tokio::test]
async fn missing_field_is_422() {
    // A body without the `b` field is a client error, not a 500.
    let res = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "a": [1] }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let res = app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        res.headers().contains_key("x-request-id"),
        "response must carry a generated x-request-id"
    );
}
