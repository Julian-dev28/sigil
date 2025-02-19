# Architecture

This document describes how Sigil is put together: its components, the data that
flows between them, the trust and key-management model, storage, and the points
where the system is meant to be extended. For the *why* behind specific choices,
see [`docs/DESIGN.md`](./docs/DESIGN.md).

## Components

Sigil is a small Rust workspace plus a TypeScript UI.

### `sigil-core`

A pure library with no I/O and no framework dependencies. It owns the domain
model and all cryptography:

- **`KeyPair`** — wraps an `ed25519_dalek::SigningKey`. Generates keys, imports and
  exports PKCS#8 DER, and derives the multibase-encoded public key and `did:key`.
- **`VerifiableCredential` / `CredentialStatus`** — serde types that match the W3C
  VC Data Model 1.1 JSON exactly, including the `@context` field and the
  `StatusList2021Entry` status block.
- **JWT-VC issuance and verification** — `issue_jwt_vc` builds and signs the JWT
  (EdDSA, `typ: JWT`, `kid: <issuerDid>#key-1`); `verify_jwt_vc` decodes the header,
  resolves the issuer key through a `DidResolver`, and checks the Ed25519 signature
  plus `nbf`/`exp`.
- **`DidResolver` trait** — the seam between verification and the outside world.
  `did:key` resolves from the identifier itself; `did:web` resolution is supplied
  by the server.
- **`StatusList2021`** — a bitstring with `set`/`is_set`/`encode`, where `encode`
  produces the gzip + base64url form embedded in a status list credential.

Keeping `sigil-core` free of I/O makes the cryptography unit-testable in isolation
and lets it be reused by other front ends (a CLI, a batch issuer) without dragging
in axum or sqlx.

### `sigil-server`

An [axum](https://github.com/tokio-rs/axum) service that exposes the HTTP API and
owns everything stateful:

- **Config** is read from the environment into a typed struct (`DATABASE_URL`,
  `SIGIL_ISSUER_HOST`, `SIGIL_BIND`, `SIGIL_ISSUER_KEY_PKCS8_B64`, `RUST_LOG`).
- **Issuer identity** is a single Ed25519 key loaded from
  `SIGIL_ISSUER_KEY_PKCS8_B64`, or generated at startup with a loud warning if
  absent. The issuer's `did:web` is derived from `SIGIL_ISSUER_HOST`.
- **`Store` trait** abstracts persistence with two implementations: `PgStore`
  (sqlx + Postgres) and `MemoryStore` (an in-process map). The server selects one
  at startup based on whether `DATABASE_URL` is set.
- **`AppState`** carries an `Arc<dyn Store>`, the issuer `KeyPair`, and the issuer DID
  into every handler.
- **Handlers** implement the full API and translate `sigil-core` and storage errors
  into the shared `ApiError` JSON envelope.

### Web UI (`web/`)

A Vite + React + TypeScript app with three flows — issuer, wallet (holder), and
verifier — over a typed fetch client that mirrors the domain model. It is a
reference consumer of the API, not a privileged component: everything it does is
expressible with `curl`.

## Data flow

### Issuance

1. A client posts `{ subjectId, type, claims, expiresInDays }`.
2. The server validates the subject DID syntax, the credential type, the claims
   object, and the expiry bound.
3. The server allocates a StatusList2021 index for the credential from the store.
4. `sigil-core` assembles the `VerifiableCredential` (context, id, type, issuer,
   issuance/expiration dates, `credentialStatus`, `credentialSubject`) and signs it
   into a JWT-VC.
5. The record — id, subject, type, JWT, VC JSON, status list id/index, timestamps —
   is persisted.
6. The response returns the id, the JWT, and the credential object.

### Verification

1. A client posts `{ jwt }`.
2. `sigil-core` decodes the JWT header, resolves the issuer's verifying key, and
   checks the signature and time bounds. This is self-contained and needs no
   database.
3. The server reads the credential's `credentialStatus` and looks up the referenced
   StatusList2021 bit to determine revocation.
4. The three checks — `signature`, `notExpired`, `notRevoked` — are returned
   independently, along with an overall `valid` flag (their conjunction).

Reporting each check separately is deliberate: a verifier may, for example, accept a
recently expired credential for a grace period while still rejecting a revoked one.

## Trust model

- **Issuer authority.** Trust is anchored in the issuer's `did:web` document, served
  over HTTPS at `/.well-known/did.json`. A verifier that trusts the domain trusts the
  keys the domain publishes. Sigil does not impose a trust registry; relying parties
  decide which issuer DIDs they accept.
- **Subjects and holders** are identified by `did:key`, which is self-certifying — the
  key is the identifier — so no resolution infrastructure is required for subjects.
- **No holder binding by default.** The reference flow issues bearer credentials. Proof
  of possession (holder binding, OID4VP) is on the roadmap and intentionally out of the
  base model to keep the primitives clear.
- **Revocation is issuer-controlled** and published as a StatusList2021 credential that
  any party can fetch and check without contacting the issuer per verification.

## Key management

- The issuer key is an Ed25519 signing key held in memory as PKCS#8 DER, sourced from
  `SIGIL_ISSUER_KEY_PKCS8_B64`.
- If no key is configured the server generates one and logs a warning. This is
  convenient for local development and unacceptable for production: an ephemeral key
  invalidates every previously issued credential on restart.
- Keys never touch the database and are excluded from logs. `.der`, `.pem`, and `.key`
  files are gitignored.
- Key rotation is not yet automated; the DID document currently advertises a single
  `key-1`. Versioned DID documents and rotation are planned (see the roadmap).

## Storage

The `credentials` table holds one row per issued credential: `id`, `subject_id`,
`credential_type`, the signed `jwt`, the `vc` JSONB, `status_list_id` /
`status_index`, a `revoked` flag, and `issued_at` / `expires_at`. A `status_lists`
table holds the encoded bitstrings. Indexes on `subject_id` and `revoked` support the
common list and revocation queries.

`MemoryStore` mirrors the same semantics in process so the demo and the integration
tests run without Postgres. Migrations live in `migrations/` and are embedded and
applied at startup via `sqlx::migrate!()` when a database is configured.

## Threat considerations

- **Signature forgery** is bounded by Ed25519; verification rejects any JWT that does
  not verify against the resolved issuer key.
- **Tampering** with the payload breaks the signature and is caught at the
  `signature` check.
- **Replay / staleness** is mitigated by `exp`/`nbf` and by revocation; verifiers
  should always consult all three checks rather than the `valid` flag alone if they
  apply custom policy.
- **Status list privacy.** StatusList2021 deliberately hides which credential a bit
  refers to by batching many credentials into one list, limiting the correlation a
  status fetch reveals.
- **Issuer key compromise** is the highest-impact failure; it is mitigated
  operationally (KMS/HSM backing, rotation) rather than in code today. See
  [`SECURITY.md`](./SECURITY.md).

## Extensibility

- **New credential formats.** Because issuance and verification are functions over the
  domain model, an SD-JWT-VC or LD-proof path can be added alongside JWT-VC without
  changing the storage or HTTP layers.
- **New DID methods.** Implement `DidResolver`; the verifier is agnostic to how a key
  is obtained.
- **New storage backends.** Implement `Store`; handlers depend only on the trait.
- **Presentation flows.** OID4VP and holder binding slot in at the API layer and reuse
  the existing verification core.
