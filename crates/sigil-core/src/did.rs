//! Decentralized Identifier (`did:web`, `did:key`) construction and resolution.

use ed25519_dalek::VerifyingKey;
use serde_json::json;

use crate::error::CoreError;
use crate::keys::verifying_key_from_multibase;

/// Helpers for constructing and resolving Decentralized Identifiers.
///
/// Sigil issues credentials under a `did:web` identifier and accepts
/// `did:key` subjects, so only those two methods are handled here.
pub struct Did;

impl Did {
    /// Build a `did:web` identifier for the given host.
    ///
    /// Per the did:web method, a port is percent-encoded (`:` -> `%3A`).
    pub fn did_web(host: &str) -> String {
        did_web(host)
    }

    /// Build a DID document for `did` exposing a single Ed25519 key.
    pub fn did_document(did: &str, key_multibase: &str) -> serde_json::Value {
        did_document(did, key_multibase)
    }
}

/// Build a `did:web` identifier for the given host (percent-encoding any port).
pub fn did_web(host: &str) -> String {
    format!("did:web:{}", host.replace(':', "%3A"))
}

/// Build a DID document exposing a single `Ed25519VerificationKey2020` key,
/// referenced by both `authentication` and `assertionMethod`.
pub fn did_document(did: &str, key_multibase: &str) -> serde_json::Value {
    let key_id = format!("{did}#key-1");
    json!({
        "@context": [
            "https://www.w3.org/ns/did/v1",
            "https://w3id.org/security/suites/ed25519-2020/v1"
        ],
        "id": did,
        "verificationMethod": [{
            "id": key_id,
            "type": "Ed25519VerificationKey2020",
            "controller": did,
            "publicKeyMultibase": key_multibase
        }],
        "authentication": [key_id],
        "assertionMethod": [key_id]
    })
}

/// Extract the Ed25519 verifying key embedded in a `did:key` identifier.
pub fn did_key_to_verifying_key(did: &str) -> Result<VerifyingKey, CoreError> {
    let multibase = did
        .strip_prefix("did:key:")
        .ok_or_else(|| CoreError::Did(format!("not a did:key identifier: {did}")))?;
    verifying_key_from_multibase(multibase)
}

/// Resolves a DID to the Ed25519 verifying key that controls it.
///
/// Implemented by the server so that the same [`verify_jwt_vc`](crate::verify_jwt_vc)
/// routine can resolve both the local issuer's `did:web` and remote `did:key`s.
pub trait DidResolver {
    /// Resolve `did` to its Ed25519 verifying key.
    fn resolve_key(&self, did: &str) -> Result<VerifyingKey, CoreError>;
}

/// A [`DidResolver`] that resolves any `did:key` locally and, optionally,
/// a single statically-registered DID (typically the issuer's `did:web`).
pub struct StaticResolver {
    did: Option<String>,
    key: Option<VerifyingKey>,
}

impl StaticResolver {
    /// A resolver that only knows how to resolve `did:key` identifiers.
    pub fn key_only() -> Self {
        StaticResolver {
            did: None,
            key: None,
        }
    }

    /// A resolver that additionally resolves `did` to `key`.
    pub fn with_registered(did: impl Into<String>, key: VerifyingKey) -> Self {
        StaticResolver {
            did: Some(did.into()),
            key: Some(key),
        }
    }
}

impl DidResolver for StaticResolver {
    fn resolve_key(&self, did: &str) -> Result<VerifyingKey, CoreError> {
        if let (Some(registered), Some(key)) = (&self.did, &self.key) {
            if registered == did {
                return Ok(*key);
            }
        }
        if did.starts_with("did:key:") {
            return did_key_to_verifying_key(did);
        }
        Err(CoreError::Resolution(format!("cannot resolve DID: {did}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyPair;

    #[test]
    fn did_web_encodes_port() {
        assert_eq!(did_web("issuer.example"), "did:web:issuer.example");
        assert_eq!(did_web("localhost:8080"), "did:web:localhost%3A8080");
    }

    #[test]
    fn did_document_contains_key() {
        let doc = did_document("did:web:issuer.example", "z6MkExample");
        assert_eq!(doc["id"], "did:web:issuer.example");
        assert_eq!(
            doc["verificationMethod"][0]["type"],
            "Ed25519VerificationKey2020"
        );
        assert_eq!(
            doc["verificationMethod"][0]["publicKeyMultibase"],
            "z6MkExample"
        );
    }

    #[test]
    fn did_key_resolves_to_original_key() {
        let kp = KeyPair::generate();
        let key = did_key_to_verifying_key(&kp.did_key()).expect("resolve did:key");
        assert_eq!(key.as_bytes(), kp.verifying_key().as_bytes());
    }
}
