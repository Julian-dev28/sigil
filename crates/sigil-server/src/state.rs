use std::sync::Arc;

use sigil_core::{did_web, KeyPair, StaticResolver};

use crate::config::public_base_url;
use crate::store::Store;

/// Shared application state handed to every request handler.
pub struct AppState {
    /// The active persistence backend.
    pub store: Arc<dyn Store>,
    /// The issuer signing key.
    pub issuer_key: KeyPair,
    /// The issuer `did:web` identifier.
    pub issuer_did: String,
    /// The public host (and optional port) the issuer is served from.
    pub issuer_host: String,
    /// The public base URL used to build absolute identifiers.
    pub base_url: String,
}

impl AppState {
    /// Assemble application state for the given issuer host and signing key.
    ///
    /// The issuer `did:web` and public base URL are both derived from `host`
    /// so the DID document, credential identifiers and status list URLs agree.
    pub fn new(store: Arc<dyn Store>, issuer_key: KeyPair, issuer_host: String) -> Self {
        let issuer_did = did_web(&issuer_host);
        let base_url = public_base_url(&issuer_host);
        AppState {
            store,
            issuer_key,
            issuer_did,
            issuer_host,
            base_url,
        }
    }

    /// A DID resolver that resolves the issuer's own `did:web` plus any
    /// `did:key`, sufficient for verifying credentials this service issues.
    pub fn resolver(&self) -> StaticResolver {
        StaticResolver::with_registered(&self.issuer_did, self.issuer_key.verifying_key())
    }

    /// The canonical URL of the status list credential with the given id.
    pub fn status_list_url(&self, list_id: i32) -> String {
        format!("{}/api/v1/status/{list_id}", self.base_url)
    }
}

/// Convenience alias for the shared, reference-counted state.
pub type SharedState = Arc<AppState>;
