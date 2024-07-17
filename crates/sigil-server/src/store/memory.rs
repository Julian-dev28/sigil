use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use uuid::Uuid;

use super::{CredentialRecord, CredentialSummary, Store, StoreError};

#[derive(Default)]
struct Inner {
    credentials: HashMap<Uuid, CredentialRecord>,
    status_lists: HashMap<i32, (String, String)>,
}

/// A process-local, in-memory [`Store`] used when no database is configured.
///
/// State is lost on restart; intended for demos, tests and local development.
#[derive(Default)]
pub struct MemoryStore {
    inner: Mutex<Inner>,
}

impl MemoryStore {
    /// Create an empty store.
    pub fn new() -> Self {
        MemoryStore::default()
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Inner>, StoreError> {
        self.inner
            .lock()
            .map_err(|_| StoreError::Database("in-memory store lock poisoned".into()))
    }
}

#[async_trait]
impl Store for MemoryStore {
    async fn insert_credential(&self, record: CredentialRecord) -> Result<(), StoreError> {
        let mut inner = self.lock()?;
        inner.credentials.insert(record.id, record);
        Ok(())
    }

    async fn get_credential(&self, id: Uuid) -> Result<Option<CredentialRecord>, StoreError> {
        let inner = self.lock()?;
        Ok(inner.credentials.get(&id).cloned())
    }

    async fn list_credentials(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<CredentialSummary>, i64), StoreError> {
        let inner = self.lock()?;
        let mut records: Vec<&CredentialRecord> = inner.credentials.values().collect();
        records.sort_by(|a, b| b.issued_at.cmp(&a.issued_at).then(b.id.cmp(&a.id)));
        let total = records.len() as i64;
        let items = records
            .into_iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .map(|r| CredentialSummary {
                id: r.id,
                credential_type: r.credential_type.clone(),
                subject_id: r.subject_id.clone(),
                issued_at: r.issued_at,
                revoked: r.revoked,
            })
            .collect();
        Ok((items, total))
    }

    async fn revoke_credential(&self, id: Uuid) -> Result<Option<CredentialRecord>, StoreError> {
        let mut inner = self.lock()?;
        match inner.credentials.get_mut(&id) {
            Some(record) => {
                record.revoked = true;
                Ok(Some(record.clone()))
            }
            None => Ok(None),
        }
    }

    async fn next_status_index(&self, list_id: i32) -> Result<i64, StoreError> {
        let inner = self.lock()?;
        let next = inner
            .credentials
            .values()
            .filter(|r| r.status_list_id == list_id)
            .map(|r| r.status_index)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        Ok(next)
    }

    async fn revoked_indices(&self, list_id: i32) -> Result<Vec<i64>, StoreError> {
        let inner = self.lock()?;
        let mut indices: Vec<i64> = inner
            .credentials
            .values()
            .filter(|r| r.status_list_id == list_id && r.revoked)
            .map(|r| r.status_index)
            .collect();
        indices.sort_unstable();
        Ok(indices)
    }

    async fn upsert_status_list(
        &self,
        list_id: i32,
        purpose: &str,
        encoded_list: &str,
    ) -> Result<(), StoreError> {
        let mut inner = self.lock()?;
        inner
            .status_lists
            .insert(list_id, (purpose.to_string(), encoded_list.to_string()));
        Ok(())
    }
}
