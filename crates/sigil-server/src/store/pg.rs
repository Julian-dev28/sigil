use async_trait::async_trait;
use serde_json::Value;
use sqlx::postgres::PgPool;
use sqlx::Row;
use uuid::Uuid;

use super::{CredentialRecord, CredentialSummary, Store, StoreError};

/// A Postgres-backed [`Store`] using runtime (non-macro) sqlx queries.
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    /// Wrap an existing connection pool.
    pub fn new(pool: PgPool) -> Self {
        PgStore { pool }
    }
}

fn db_err(e: sqlx::Error) -> StoreError {
    StoreError::Database(e.to_string())
}

fn record_from_row(row: &sqlx::postgres::PgRow) -> Result<CredentialRecord, StoreError> {
    let status_index: i32 = row.try_get("status_index").map_err(db_err)?;
    Ok(CredentialRecord {
        id: row.try_get("id").map_err(db_err)?,
        subject_id: row.try_get("subject_id").map_err(db_err)?,
        credential_type: row.try_get("credential_type").map_err(db_err)?,
        jwt: row.try_get("jwt").map_err(db_err)?,
        vc: row.try_get::<Value, _>("vc").map_err(db_err)?,
        status_list_id: row.try_get("status_list_id").map_err(db_err)?,
        status_index: status_index as i64,
        revoked: row.try_get("revoked").map_err(db_err)?,
        issued_at: row.try_get("issued_at").map_err(db_err)?,
        expires_at: row.try_get("expires_at").map_err(db_err)?,
    })
}

#[async_trait]
impl Store for PgStore {
    async fn insert_credential(&self, record: CredentialRecord) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO credentials \
             (id, subject_id, credential_type, jwt, vc, status_list_id, status_index, revoked, issued_at, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(record.id)
        .bind(&record.subject_id)
        .bind(&record.credential_type)
        .bind(&record.jwt)
        .bind(&record.vc)
        .bind(record.status_list_id)
        .bind(record.status_index as i32)
        .bind(record.revoked)
        .bind(record.issued_at)
        .bind(record.expires_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn get_credential(&self, id: Uuid) -> Result<Option<CredentialRecord>, StoreError> {
        let row = sqlx::query(
            "SELECT id, subject_id, credential_type, jwt, vc, status_list_id, status_index, \
             revoked, issued_at, expires_at FROM credentials WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(record_from_row).transpose()
    }

    async fn list_credentials(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<CredentialSummary>, i64), StoreError> {
        let total: i64 = sqlx::query("SELECT COUNT(*) AS count FROM credentials")
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?
            .try_get("count")
            .map_err(db_err)?;

        let rows = sqlx::query(
            "SELECT id, credential_type, subject_id, issued_at, revoked FROM credentials \
             ORDER BY issued_at DESC, id DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit.max(0))
        .bind(offset.max(0))
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let items = rows
            .iter()
            .map(|row| {
                Ok(CredentialSummary {
                    id: row.try_get("id").map_err(db_err)?,
                    credential_type: row.try_get("credential_type").map_err(db_err)?,
                    subject_id: row.try_get("subject_id").map_err(db_err)?,
                    issued_at: row.try_get("issued_at").map_err(db_err)?,
                    revoked: row.try_get("revoked").map_err(db_err)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

        Ok((items, total))
    }

    async fn revoke_credential(&self, id: Uuid) -> Result<Option<CredentialRecord>, StoreError> {
        let row = sqlx::query(
            "UPDATE credentials SET revoked = true WHERE id = $1 \
             RETURNING id, subject_id, credential_type, jwt, vc, status_list_id, status_index, \
             revoked, issued_at, expires_at",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(record_from_row).transpose()
    }

    async fn next_status_index(&self, list_id: i32) -> Result<i64, StoreError> {
        let next: i32 = sqlx::query(
            "SELECT COALESCE(MAX(status_index), -1) + 1 AS next FROM credentials \
             WHERE status_list_id = $1",
        )
        .bind(list_id)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?
        .try_get("next")
        .map_err(db_err)?;
        Ok(next as i64)
    }

    async fn revoked_indices(&self, list_id: i32) -> Result<Vec<i64>, StoreError> {
        let rows = sqlx::query(
            "SELECT status_index FROM credentials WHERE status_list_id = $1 AND revoked = true \
             ORDER BY status_index",
        )
        .bind(list_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter()
            .map(|row| {
                let idx: i32 = row.try_get("status_index").map_err(db_err)?;
                Ok(idx as i64)
            })
            .collect()
    }

    async fn upsert_status_list(
        &self,
        list_id: i32,
        purpose: &str,
        encoded_list: &str,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO status_lists (id, purpose, encoded_list, updated_at) \
             VALUES ($1, $2, $3, now()) \
             ON CONFLICT (id) DO UPDATE SET purpose = EXCLUDED.purpose, \
             encoded_list = EXCLUDED.encoded_list, updated_at = now()",
        )
        .bind(list_id)
        .bind(purpose)
        .bind(encoded_list)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }
}
