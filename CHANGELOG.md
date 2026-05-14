# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Reference terraform skeleton for an ECS Fargate + RDS deployment.

### Changed
- Verify responses now always return HTTP 200 with per-check results, including for
  invalid credentials, so callers can inspect individual checks.

## [0.5.0] - 2026-05-14

### Added
- `POST /api/v1/credentials/:id/revoke` and StatusList2021 bit updates wired end to end.
- Web UI: revoke action on the wallet credential detail view.
- Integration test covering revoke-then-verify (`notRevoked` flips to `false`).

### Changed
- Verification now reports `signature`, `notExpired`, and `notRevoked` independently
  with an aggregate `valid` flag, replacing the previous single boolean.
- Upgraded axum to 0.7 and refreshed the tower-http middleware stack.

### Fixed
- Reject JWT-VCs whose `kid` does not match the resolved issuer verification method.

## [0.4.0] - 2025-09-02

### Added
- `Store` trait with `PgStore` and `MemoryStore`; the server now starts without a
  database using an in-memory fallback.
- OID4VCI issuer metadata at `/.well-known/openid-credential-issuer`.
- `docs/openapi.yaml` describing every endpoint and schema.

### Changed
- Issuer key is now loaded from `SIGIL_ISSUER_KEY_PKCS8_B64` and only generated (with a
  warning) when unset.
- Consolidated error handling behind a shared `ApiError` JSON envelope.

### Fixed
- Correct multicodec prefix (`0xed01`) when deriving `did:key` from an Ed25519 key.

## [0.3.0] - 2025-03-18

### Added
- StatusList2021 support in `sigil-core`: bitstring `set`/`is_set`/`encode` (gzip +
  base64url) and `to_credential`.
- `GET /api/v1/status/:listId` serving the status list as a JWT-VC.
- Web verifier page renders per-check pass/fail badges.

### Changed
- Credentials now include a `credentialStatus` (`StatusList2021Entry`) block at issuance.

## [0.2.0] - 2024-10-07

### Added
- `did:web` issuer identity and a served DID document at `/.well-known/did.json`.
- Credential listing (`GET /api/v1/credentials`) and detail (`GET /api/v1/credentials/:id`).
- Postgres persistence via sqlx with the initial `credentials` migration.
- React wallet and issuer pages over a typed API client.

### Changed
- Switched credential encoding to JWT-VC with EdDSA using `jsonwebtoken` v9.

### Fixed
- Correct `nbf`/`exp` handling for expired-credential detection.

## [0.1.0] - 2024-06-11

### Added
- Initial `sigil-core` library: `KeyPair` (Ed25519, PKCS#8 DER), `did:key` derivation,
  `VerifiableCredential` model, and `issue_jwt_vc` / `verify_jwt_vc`.
- Minimal axum server with `GET /health` and `POST /api/v1/credentials/issue`.
- Project scaffolding, CI, and Apache-2.0 license.

[Unreleased]: https://github.com/sigil-id/sigil/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/sigil-id/sigil/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/sigil-id/sigil/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/sigil-id/sigil/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/sigil-id/sigil/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/sigil-id/sigil/releases/tag/v0.1.0
