-- Sigil initial schema.
--
-- Stores issued credentials and the StatusList2021 revocation lists they are
-- enrolled in. All queries are executed at runtime (no compile-time `query!`
-- macros), so this schema is the single source of truth for column names.

CREATE TABLE IF NOT EXISTS credentials (
    id              uuid PRIMARY KEY,
    subject_id      text NOT NULL,
    credential_type text NOT NULL,
    jwt             text NOT NULL,
    vc              jsonb NOT NULL,
    status_list_id  integer NOT NULL DEFAULT 1,
    status_index    integer NOT NULL,
    revoked         boolean NOT NULL DEFAULT false,
    issued_at       timestamptz NOT NULL DEFAULT now(),
    expires_at      timestamptz
);

CREATE INDEX IF NOT EXISTS idx_credentials_subject_id ON credentials (subject_id);
CREATE INDEX IF NOT EXISTS idx_credentials_revoked ON credentials (revoked);
CREATE INDEX IF NOT EXISTS idx_credentials_status ON credentials (status_list_id, status_index);

CREATE TABLE IF NOT EXISTS status_lists (
    id           serial PRIMARY KEY,
    purpose      text NOT NULL,
    encoded_list text NOT NULL,
    updated_at   timestamptz NOT NULL DEFAULT now()
);
