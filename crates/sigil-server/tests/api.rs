//! End-to-end HTTP tests exercising the router with the in-memory store.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use sigil_core::KeyPair;
use sigil_server::store::MemoryStore;
use sigil_server::{build_router, AppState};

fn test_app() -> Router {
    let store = Arc::new(MemoryStore::new());
    let key = KeyPair::generate();
    let state = Arc::new(AppState::new(store, key, "localhost:8080".to_string()));
    build_router(state)
}

fn subject_did() -> String {
    KeyPair::generate().did_key()
}

async fn call(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.expect("router responds");
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, body)
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

async fn issue(app: &Router, subject: &str) -> (String, String) {
    let (status, body) = call(
        app,
        post(
            "/api/v1/credentials/issue",
            json!({
                "subjectId": subject,
                "type": "EmploymentCredential",
                "claims": { "role": "Engineer", "org": "Acme" },
                "expiresInDays": 365
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "issue failed: {body}");
    let id = body["id"].as_str().expect("id").to_string();
    let jwt = body["jwt"].as_str().expect("jwt").to_string();
    (id, jwt)
}

#[tokio::test]
async fn health_reports_ok() {
    let app = test_app();
    let (status, body) = call(&app, get("/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn did_document_exposes_key() {
    let app = test_app();
    let (status, body) = call(&app, get("/.well-known/did.json")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "did:web:localhost%3A8080");
    assert_eq!(
        body["verificationMethod"][0]["type"],
        "Ed25519VerificationKey2020"
    );
}

#[tokio::test]
async fn issue_then_verify_is_valid() {
    let app = test_app();
    let subject = subject_did();
    let (_id, jwt) = issue(&app, &subject).await;

    let (status, v) = call(
        &app,
        post("/api/v1/credentials/verify", json!({ "jwt": jwt })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["valid"], true);
    assert_eq!(v["checks"]["signature"], true);
    assert_eq!(v["checks"]["notExpired"], true);
    assert_eq!(v["checks"]["notRevoked"], true);
    assert_eq!(v["subject"], subject);
    assert_eq!(v["credential"]["credentialSubject"]["role"], "Engineer");
}

#[tokio::test]
async fn tampered_signature_is_rejected() {
    let app = test_app();
    let (_id, jwt) = issue(&app, &subject_did()).await;

    let parts: Vec<&str> = jwt.split('.').collect();
    let sig = parts[2];
    let first = if sig.starts_with('A') { 'B' } else { 'A' };
    let mutated = format!("{first}{}", &sig[1..]);
    let tampered = format!("{}.{}.{}", parts[0], parts[1], mutated);

    let (status, v) = call(
        &app,
        post("/api/v1/credentials/verify", json!({ "jwt": tampered })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["checks"]["signature"], false);
    assert_eq!(v["valid"], false);
}

#[tokio::test]
async fn revoked_credential_fails_verification() {
    let app = test_app();
    let (id, jwt) = issue(&app, &subject_did()).await;

    let (status, _) = call(
        &app,
        post(&format!("/api/v1/credentials/{id}/revoke"), json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, v) = call(
        &app,
        post("/api/v1/credentials/verify", json!({ "jwt": jwt })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["checks"]["signature"], true);
    assert_eq!(v["checks"]["notRevoked"], false);
    assert_eq!(v["valid"], false);
}

#[tokio::test]
async fn list_and_fetch_credentials() {
    let app = test_app();
    let (id, _jwt) = issue(&app, &subject_did()).await;

    let (status, list) = call(&app, get("/api/v1/credentials")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(list["total"].as_i64().unwrap() >= 1);
    assert_eq!(list["items"][0]["type"], "EmploymentCredential");

    let (status, detail) = call(&app, get(&format!("/api/v1/credentials/{id}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["id"], id);
    assert!(detail["jwt"].as_str().unwrap().contains('.'));

    let (status, _) = call(
        &app,
        get("/api/v1/credentials/00000000-0000-0000-0000-000000000000"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn issue_rejects_non_did_subject() {
    let app = test_app();
    let (status, body) = call(
        &app,
        post(
            "/api/v1/credentials/issue",
            json!({ "subjectId": "not-a-did", "type": "TestCredential" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_request");
}
