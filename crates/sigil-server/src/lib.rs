//! # sigil-server
//!
//! The HTTP service for the Sigil Verifiable Credentials platform. It wires the
//! transport-agnostic [`sigil_core`] primitives into an [`axum`] application:
//! issuance, verification, listing, revocation and StatusList2021 endpoints,
//! plus `did:web` and OID4VCI discovery documents.
//!
//! The crate exposes [`build_router`] and [`build_state`] so the same wiring can
//! be exercised from integration tests and the `sigil-server` binary.
#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub mod config;
pub mod error;
pub mod handlers;
pub mod state;
pub mod store;

pub use config::Config;
pub use state::{AppState, SharedState};

use store::{MemoryStore, PgStore, Store};

/// Build the application router for the given shared state.
///
/// The router mounts the well-known discovery documents at the root and the
/// versioned API under `/api/v1`, wrapped in request tracing and a permissive
/// development CORS policy.
pub fn build_router(state: SharedState) -> Router {
    let api = Router::new()
        .route("/credentials/issue", post(handlers::issue))
        .route("/credentials/verify", post(handlers::verify))
        .route("/credentials", get(handlers::list_credentials))
        .route("/credentials/:id", get(handlers::get_credential))
        .route("/credentials/:id/revoke", post(handlers::revoke))
        .route("/status/:list_id", get(handlers::status_list));

    Router::new()
        .route("/health", get(handlers::health))
        .route("/.well-known/did.json", get(handlers::did_document))
        .route(
            "/.well-known/openid-credential-issuer",
            get(handlers::oid4vci_metadata),
        )
        .nest("/api/v1", api)
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
        .with_state(state)
}

/// A development CORS policy.
///
/// Permissive by default so the bundled web client (served from a different
/// origin) works out of the box; tighten this via a reverse proxy or a stricter
/// [`CorsLayer`] before exposing the service publicly.
fn cors_layer() -> CorsLayer {
    CorsLayer::permissive()
}

/// Assemble application state from configuration.
///
/// When `DATABASE_URL` is set a Postgres pool is opened and migrations are
/// applied; otherwise an in-memory store is used so the service runs with no
/// external dependencies. A missing issuer key results in an ephemeral key and
/// a warning.
pub async fn build_state(config: &Config) -> anyhow::Result<SharedState> {
    let (issuer_key, generated) = config.load_or_generate_key();
    if generated {
        tracing::warn!(
            "issuer signing key was generated in-process; set SIGIL_ISSUER_KEY_PKCS8_B64 \
             for a stable did:web across restarts"
        );
    }

    let store: Arc<dyn Store> = match &config.database_url {
        Some(url) => {
            let pool = PgPoolOptions::new()
                .max_connections(10)
                .connect(url)
                .await?;
            sqlx::migrate!("../../migrations").run(&pool).await?;
            tracing::info!("connected to postgres and applied migrations");
            Arc::new(PgStore::new(pool))
        }
        None => {
            tracing::warn!(
                "DATABASE_URL is not set; using the in-memory store (data is not persisted)"
            );
            Arc::new(MemoryStore::new())
        }
    };

    let state = AppState::new(store, issuer_key, config.issuer_host.clone());
    Ok(Arc::new(state))
}

/// Initialise tracing/logging from `RUST_LOG` (defaulting to a sensible level).
pub fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,sigil_server=debug,tower_http=info"));
    fmt().with_env_filter(filter).with_target(false).init();
}
