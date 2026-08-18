//! The axum API.
//!
//! Every route answers from the store, the DynamoDB table
//! `docs/design/persistence.md` describes.
//!
//! Every request under `/api` names [`identity::Owner`] in its handler
//! signature, which is what decides who the caller is and whether they are
//! one at all — DR-0028. `/health` names it deliberately, since a probe has
//! no token.
//!
//! No CORS layer is configured: in development the browser talks to trunk, which
//! proxies `/api` here, so every request is same-origin. Production serves the
//! SPA from a different origin, and CORS is answered by the HTTP API rather than
//! here — see DR-0009.

mod action_types;
mod actions;
mod cognito;
mod dashboard;
mod identity;
mod jwks;
mod store;
#[cfg(test)]
mod testkey;

use std::sync::Arc;

use axum::Router;
use axum::extract::FromRef;
use axum::routing::get;

use crate::identity::Auth;
use crate::store::Store;

const ADDRESS: &str = "127.0.0.1:3000";

#[derive(Clone)]
struct AppState {
    store: Arc<Store>,
    auth: Arc<Auth>,
}

impl FromRef<AppState> for Arc<Store> {
    fn from_ref(state: &AppState) -> Self {
        state.store.clone()
    }
}

impl FromRef<AppState> for Arc<Auth> {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Once, at startup: the SDK client is expensive to build and cheap to
    // share, and which store this is cannot change while the process runs.
    let store = Arc::new(Store::from_environment().await);
    println!("action types are stored in {}", store.describe());

    // Before the listener, deliberately — a pool that cannot be reached is a
    // reason to stop with the reason on screen, not to accept connections and
    // refuse every one of them for a cause nobody can see.
    let auth = Arc::new(Auth::from_environment().await?);
    println!("callers are authenticated by {}", auth.describe());

    let app = router(AppState { store, auth });

    let listener = tokio::net::TcpListener::bind(ADDRESS).await?;
    println!("API listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

/// Separate from [`main`] so that a test can drive it with
/// `tower::ServiceExt::oneshot` instead of a bound listener — see the
/// `#[cfg(test)] mod tests` below and `testing.md`.
fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/dashboard", get(dashboard::dashboard))
        .route(
            "/api/action-types",
            get(action_types::list).post(action_types::create),
        )
        .route(
            "/api/action-types/{id}",
            get(action_types::get_one)
                .put(action_types::update)
                .delete(action_types::delete),
        )
        .route("/api/actions", get(actions::list).post(actions::create))
        .route(
            "/api/actions/{id}",
            get(actions::get_one)
                .put(actions::update)
                .delete(actions::delete),
        )
        .with_state(state)
}

/// What the Lambda Web Adapter's readiness check asks for, and what a probe
/// gets. Deliberately unauthenticated, and it reveals nothing.
async fn health() -> &'static str {
    "ok"
}

/// Drives the real [`router`] with `tower::ServiceExt::oneshot` — in-process,
/// no listener, no network. What every other test in this crate calls
/// directly (`validate()`, a `Store` method, `mock_caller()`) is exercised
/// here as an HTTP request instead, so routing, extraction and (de)serialisation
/// are checked once, at the boundary that actually receives them — testing.md.
#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::*;
    use crate::cognito::{Verification, Verifier};
    use crate::jwks::Keys;
    use crate::testkey;

    fn memory_state(auth: Auth) -> AppState {
        AppState {
            store: Arc::new(Store::memory()),
            auth: Arc::new(auth),
        }
    }

    /// Both headers `Auth::Mock` reads for a named caller — DR-0028.
    fn mock_caller(request: Request<Body>, subject: &str) -> Request<Body> {
        let (mut parts, body) = request.into_parts();
        parts.headers.insert("x-auth-edge", "test".parse().unwrap());
        parts
            .headers
            .insert("x-auth-subject", subject.parse().unwrap());
        Request::from_parts(parts, body)
    }

    fn cognito_verifier() -> Verifier {
        Verifier {
            verification: Verification {
                issuer: testkey::ISSUER.to_owned(),
                audience: testkey::AUDIENCE.to_owned(),
            },
            keys: Keys::parse(testkey::JWKS.as_bytes()).unwrap(),
        }
    }

    /// A well-formed access token whose `exp` is against the real clock —
    /// `identity::tests::current_token` is the same fixture, duplicated
    /// rather than shared because each test module here already keeps its
    /// own, `cognito.rs` and `jwks.rs` included.
    fn current_token() -> String {
        testkey::sign(
            &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
            &format!(
                r#"{{"sub":"abc-123","iss":"{}","client_id":"{}","token_use":"access","exp":{}}}"#,
                testkey::ISSUER,
                testkey::AUDIENCE,
                crate::cognito::unix_time() + 3600,
            ),
        )
    }

    async fn json_body(response: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_is_reachable_with_no_auth_at_all() {
        let app = router(memory_state(Auth::Mock));

        let response = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// The end-to-end path every other test reaches into: a real JSON body,
    /// routed, extracted, validated, stored, and answered as JSON — then read
    /// back through a second request, which is what proves the first one
    /// reached the store and not just the handler's return value.
    #[tokio::test]
    async fn creating_and_then_listing_goes_through_the_router() {
        let app = router(memory_state(Auth::Mock));

        let create = mock_caller(
            Request::post("/api/action-types")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Running", "unit": "km", "icon": "footprints"}).to_string(),
                ))
                .unwrap(),
            "alice",
        );
        let response = app.clone().oneshot(create).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created = json_body(response).await;
        assert_eq!(created["name"], "Running");

        let list = mock_caller(
            Request::get("/api/action-types")
                .body(Body::empty())
                .unwrap(),
            "alice",
        );
        let response = app.oneshot(list).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let listed = json_body(response).await;
        assert_eq!(listed, json!([created]));
    }

    /// Nothing in `action_types::validate` runs: the body never reaches the
    /// handler at all, because `Json<NewActionType>` fails to extract. Axum's
    /// own rejection answers this, not `action_types::Failure`.
    #[tokio::test]
    async fn a_malformed_json_body_is_a_400_not_a_500() {
        let app = router(memory_state(Auth::Mock));

        let request = mock_caller(
            Request::post("/api/action-types")
                .header("content-type", "application/json")
                .body(Body::from("not json"))
                .unwrap(),
            "alice",
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// A path the router knows, with a method none of its handlers answer —
    /// axum's own routing, not anything this crate wrote.
    #[tokio::test]
    async fn an_unmapped_method_is_405() {
        let app = router(memory_state(Auth::Mock));

        let request = mock_caller(
            Request::patch("/api/action-types")
                .body(Body::empty())
                .unwrap(),
            "alice",
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    /// `identity::tests::an_edge_that_named_nobody_is_refused` checks
    /// `mock_caller()` in isolation; this checks that the rejection it
    /// produces actually reaches the visitor as `401` once axum's
    /// `IntoResponse` machinery is in the loop.
    #[tokio::test]
    async fn an_edge_naming_nobody_is_401_through_the_router() {
        let app = router(memory_state(Auth::Mock));

        let (mut parts, body) = Request::get("/api/action-types")
            .body(Body::empty())
            .unwrap()
            .into_parts();
        parts.headers.insert("x-auth-edge", "test".parse().unwrap());
        let request = Request::from_parts(parts, body);

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    /// `identity::tests::no_token_under_cognito_is_refused_not_the_development_owner`
    /// checks the extractor function directly; this is the same rule through
    /// the router `Auth::Cognito` actually runs under.
    #[tokio::test]
    async fn a_missing_token_under_cognito_is_401_through_the_router() {
        let app = router(memory_state(Auth::Cognito(cognito_verifier())));

        let response = app
            .oneshot(
                Request::get("/api/action-types")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn a_verified_cognito_token_reaches_the_handler() {
        let app = router(memory_state(Auth::Cognito(cognito_verifier())));

        let response = app
            .oneshot(
                Request::get("/api/action-types")
                    .header("authorization", format!("Bearer {}", current_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await, json!([]));
    }

    /// The action-record counterpart of
    /// `creating_and_then_listing_goes_through_the_router`: a type is
    /// registered first, a record against it is created, updated and read
    /// back, and finally deleted — the full lifecycle through routing,
    /// extraction and (de)serialisation, not just the handler's own return
    /// value.
    #[tokio::test]
    async fn recording_updating_and_deleting_an_action_goes_through_the_router() {
        let app = router(memory_state(Auth::Mock));

        let create_type = mock_caller(
            Request::post("/api/action-types")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Running", "unit": "km", "icon": "footprints"}).to_string(),
                ))
                .unwrap(),
            "alice",
        );
        let response = app.clone().oneshot(create_type).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let action_type = json_body(response).await;
        let type_id = action_type["id"].as_str().unwrap().to_owned();

        let create_record = mock_caller(
            Request::post("/api/actions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"type_id": type_id, "value": 5.2}).to_string(),
                ))
                .unwrap(),
            "alice",
        );
        let response = app.clone().oneshot(create_record).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created = json_body(response).await;
        assert_eq!(created["action_type"], action_type);
        assert_eq!(created["value"], 5.2);
        let record_id = created["id"].as_str().unwrap().to_owned();

        let list = mock_caller(
            Request::get("/api/actions").body(Body::empty()).unwrap(),
            "alice",
        );
        let response = app.clone().oneshot(list).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await, json!([created]));

        let update = mock_caller(
            Request::put(format!("/api/actions/{record_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({"value": 6.0}).to_string()))
                .unwrap(),
            "alice",
        );
        let response = app.clone().oneshot(update).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let updated = json_body(response).await;
        assert_eq!(updated["value"], 6.0);
        assert_eq!(updated["id"], record_id.clone());

        let get = mock_caller(
            Request::get(format!("/api/actions/{record_id}"))
                .body(Body::empty())
                .unwrap(),
            "alice",
        );
        let response = app.clone().oneshot(get).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await, updated);

        let delete = mock_caller(
            Request::delete(format!("/api/actions/{record_id}"))
                .body(Body::empty())
                .unwrap(),
            "alice",
        );
        let response = app.clone().oneshot(delete).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let list_again = mock_caller(
            Request::get("/api/actions").body(Body::empty()).unwrap(),
            "alice",
        );
        let response = app.oneshot(list_again).await.unwrap();
        assert_eq!(json_body(response).await, json!([]));
    }

    /// `type_id` naming nothing in the caller's own partition is a `400`, not
    /// a `404` — it is a body field, not the URL's own addressed resource.
    #[tokio::test]
    async fn creating_an_action_for_an_unknown_type_is_400_through_the_router() {
        let app = router(memory_state(Auth::Mock));

        let create_record = mock_caller(
            Request::post("/api/actions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"type_id": "not-a-real-id", "value": 5.2}).to_string(),
                ))
                .unwrap(),
            "alice",
        );
        let response = app.oneshot(create_record).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// The dashboard counterpart of the actions lifecycle test above: two
    /// actions are recorded through `/api/actions`, then `GET
    /// /api/dashboard` is what proves `dashboard::dashboard` actually reads
    /// them back through the router rather than answering fixed values.
    #[tokio::test]
    async fn dashboard_reflects_recorded_actions_through_the_router() {
        let app = router(memory_state(Auth::Mock));

        let create_type = mock_caller(
            Request::post("/api/action-types")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Running", "unit": "km", "icon": "footprints"}).to_string(),
                ))
                .unwrap(),
            "alice",
        );
        let response = app.clone().oneshot(create_type).await.unwrap();
        let type_id = json_body(response).await["id"].as_str().unwrap().to_owned();

        for value in [5.2, 3.1] {
            let create_record = mock_caller(
                Request::post("/api/actions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"type_id": type_id, "value": value}).to_string(),
                    ))
                    .unwrap(),
                "alice",
            );
            let response = app.clone().oneshot(create_record).await.unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);
        }

        let dashboard = mock_caller(
            Request::get("/api/dashboard").body(Body::empty()).unwrap(),
            "alice",
        );
        let response = app.oneshot(dashboard).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;

        assert_eq!(body["summary"]["total"], 2);
        assert_eq!(body["summary"]["daily"].as_array().unwrap().len(), 10);
        let recent = body["recent"].as_array().unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0]["value"], 3.1);
        assert_eq!(recent[1]["value"], 5.2);
    }
}
