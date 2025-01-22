//! StatusList2021 revocation bitstrings (GZIP + base64url encoded).

use std::io::{Read, Write};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::{json, Value};

use crate::credential::{VerifiableCredential, STATUS_LIST_CONTEXT, VC_CONTEXT_V1};
use crate::error::CoreError;

/// The default StatusList2021 bitstring length (16 KB = 131072 entries),
/// matching the minimum herd-privacy size recommended by the specification.
pub const DEFAULT_STATUS_LIST_LEN: usize = 16 * 1024 * 8;

/// A StatusList2021 revocation bitstring.
///
/// Each credential is assigned an index; a set bit means the credential is
/// revoked. The list is serialized as a GZIP-compressed, base64url-encoded
/// bitstring per the StatusList2021 specification.
#[derive(Debug, Clone)]
pub struct StatusList2021 {
    bits: Vec<u8>,
    len: usize,
}

impl StatusList2021 {
    /// Create an all-zero status list with capacity for `len` entries.
    pub fn new(len: usize) -> Self {
        StatusList2021 {
            bits: vec![0u8; len.div_ceil(8)],
            len,
        }
    }

    /// The number of entries the list can hold.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the list holds zero entries.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Set or clear the revocation bit at `index`.
    ///
    /// Returns an error if `index` is out of range.
    pub fn set(&mut self, index: usize, revoked: bool) -> Result<(), CoreError> {
        if index >= self.len {
            return Err(CoreError::StatusList(format!(
                "index {index} out of range (len {})",
                self.len
            )));
        }
        let byte = index / 8;
        let bit = 7 - (index % 8) as u8;
        if revoked {
            self.bits[byte] |= 1 << bit;
        } else {
            self.bits[byte] &= !(1 << bit);
        }
        Ok(())
    }

    /// Whether the revocation bit at `index` is set. Out-of-range is `false`.
    pub fn is_set(&self, index: usize) -> bool {
        if index >= self.len {
            return false;
        }
        let byte = index / 8;
        let bit = 7 - (index % 8) as u8;
        self.bits[byte] & (1 << bit) != 0
    }

    /// Encode the bitstring as GZIP + base64url (no padding).
    pub fn encode(&self) -> Result<String, CoreError> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&self.bits)
            .map_err(|e| CoreError::StatusList(format!("gzip write failed: {e}")))?;
        let compressed = encoder
            .finish()
            .map_err(|e| CoreError::StatusList(format!("gzip finish failed: {e}")))?;
        Ok(URL_SAFE_NO_PAD.encode(compressed))
    }

    /// Decode a GZIP + base64url bitstring back into a status list of `len`.
    pub fn decode(encoded: &str, len: usize) -> Result<Self, CoreError> {
        let compressed = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|e| CoreError::StatusList(format!("invalid base64url: {e}")))?;
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut bits = Vec::new();
        decoder
            .read_to_end(&mut bits)
            .map_err(|e| CoreError::StatusList(format!("gzip decode failed: {e}")))?;
        let needed = len.div_ceil(8);
        if bits.len() < needed {
            bits.resize(needed, 0);
        }
        Ok(StatusList2021 { bits, len })
    }

    /// The `credentialSubject` object of a StatusList2021 credential.
    pub fn credential_subject(&self, id: &str, purpose: &str) -> Result<Value, CoreError> {
        Ok(json!({
            "id": format!("{id}#list"),
            "type": "StatusList2021",
            "statusPurpose": purpose,
            "encodedList": self.encode()?,
        }))
    }

    /// Build a StatusList2021 credential wrapping this list.
    ///
    /// `id` is the canonical URL of the status list credential and `issuer_did`
    /// is the DID that will sign it.
    pub fn to_credential(
        &self,
        id: &str,
        issuer_did: &str,
        purpose: &str,
    ) -> Result<VerifiableCredential, CoreError> {
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        Ok(VerifiableCredential {
            context: vec![VC_CONTEXT_V1.to_string(), STATUS_LIST_CONTEXT.to_string()],
            id: id.to_string(),
            types: vec![
                "VerifiableCredential".to_string(),
                "StatusList2021Credential".to_string(),
            ],
            issuer: issuer_did.to_string(),
            issuance_date: now,
            expiration_date: None,
            credential_status: None,
            credential_subject: self.credential_subject(id, purpose)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_roundtrip() {
        let mut list = StatusList2021::new(1024);
        assert!(!list.is_set(42));
        list.set(42, true).unwrap();
        assert!(list.is_set(42));
        list.set(42, false).unwrap();
        assert!(!list.is_set(42));
    }

    #[test]
    fn out_of_range_set_errors() {
        let mut list = StatusList2021::new(8);
        assert!(list.set(8, true).is_err());
        assert!(!list.is_set(8));
    }

    #[test]
    fn encode_decode_preserves_bits() {
        let mut list = StatusList2021::new(2048);
        for i in [0usize, 1, 7, 8, 100, 1000, 2047] {
            list.set(i, true).unwrap();
        }
        let encoded = list.encode().unwrap();
        let decoded = StatusList2021::decode(&encoded, 2048).unwrap();
        for i in 0..2048 {
            assert_eq!(list.is_set(i), decoded.is_set(i), "mismatch at {i}");
        }
    }

    #[test]
    fn to_credential_has_encoded_list() {
        let list = StatusList2021::new(DEFAULT_STATUS_LIST_LEN);
        let vc = list
            .to_credential(
                "https://issuer.example/api/v1/status/1",
                "did:web:issuer.example",
                "revocation",
            )
            .unwrap();
        assert!(vc.types.contains(&"StatusList2021Credential".to_string()));
        assert_eq!(vc.credential_subject["statusPurpose"], "revocation");
        assert!(vc.credential_subject["encodedList"].is_string());
    }
}
