//! JWT-VC issuance and verification using EdDSA (Ed25519).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{
    decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};

use crate::credential::VerifiableCredential;
use crate::did::DidResolver;
use crate::error::CoreError;
use crate::keys::KeyPair;

/// The JWT-VC claim set: standard registered claims plus the `vc` object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcJwtClaims {
    /// Issuer DID (`iss`).
    pub iss: String,
    /// Subject DID (`sub`).
    pub sub: String,
    /// Not-before, seconds since the Unix epoch (`nbf`).
    pub nbf: i64,
    /// Expiry, seconds since the Unix epoch (`exp`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
    /// JWT id, mirrors the credential id (`jti`).
    pub jti: String,
    /// The embedded Verifiable Credential.
    pub vc: VerifiableCredential,
}

/// The outcome of verifying a JWT-VC signature and temporal validity.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether the Ed25519 signature verified against the resolved key.
    pub signature: bool,
    /// Whether the credential is currently within its validity window.
    pub not_expired: bool,
    /// The decoded claim set.
    pub claims: VcJwtClaims,
}

/// Encode a credential as a signed JWT-VC (EdDSA / Ed25519).
///
/// The JWT header carries `kid = <issuer_did>#key-1`; registered claims are
/// derived from the credential (`iss`, `sub`, `nbf`, `exp`, `jti`).
pub fn issue_jwt_vc(
    vc: &VerifiableCredential,
    issuer_did: &str,
    key: &KeyPair,
) -> Result<String, CoreError> {
    let sub = vc
        .subject_id()
        .ok_or_else(|| CoreError::Credential("credentialSubject.id is required".into()))?
        .to_string();
    let nbf = vc.issuance_timestamp().ok_or_else(|| {
        CoreError::Credential("issuanceDate must be a valid RFC 3339 date".into())
    })?;

    let claims = VcJwtClaims {
        iss: issuer_did.to_string(),
        sub,
        nbf,
        exp: vc.expiration_timestamp(),
        jti: vc.id.clone(),
        vc: vc.clone(),
    };

    let mut header = Header::new(Algorithm::EdDSA);
    header.typ = Some("JWT".to_string());
    header.kid = Some(format!("{issuer_did}#key-1"));

    let encoding = EncodingKey::from_ed_der(&key.to_pkcs8_der());
    encode(&header, &claims, &encoding).map_err(|e| CoreError::Jwt(format!("encode failed: {e}")))
}

/// Verify a JWT-VC's signature and expiry.
///
/// The signing key is resolved from the header `kid` (its DID part) via the
/// supplied [`DidResolver`]. The signature and temporal checks are reported
/// independently so callers can surface fine-grained results; a bad signature
/// yields `signature = false` rather than an error, provided the token is
/// otherwise structurally sound.
pub fn verify_jwt_vc(
    jwt: &str,
    resolver: &dyn DidResolver,
) -> Result<VerificationResult, CoreError> {
    let header = decode_header(jwt).map_err(|e| CoreError::Jwt(format!("invalid header: {e}")))?;
    if header.alg != Algorithm::EdDSA {
        return Err(CoreError::Jwt(format!(
            "unsupported algorithm: {:?}",
            header.alg
        )));
    }
    let kid = header
        .kid
        .ok_or_else(|| CoreError::Jwt("missing kid header".into()))?;
    let issuer_did = kid.split('#').next().unwrap_or(&kid).to_string();

    let verifying_key = resolver.resolve_key(&issuer_did)?;

    // Decode the claim set without verifying the signature so we can always
    // report the credential contents, then verify the signature separately.
    let claims = decode_claims_unverified(jwt)?;

    let decoding = DecodingKey::from_ed_der(verifying_key.as_bytes());
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.validate_exp = false;
    validation.validate_aud = false;
    validation.required_spec_claims.clear();

    let signature = match decode::<VcJwtClaims>(jwt, &decoding, &validation) {
        Ok(_) => true,
        Err(e) => match e.kind() {
            ErrorKind::InvalidSignature => false,
            // Any other decode failure at this point (the payload already
            // parsed above) indicates the token cannot be trusted.
            _ => false,
        },
    };

    let now = Utc::now().timestamp();
    let not_expired = claims.exp.map(|exp| exp > now).unwrap_or(true);

    Ok(VerificationResult {
        signature,
        not_expired,
        claims,
    })
}

/// Decode the JWT payload (claims) without verifying the signature.
fn decode_claims_unverified(jwt: &str) -> Result<VcJwtClaims, CoreError> {
    let mut parts = jwt.split('.');
    let (_header, payload) = match (parts.next(), parts.next(), parts.next()) {
        (Some(h), Some(p), Some(_sig)) => (h, p),
        _ => {
            return Err(CoreError::Jwt(
                "malformed JWT: expected three segments".into(),
            ))
        }
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| CoreError::Jwt(format!("invalid base64url payload: {e}")))?;
    serde_json::from_slice(&bytes).map_err(|e| CoreError::Jwt(format!("invalid claims: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::VerifiableCredential;
    use crate::did::StaticResolver;
    use chrono::Duration;
    use serde_json::json;

    fn issuer() -> (KeyPair, String) {
        let kp = KeyPair::generate();
        (kp, "did:web:issuer.example".to_string())
    }

    #[test]
    fn issue_then_verify_roundtrip() {
        let (kp, did) = issuer();
        let vc = VerifiableCredential::new(
            &did,
            "did:key:z6MkSubject",
            "EmploymentCredential",
            json!({ "role": "Engineer" }),
            Some(Duration::days(365)),
        );
        let jwt = issue_jwt_vc(&vc, &did, &kp).expect("issue");

        let resolver = StaticResolver::with_registered(&did, kp.verifying_key());
        let result = verify_jwt_vc(&jwt, &resolver).expect("verify");

        assert!(result.signature);
        assert!(result.not_expired);
        assert_eq!(result.claims.iss, did);
        assert_eq!(result.claims.sub, "did:key:z6MkSubject");
        assert_eq!(result.claims.vc.credential_subject["role"], "Engineer");
    }

    #[test]
    fn tampered_signature_fails() {
        let (kp, did) = issuer();
        let vc = VerifiableCredential::new(
            &did,
            "did:key:z6MkSubject",
            "TestCredential",
            json!({}),
            Some(Duration::days(1)),
        );
        let jwt = issue_jwt_vc(&vc, &did, &kp).expect("issue");

        // Verify against an unrelated key: signature must not validate.
        let other = KeyPair::generate();
        let resolver = StaticResolver::with_registered(&did, other.verifying_key());
        let result = verify_jwt_vc(&jwt, &resolver).expect("verify");
        assert!(!result.signature);
    }

    #[test]
    fn expired_credential_detected() {
        let (kp, did) = issuer();
        let vc = VerifiableCredential::new(
            &did,
            "did:key:z6MkSubject",
            "TestCredential",
            json!({}),
            Some(Duration::days(-1)),
        );
        let jwt = issue_jwt_vc(&vc, &did, &kp).expect("issue");

        let resolver = StaticResolver::with_registered(&did, kp.verifying_key());
        let result = verify_jwt_vc(&jwt, &resolver).expect("verify");
        assert!(result.signature);
        assert!(!result.not_expired);
    }
}
