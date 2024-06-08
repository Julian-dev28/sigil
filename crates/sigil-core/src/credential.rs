//! The W3C Verifiable Credential (Data Model 1.1) data model.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// The default JSON-LD context for W3C VC Data Model 1.1 credentials.
pub const VC_CONTEXT_V1: &str = "https://www.w3.org/2018/credentials/v1";
/// The JSON-LD context that defines StatusList2021 terms.
pub const STATUS_LIST_CONTEXT: &str = "https://w3id.org/vc/status-list/2021/v1";

/// A W3C Verifiable Credential (Data Model 1.1) in its JSON representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCredential {
    /// The JSON-LD `@context`.
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    /// Credential identifier, typically `urn:uuid:<uuid>`.
    pub id: String,
    /// Credential types; always includes `VerifiableCredential`.
    #[serde(rename = "type")]
    pub types: Vec<String>,
    /// The issuer DID.
    pub issuer: String,
    /// Issuance timestamp (RFC 3339).
    #[serde(rename = "issuanceDate")]
    pub issuance_date: String,
    /// Optional expiration timestamp (RFC 3339).
    #[serde(rename = "expirationDate", skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,
    /// Optional revocation status entry.
    #[serde(rename = "credentialStatus", skip_serializing_if = "Option::is_none")]
    pub credential_status: Option<CredentialStatus>,
    /// The credential subject; a free-form object that must carry an `id`.
    #[serde(rename = "credentialSubject")]
    pub credential_subject: Value,
}

impl VerifiableCredential {
    /// Build a new credential from its constituent parts.
    ///
    /// A random `urn:uuid` id is assigned, `issuanceDate` is set to now and,
    /// when `valid_for` is supplied, `expirationDate` is computed from it.
    pub fn new(
        issuer_did: &str,
        subject_id: &str,
        extra_type: &str,
        claims: Value,
        valid_for: Option<Duration>,
    ) -> Self {
        let now = Utc::now();
        let mut subject = serde_json::Map::new();
        subject.insert("id".to_string(), Value::String(subject_id.to_string()));
        if let Value::Object(map) = claims {
            for (k, v) in map {
                if k != "id" {
                    subject.insert(k, v);
                }
            }
        }

        VerifiableCredential {
            context: vec![VC_CONTEXT_V1.to_string()],
            id: format!("urn:uuid:{}", Uuid::new_v4()),
            types: vec!["VerifiableCredential".to_string(), extra_type.to_string()],
            issuer: issuer_did.to_string(),
            issuance_date: to_rfc3339(now),
            expiration_date: valid_for.map(|d| to_rfc3339(now + d)),
            credential_status: None,
            credential_subject: Value::Object(subject),
        }
    }

    /// The subject identifier (`credentialSubject.id`), if present.
    pub fn subject_id(&self) -> Option<&str> {
        self.credential_subject.get("id").and_then(Value::as_str)
    }

    /// Parse `issuanceDate` into a Unix timestamp (seconds).
    pub fn issuance_timestamp(&self) -> Option<i64> {
        parse_rfc3339(&self.issuance_date)
    }

    /// Parse `expirationDate` into a Unix timestamp (seconds), if present.
    pub fn expiration_timestamp(&self) -> Option<i64> {
        self.expiration_date.as_deref().and_then(parse_rfc3339)
    }
}

/// A StatusList2021 revocation entry embedded in a credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialStatus {
    /// The status entry id, e.g. `https://issuer/status/1#42`.
    pub id: String,
    /// Always `StatusList2021Entry`.
    #[serde(rename = "type")]
    pub type_: String,
    /// The status purpose, e.g. `revocation`.
    #[serde(rename = "statusPurpose")]
    pub status_purpose: String,
    /// The bit index of this credential within the status list.
    #[serde(rename = "statusListIndex")]
    pub status_list_index: String,
    /// The URL of the StatusList2021 credential.
    #[serde(rename = "statusListCredential")]
    pub status_list_credential: String,
}

impl CredentialStatus {
    /// Build a revocation-purpose status entry.
    pub fn revocation(status_list_credential: &str, index: u64) -> Self {
        CredentialStatus {
            id: format!("{status_list_credential}#{index}"),
            type_: "StatusList2021Entry".to_string(),
            status_purpose: "revocation".to_string(),
            status_list_index: index.to_string(),
            status_list_credential: status_list_credential.to_string(),
        }
    }
}

fn to_rfc3339(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn parse_rfc3339(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn new_credential_populates_subject_and_type() {
        let vc = VerifiableCredential::new(
            "did:web:issuer.example",
            "did:key:z6MkSubject",
            "EmploymentCredential",
            json!({ "role": "Engineer" }),
            Some(Duration::days(365)),
        );
        assert_eq!(vc.subject_id(), Some("did:key:z6MkSubject"));
        assert!(vc.types.contains(&"VerifiableCredential".to_string()));
        assert!(vc.types.contains(&"EmploymentCredential".to_string()));
        assert_eq!(vc.credential_subject["role"], "Engineer");
        assert!(vc.expiration_date.is_some());
        assert!(vc.id.starts_with("urn:uuid:"));
    }

    #[test]
    fn timestamps_parse() {
        let vc = VerifiableCredential::new(
            "did:web:issuer.example",
            "did:key:z6MkSubject",
            "TestCredential",
            json!({}),
            Some(Duration::days(1)),
        );
        assert!(vc.issuance_timestamp().is_some());
        assert!(vc.expiration_timestamp().unwrap() > vc.issuance_timestamp().unwrap());
    }
}
