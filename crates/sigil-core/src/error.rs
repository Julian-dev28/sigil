//! Error types produced by the Sigil core library.

use thiserror::Error;

/// Errors produced by the Sigil core library.
#[derive(Debug, Error)]
pub enum CoreError {
    /// A cryptographic key could not be parsed, generated or used.
    #[error("key error: {0}")]
    Key(String),

    /// A DID string or DID document was malformed or unsupported.
    #[error("did error: {0}")]
    Did(String),

    /// A JWT could not be encoded, decoded or was structurally invalid.
    #[error("jwt error: {0}")]
    Jwt(String),

    /// The referenced verification key or DID could not be resolved.
    #[error("resolution error: {0}")]
    Resolution(String),

    /// A credential, claim set or status list was semantically invalid.
    #[error("credential error: {0}")]
    Credential(String),

    /// A status list bitstring could not be encoded or decoded.
    #[error("status list error: {0}")]
    StatusList(String),

    /// A value could not be (de)serialized to/from JSON.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
