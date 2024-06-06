//! Ed25519 key pairs and their PKCS#8, `did:key` and multibase encodings.

use ed25519_dalek::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;

use crate::error::CoreError;

/// Multicodec prefix for an Ed25519 public key (`ed25519-pub`, 0xed 0x01),
/// as used by the `did:key` method and `publicKeyMultibase` encodings.
pub const ED25519_MULTICODEC_PREFIX: [u8; 2] = [0xed, 0x01];

/// An Ed25519 key pair used to sign and identify credentials.
///
/// Wraps [`ed25519_dalek::SigningKey`] and provides the DID / multibase
/// encodings that the rest of Sigil relies on.
#[derive(Clone)]
pub struct KeyPair {
    signing: SigningKey,
}

impl KeyPair {
    /// Generate a fresh random Ed25519 key pair using the OS CSPRNG.
    pub fn generate() -> KeyPair {
        let signing = SigningKey::generate(&mut OsRng);
        KeyPair { signing }
    }

    /// Construct a key pair from its wrapped signing key.
    pub fn from_signing_key(signing: SigningKey) -> KeyPair {
        KeyPair { signing }
    }

    /// Parse a key pair from a PKCS#8 DER encoded private key.
    pub fn from_pkcs8_der(bytes: &[u8]) -> Result<KeyPair, CoreError> {
        let signing = SigningKey::from_pkcs8_der(bytes)
            .map_err(|e| CoreError::Key(format!("invalid PKCS#8 DER: {e}")))?;
        Ok(KeyPair { signing })
    }

    /// Serialize the private key as PKCS#8 DER bytes.
    pub fn to_pkcs8_der(&self) -> Vec<u8> {
        // `SigningKey` always produces valid PKCS#8, so this cannot fail in
        // practice; fall back to an empty vec only defensively.
        self.signing
            .to_pkcs8_der()
            .map(|doc| doc.as_bytes().to_vec())
            .unwrap_or_default()
    }

    /// The Ed25519 verifying (public) key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing.verifying_key()
    }

    /// Borrow the underlying signing key.
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing
    }

    /// The public key encoded as multibase (`z` + base58btc) over the
    /// multicodec-prefixed 32 byte Ed25519 public key.
    pub fn public_key_multibase(&self) -> String {
        public_key_multibase(&self.verifying_key())
    }

    /// The `did:key` identifier for this key pair.
    pub fn did_key(&self) -> String {
        format!("did:key:{}", self.public_key_multibase())
    }
}

impl std::fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print private key material.
        f.debug_struct("KeyPair")
            .field("did_key", &self.did_key())
            .finish()
    }
}

/// Encode a verifying key as multibase (`z` + base58btc) with the Ed25519
/// multicodec prefix.
pub fn public_key_multibase(key: &VerifyingKey) -> String {
    let mut bytes = Vec::with_capacity(2 + 32);
    bytes.extend_from_slice(&ED25519_MULTICODEC_PREFIX);
    bytes.extend_from_slice(key.as_bytes());
    format!("z{}", bs58::encode(bytes).into_string())
}

/// Decode an Ed25519 `publicKeyMultibase` value back into a verifying key.
pub fn verifying_key_from_multibase(multibase: &str) -> Result<VerifyingKey, CoreError> {
    let stripped = multibase.strip_prefix('z').ok_or_else(|| {
        CoreError::Key("multibase value must use the 'z' (base58btc) prefix".into())
    })?;
    let decoded = bs58::decode(stripped)
        .into_vec()
        .map_err(|e| CoreError::Key(format!("invalid base58btc: {e}")))?;
    if decoded.len() != 34 || decoded[0..2] != ED25519_MULTICODEC_PREFIX {
        return Err(CoreError::Key(
            "not a multicodec-prefixed Ed25519 public key".into(),
        ));
    }
    let raw: [u8; 32] = decoded[2..34]
        .try_into()
        .map_err(|_| CoreError::Key("invalid public key length".into()))?;
    VerifyingKey::from_bytes(&raw).map_err(|e| CoreError::Key(format!("invalid Ed25519 key: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkcs8_roundtrip_preserves_key() {
        let kp = KeyPair::generate();
        let der = kp.to_pkcs8_der();
        assert!(!der.is_empty());
        let restored = KeyPair::from_pkcs8_der(&der).expect("parse DER");
        assert_eq!(kp.did_key(), restored.did_key());
        assert_eq!(
            kp.verifying_key().as_bytes(),
            restored.verifying_key().as_bytes()
        );
    }

    #[test]
    fn did_key_has_expected_shape() {
        let kp = KeyPair::generate();
        let did = kp.did_key();
        assert!(did.starts_with("did:key:z6Mk"), "unexpected did:key: {did}");
    }

    #[test]
    fn multibase_roundtrip() {
        let kp = KeyPair::generate();
        let mb = kp.public_key_multibase();
        let recovered = verifying_key_from_multibase(&mb).expect("decode multibase");
        assert_eq!(recovered.as_bytes(), kp.verifying_key().as_bytes());
    }
}
