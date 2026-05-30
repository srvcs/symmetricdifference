use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use utoipa::{OpenApi, ToSchema};

/// This service's identity. `srvcs-symmetricdifference` is a leaf: it depends on
/// no other service. It computes the symmetric difference of two lists of
/// integers entirely with local logic.
pub const SERVICE: &str = "srvcs-symmetricdifference";
pub const CONCERN: &str = "sets: symmetric difference";
pub const DEPENDS_ON: &[&str] = &[];

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    /// The first set, as a list of integers. Every element must be a JSON
    /// integer (i64). Duplicates are allowed and treated as a set.
    #[schema(value_type = Object)]
    pub a: Vec<Value>,
    /// The second set, as a list of integers. Every element must be a JSON
    /// integer (i64). Duplicates are allowed and treated as a set.
    #[schema(value_type = Object)]
    pub b: Vec<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct SymmetricDifferenceResponse {
    #[schema(value_type = Object)]
    pub a: Vec<Value>,
    #[schema(value_type = Object)]
    pub b: Vec<Value>,
    pub result: Vec<i64>,
}

/// The single concern: the symmetric difference of `a` and `b`.
///
/// Returns `None` if any element of either list is not a JSON integer.
/// Otherwise returns `Some(result)` where `result` is the sorted list of
/// distinct values that appear in exactly one of `a` or `b` (not both).
pub fn symmetric_difference(a: &[Value], b: &[Value]) -> Option<Vec<i64>> {
    let set_a = to_set(a)?;
    let set_b = to_set(b)?;
    // BTreeSet::symmetric_difference yields the elements in ascending order.
    Some(set_a.symmetric_difference(&set_b).copied().collect())
}

/// Parse every element of `values` as an i64 into a `BTreeSet`, deduplicating.
///
/// Returns `None` if any element is not a JSON integer.
fn to_set(values: &[Value]) -> Option<BTreeSet<i64>> {
    let mut set = BTreeSet::new();
    for v in values {
        match v.as_i64() {
            Some(n) => {
                set.insert(n);
            }
            None => return None,
        }
    }
    Some(set)
}

/// `POST /` — the symmetric difference of the lists `a` and `b`.
///
/// Reads each element of both lists as a JSON integer (`i64`). If any element is
/// not an integer the request is rejected with `422`. Otherwise the sorted list
/// of distinct values appearing in exactly one of `a` or `b` is returned as
/// `result`.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = SymmetricDifferenceResponse),
        (status = 422, description = "an element is not a valid integer")
    )
)]
pub async fn evaluate(Json(req): Json<EvalRequest>) -> Response {
    match symmetric_difference(&req.a, &req.b) {
        Some(result) => (
            StatusCode::OK,
            Json(json!({ "a": req.a, "b": req.b, "result": result })),
        )
            .into_response(),
        None => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "a and b must be lists of integers" })),
        )
            .into_response(),
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, SymmetricDifferenceResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some(), "GET / documented");
        assert!(root.post.is_some(), "POST / documented");
    }

    #[test]
    fn index_reports_identity() {
        // Identity constants are the public contract of this leaf service.
        assert_eq!(SERVICE, "srvcs-symmetricdifference");
        assert_eq!(CONCERN, "sets: symmetric difference");
        assert!(DEPENDS_ON.is_empty());
    }

    #[test]
    fn asserted_case() {
        assert_eq!(
            symmetric_difference(
                &[json!(1), json!(2), json!(3)],
                &[json!(2), json!(3), json!(4)]
            ),
            Some(vec![1, 4])
        );
    }

    #[test]
    fn result_is_sorted_and_distinct() {
        assert_eq!(
            symmetric_difference(&[json!(4), json!(1), json!(9)], &[json!(9), json!(2)]),
            Some(vec![1, 2, 4])
        );
    }

    #[test]
    fn duplicates_within_a_list_are_ignored() {
        // Duplicates inside a list collapse to a single set member.
        assert_eq!(
            symmetric_difference(&[json!(1), json!(1), json!(2)], &[json!(2), json!(2)]),
            Some(vec![1])
        );
        assert_eq!(
            symmetric_difference(&[json!(5), json!(5)], &[json!(5), json!(5)]),
            Some(vec![])
        );
    }

    #[test]
    fn disjoint_lists_yield_all_elements() {
        assert_eq!(
            symmetric_difference(&[json!(1), json!(2)], &[json!(3), json!(4)]),
            Some(vec![1, 2, 3, 4])
        );
    }

    #[test]
    fn identical_sets_yield_empty() {
        assert_eq!(
            symmetric_difference(
                &[json!(1), json!(2), json!(3)],
                &[json!(3), json!(2), json!(1)]
            ),
            Some(vec![])
        );
    }

    #[test]
    fn empty_lists() {
        assert_eq!(symmetric_difference(&[], &[]), Some(vec![]));
        assert_eq!(
            symmetric_difference(&[json!(1), json!(2)], &[]),
            Some(vec![1, 2])
        );
        assert_eq!(
            symmetric_difference(&[], &[json!(-1), json!(0)]),
            Some(vec![-1, 0])
        );
    }

    #[test]
    fn negatives_are_ordered() {
        assert_eq!(
            symmetric_difference(&[json!(-3), json!(0)], &[json!(0), json!(5)]),
            Some(vec![-3, 5])
        );
    }

    #[test]
    fn non_integer_element_is_rejected() {
        for bad in [
            json!("1"),
            json!(1.5),
            json!(true),
            json!(null),
            json!([1]),
            json!({ "v": 1 }),
        ] {
            assert_eq!(
                symmetric_difference(&[json!(1), bad.clone()], &[json!(2)]),
                None,
                "{bad} in a should be rejected"
            );
            assert_eq!(
                symmetric_difference(&[json!(1)], &[json!(2), bad.clone()]),
                None,
                "{bad} in b should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn evaluate_returns_200_with_result() {
        let resp = evaluate(Json(EvalRequest {
            a: vec![json!(1), json!(2), json!(3)],
            b: vec![json!(2), json!(3), json!(4)],
        }))
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn evaluate_returns_422_for_non_integer() {
        let resp = evaluate(Json(EvalRequest {
            a: vec![json!(1), json!(1.5)],
            b: vec![json!(2)],
        }))
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn index_reports_identity_over_http() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-symmetricdifference");
        assert!(info.depends_on.is_empty());
    }
}
