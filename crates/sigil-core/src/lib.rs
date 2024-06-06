//! # sigil-core
//!
//! Core building blocks for the Sigil Verifiable Credentials platform:
//!
//! - Ed25519 [`KeyPair`]s with PKCS#8, `did:key` and multibase encodings.
//! - [`did:web`](did::did_web) identifiers and DID documents, plus a
//!   [`DidResolver`] abstraction.
//! - The [`VerifiableCredential`] data model (W3C VC Data Model 1.1).
//! - JWT-VC [issuance](issue_jwt_vc) and [verification](verify_jwt_vc) using
//!   EdDSA (`jsonwebtoken`).
//! - [`StatusList2021`] revocation bitstrings.
//!
//! The crate is transport- and storage-agnostic; the `sigil-server` crate wires
//! it into an HTTP service.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod credential;
pub mod did;
pub mod error;
pub mod jwt;
pub mod keys;
pub mod statuslist;

pub use credential::{CredentialStatus, VerifiableCredential, VC_CONTEXT_V1};
pub use did::{did_document, did_key_to_verifying_key, did_web, Did, DidResolver, StaticResolver};
pub use error::CoreError;
pub use jwt::{issue_jwt_vc, verify_jwt_vc, VcJwtClaims, VerificationResult};
pub use keys::{public_key_multibase, verifying_key_from_multibase, KeyPair};
pub use statuslist::{StatusList2021, DEFAULT_STATUS_LIST_LEN};

// Re-export the concrete Ed25519 verifying key type used across the API so
// downstream crates need not depend on `ed25519-dalek` directly.
pub use ed25519_dalek::VerifyingKey;
