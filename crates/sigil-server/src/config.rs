use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sigil_core::KeyPair;

/// Runtime configuration, assembled from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Postgres connection string. When absent, an in-memory store is used.
    pub database_url: Option<String>,
    /// Public host (and optional port) the issuer is reachable at.
    pub issuer_host: String,
    /// Socket address the HTTP server binds to.
    pub bind: String,
    /// Optional base64 PKCS#8 DER issuer signing key.
    pub issuer_key_b64: Option<String>,
}

impl Config {
    /// Load configuration from the process environment, applying defaults.
    pub fn from_env() -> Self {
        Config {
            database_url: non_empty_env("DATABASE_URL"),
            issuer_host: non_empty_env("SIGIL_ISSUER_HOST")
                .unwrap_or_else(|| "localhost:8080".to_string()),
            bind: non_empty_env("SIGIL_BIND").unwrap_or_else(|| "0.0.0.0:8080".to_string()),
            issuer_key_b64: non_empty_env("SIGIL_ISSUER_KEY_PKCS8_B64"),
        }
    }

    /// The public base URL (scheme + host) used to build absolute identifiers.
    ///
    /// Loopback hosts default to `http`, everything else to `https`.
    pub fn public_base_url(&self) -> String {
        public_base_url(&self.issuer_host)
    }

    /// Load the configured issuer key, or generate a fresh one.
    ///
    /// Returns the key pair and whether it was freshly generated (so the caller
    /// can warn that it is ephemeral).
    pub fn load_or_generate_key(&self) -> (KeyPair, bool) {
        if let Some(b64) = &self.issuer_key_b64 {
            match STANDARD
                .decode(b64.trim())
                .map_err(|e| e.to_string())
                .and_then(|der| KeyPair::from_pkcs8_der(&der).map_err(|e| e.to_string()))
            {
                Ok(key) => return (key, false),
                Err(err) => {
                    tracing::error!(
                        error = %err,
                        "SIGIL_ISSUER_KEY_PKCS8_B64 could not be parsed; generating an ephemeral key"
                    );
                }
            }
        }
        (KeyPair::generate(), true)
    }
}

/// Derive the public base URL (scheme + host) for an issuer host.
///
/// Loopback hosts default to `http`, everything else to `https`. Shared by
/// [`Config::public_base_url`] and the application state so both agree.
pub fn public_base_url(host: &str) -> String {
    let scheme = if host.starts_with("localhost")
        || host.starts_with("127.0.0.1")
        || host.starts_with("[::1]")
    {
        "http"
    } else {
        "https"
    };
    format!("{scheme}://{host}")
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}
