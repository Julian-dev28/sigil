//! HTTP request handlers implementing the Sigil API.
//!
//! Every handler returns [`Result<_, ApiError>`]; `ApiError` renders the shared
//! JSON error envelope. Request and response bodies use `camelCase` to match the
//! published API and the TypeScript client.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use sigil_core::{
    issue_jwt_vc, verify_jwt_vc, CredentialStatus, Did, StatusList2021, VcJwtClaims,
    VerifiableCredential, DEFAULT_STATUS_LIST_LEN,
};

use crate::error::ApiError;
use crate::state::SharedState;
use crate::store::{
    CredentialRecord, CredentialSummary, DEFAULT_STATUS_LIST_ID, DEFAULT_STATUS_PURPOSE,
};

/// Liveness probe.
pub(crate) async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

/// The issuer's `did:web` DID document.
pub(crate) async fn did_document(State(state): State<SharedState>) -> impl IntoResponse {
    Json(Did::did_document(
        &state.issuer_did,
        &state.issuer_key.public_key_multibase(),
    ))
}

/// OpenID for Verifiable Credential Issuance (OID4VCI) issuer metadata.
pub(crate) async fn oid4vci_metadata(State(state): State<SharedState>) -> impl IntoResponse {
    Json(json!({
        "credential_issuer": state.base_url,
        "credential_endpoint": format!("{}/api/v1/credentials/issue", state.base_url),
        "credential_configurations_supported": {
            "VerifiableCredential": {
                "format": "jwt_vc_json",
                "cryptographic_binding_methods_supported": ["did:key"],
                "credential_signing_alg_values_supported": ["EdDSA"],
                "credential_definition": {
                    "type": ["VerifiableCredential"]
                }
            }
        }
    }))
}

// ---- Issuance ------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IssueRequest {
    /// The subject DID the credential is bound to.
    subject_id: String,
    /// The credential's extra type, e.g. `EmploymentCredential`.
    #[serde(rename = "type")]
    credential_type: String,
    /// Free-form subject claims.
    #[serde(default)]
    claims: Value,
    /// Validity window in days; omit for a non-expiring credential.
    #[serde(default)]
    expires_in_days: Option<i64>,
}

#[derive(Debug, Serialize)]
struct IssueResponse {
    id: String,
    jwt: String,
    credential: VerifiableCredential,
}

/// Issue a new JWT-VC bound to the requested subject.
pub(crate) async fn issue(
    State(state): State<SharedState>,
    Json(req): Json<IssueRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if !req.subject_id.starts_with("did:") {
        return Err(ApiError::InvalidRequest(
            "subjectId must be a DID (e.g. did:key:...)".into(),
        ));
    }
    if req.credential_type.trim().is_empty() {
        return Err(ApiError::InvalidRequest("type must not be empty".into()));
    }
    let claims = match req.claims {
        Value::Null => json!({}),
        obj @ Value::Object(_) => obj,
        _ => {
            return Err(ApiError::InvalidRequest(
                "claims must be a JSON object".into(),
            ))
        }
    };
    if let Some(days) = req.expires_in_days {
        if days <= 0 {
            return Err(ApiError::InvalidRequest(
                "expiresInDays must be a positive integer".into(),
            ));
        }
    }
    let valid_for = req.expires_in_days.map(Duration::days);

    // Enrol the credential in the default StatusList2021 so it can be revoked.
    let index = state
        .store
        .next_status_index(DEFAULT_STATUS_LIST_ID)
        .await?;
    let status_url = state.status_list_url(DEFAULT_STATUS_LIST_ID);

    let mut vc = VerifiableCredential::new(
        &state.issuer_did,
        &req.subject_id,
        &req.credential_type,
        claims,
        valid_for,
    );
    vc.credential_status = Some(CredentialStatus::revocation(&status_url, index as u64));

    let jwt = issue_jwt_vc(&vc, &state.issuer_did, &state.issuer_key)?;
    let id = parse_urn_uuid(&vc.id)?;

    let record = CredentialRecord {
        id,
        subject_id: req.subject_id.clone(),
        credential_type: req.credential_type.clone(),
        jwt: jwt.clone(),
        vc: serde_json::to_value(&vc)?,
        status_list_id: DEFAULT_STATUS_LIST_ID,
        status_index: index,
        revoked: false,
        issued_at: Utc::now(),
        expires_at: vc
            .expiration_timestamp()
            .and_then(|ts| DateTime::from_timestamp(ts, 0)),
    };
    state.store.insert_credential(record).await?;

    Ok((
        StatusCode::CREATED,
        Json(IssueResponse {
            id: id.to_string(),
            jwt,
            credential: vc,
        }),
    ))
}

// ---- Verification --------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct VerifyRequest {
    jwt: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Checks {
    signature: bool,
    not_expired: bool,
    not_revoked: bool,
}

#[derive(Debug, Serialize)]
struct VerifyResponse {
    valid: bool,
    checks: Checks,
    issuer: String,
    subject: String,
    credential: VerifiableCredential,
}

/// Verify a JWT-VC's signature, expiry and revocation status.
pub(crate) async fn verify(
    State(state): State<SharedState>,
    Json(req): Json<VerifyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let resolver = state.resolver();
    let result = verify_jwt_vc(&req.jwt, &resolver)
        .map_err(|e| ApiError::VerificationFailed(e.to_string()))?;

    let not_revoked = check_not_revoked(&state, &result.claims).await?;
    let valid = result.signature && result.not_expired && not_revoked;

    Ok(Json(VerifyResponse {
        valid,
        checks: Checks {
            signature: result.signature,
            not_expired: result.not_expired,
            not_revoked,
        },
        issuer: result.claims.iss.clone(),
        subject: result.claims.sub.clone(),
        credential: result.claims.vc,
    }))
}

/// Resolve the revocation state of a verified credential.
///
/// Credentials this service issued are looked up by id; unknown credentials are
/// treated as not-revoked (this deployment cannot speak to their status).
async fn check_not_revoked(state: &SharedState, claims: &VcJwtClaims) -> Result<bool, ApiError> {
    let Some(id) = claims
        .jti
        .strip_prefix("urn:uuid:")
        .and_then(|s| Uuid::parse_str(s).ok())
    else {
        return Ok(true);
    };
    match state.store.get_credential(id).await? {
        Some(record) => Ok(!record.revoked),
        None => Ok(true),
    }
}

// ---- Listing & lookup ----------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct ListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListItem {
    id: String,
    #[serde(rename = "type")]
    credential_type: String,
    subject_id: String,
    issued_at: String,
    revoked: bool,
}

impl From<CredentialSummary> for ListItem {
    fn from(s: CredentialSummary) -> Self {
        ListItem {
            id: s.id.to_string(),
            credential_type: s.credential_type,
            subject_id: s.subject_id,
            issued_at: s.issued_at.to_rfc3339(),
            revoked: s.revoked,
        }
    }
}

#[derive(Debug, Serialize)]
struct ListResponse {
    items: Vec<ListItem>,
    total: i64,
}

/// List issued credentials, most recent first.
pub(crate) async fn list_credentials(
    State(state): State<SharedState>,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let (summaries, total) = state.store.list_credentials(limit, offset).await?;
    Ok(Json(ListResponse {
        items: summaries.into_iter().map(ListItem::from).collect(),
        total,
    }))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CredentialDetail {
    id: String,
    subject_id: String,
    #[serde(rename = "type")]
    credential_type: String,
    jwt: String,
    #[serde(rename = "credential")]
    vc: Value,
    status_list_id: i32,
    status_index: i64,
    revoked: bool,
    issued_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
}

impl From<CredentialRecord> for CredentialDetail {
    fn from(r: CredentialRecord) -> Self {
        CredentialDetail {
            id: r.id.to_string(),
            subject_id: r.subject_id,
            credential_type: r.credential_type,
            jwt: r.jwt,
            vc: r.vc,
            status_list_id: r.status_list_id,
            status_index: r.status_index,
            revoked: r.revoked,
            issued_at: r.issued_at.to_rfc3339(),
            expires_at: r.expires_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// Fetch a single issued credential by id.
pub(crate) async fn get_credential(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let id = parse_uuid(&id)?;
    match state.store.get_credential(id).await? {
        Some(record) => Ok(Json(CredentialDetail::from(record))),
        None => Err(ApiError::NotFound(format!("no credential with id {id}"))),
    }
}

// ---- Revocation & status list -------------------------------------------

#[derive(Debug, Serialize)]
struct RevokeResponse {
    id: String,
    revoked: bool,
}

/// Revoke a credential and refresh the persisted status list.
pub(crate) async fn revoke(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let id = parse_uuid(&id)?;
    let record = state
        .store
        .revoke_credential(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("no credential with id {id}")))?;

    refresh_status_list(&state, record.status_list_id).await?;

    Ok(Json(RevokeResponse {
        id: id.to_string(),
        revoked: true,
    }))
}

/// The signed StatusList2021 credential for a given list.
pub(crate) async fn status_list(
    State(state): State<SharedState>,
    Path(list_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let list = build_status_list(&state, list_id).await?;
    let url = state.status_list_url(list_id);
    let vc = list.to_credential(&url, &state.issuer_did, DEFAULT_STATUS_PURPOSE)?;
    let jwt = issue_jwt_vc(&vc, &state.issuer_did, &state.issuer_key)?;
    Ok(Json(json!({ "jwt": jwt, "credential": vc })))
}

/// Rebuild the StatusList2021 bitstring for a list from its revoked indices.
async fn build_status_list(state: &SharedState, list_id: i32) -> Result<StatusList2021, ApiError> {
    let indices = state.store.revoked_indices(list_id).await?;
    let mut list = StatusList2021::new(DEFAULT_STATUS_LIST_LEN);
    for index in indices {
        // Indices originate from issuance and are always in range; ignore any
        // stray out-of-range value rather than failing the whole response.
        let _ = list.set(index as usize, true);
    }
    Ok(list)
}

/// Recompute and persist the encoded status list after a revocation.
async fn refresh_status_list(state: &SharedState, list_id: i32) -> Result<(), ApiError> {
    let list = build_status_list(state, list_id).await?;
    let encoded = list.encode()?;
    state
        .store
        .upsert_status_list(list_id, DEFAULT_STATUS_PURPOSE, &encoded)
        .await?;
    Ok(())
}

// ---- Helpers -------------------------------------------------------------

fn parse_uuid(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw)
        .map_err(|_| ApiError::InvalidRequest(format!("invalid credential id: {raw}")))
}

fn parse_urn_uuid(id: &str) -> Result<Uuid, ApiError> {
    id.strip_prefix("urn:uuid:")
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::Internal(format!("credential id is not a urn:uuid: {id}")))
}
