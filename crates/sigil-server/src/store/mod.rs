use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

mod memory;
mod pg;

pub use memory::MemoryStore;
pub use pg::PgStore;

/// The default status list every credential is enrolled in.
pub const DEFAULT_STATUS_LIST_ID: i32 = 1;
/// The default status list purpose.
pub const DEFAULT_STATUS_PURPOSE: &str = "revocation";

/// A stored credential record.
#[derive(Debug, Clone)]
pub struct CredentialRecord {
    /// Credential id (the UUID embedded in the credential's `urn:uuid:` id).
    pub id: Uuid,
    /// The subject DID.
    pub subject_id: String,
    /// The credential's extra type, e.g. `EmploymentCredential`.
    pub credential_type: String,
    /// The signed JWT-VC.
    pub jwt: String,
    /// The credential object as JSON.
    pub vc: Value,
    /// The status list this credential is enrolled in.
    pub status_list_id: i32,
    /// The credential's bit index within the status list.
    pub status_index: i64,
    /// Whether the credential has been revoked.
    pub revoked: bool,
    /// Issuance time.
    pub issued_at: DateTime<Utc>,
    /// Expiration time, if any.
    pub expires_at: Option<DateTime<Utc>>,
}

/// A summary row returned by list queries.
#[derive(Debug, Clone)]
pub struct CredentialSummary {
    /// Credential id.
    pub id: Uuid,
    /// The credential's extra type.
    pub credential_type: String,
    /// The subject DID.
    pub subject_id: String,
    /// Issuance time.
    pub issued_at: DateTime<Utc>,
    /// Whether the credential has been revoked.
    pub revoked: bool,
}

/// Errors returned by [`Store`] implementations.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// A database operation failed.
    #[error("database error: {0}")]
    Database(String),

    /// A stored value could not be (de)serialized.
    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Persistence abstraction for credentials and status lists.
///
/// Two implementations are provided: [`PgStore`] (Postgres) and
/// [`MemoryStore`] (a process-local fallback used when no database is
/// configured).
#[async_trait]
pub trait Store: Send + Sync {
    /// Persist a newly issued credential.
    async fn insert_credential(&self, record: CredentialRecord) -> Result<(), StoreError>;

    /// Fetch a credential by id.
    async fn get_credential(&self, id: Uuid) -> Result<Option<CredentialRecord>, StoreError>;

    /// List credentials most-recent-first, returning a page and the total count.
    async fn list_credentials(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<CredentialSummary>, i64), StoreError>;

    /// Mark a credential revoked, returning the updated record (or `None`).
    async fn revoke_credential(&self, id: Uuid) -> Result<Option<CredentialRecord>, StoreError>;

    /// Allocate the next free bit index within a status list.
    async fn next_status_index(&self, list_id: i32) -> Result<i64, StoreError>;

    /// The bit indices of all revoked credentials within a status list.
    async fn revoked_indices(&self, list_id: i32) -> Result<Vec<i64>, StoreError>;

    /// Insert or update the persisted encoded status list.
    async fn upsert_status_list(
        &self,
        list_id: i32,
        purpose: &str,
        encoded_list: &str,
    ) -> Result<(), StoreError>;
}
