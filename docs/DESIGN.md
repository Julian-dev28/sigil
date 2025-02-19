# Design decisions

This file records the significant architectural decisions behind Sigil in a
lightweight ADR style. Each entry states the context, the decision, and the
trade-offs we accepted. Decisions are additive; where a later decision revises an
earlier one, the earlier entry is annotated rather than deleted.

---

## ADR 0001 — JWT-VC as the primary credential format

**Status:** Accepted

**Context.** The W3C VC Data Model can be secured two ways: JOSE/JWT proofs (JWT-VC)
or Linked Data Proofs (LD-proofs / Data Integrity). LD-proofs are more expressive —
they operate on the RDF graph and support proof suites like BBS+ for selective
disclosure — but they require JSON-LD canonicalization (URDNA2015), context
resolution, and a heavier verification pipeline.

**Decision.** Use JWT-VC (EdDSA) as the primary, default credential format.

**Rationale.**
- Verification is a well-trodden path: decode the JOSE header, resolve a key, check a
  signature and time bounds. No canonicalization, no network fetches of `@context`.
- The tooling ecosystem for JOSE is mature and available in every language, which
  matters for a platform whose whole point is interoperability with third-party
  verifiers.
- Ed25519 keys are small and fast, and `did:key` gives us self-certifying subject
  identifiers for free.

**Consequences / trade-offs.**
- No native selective disclosure. We accept this for now; SD-JWT-VC is the planned path
  (see ADR 0006) and slots in without disturbing the core.
- We are committed to keeping the `vc` claim's JSON identical to the embedded credential
  object so that consumers can treat either representation as canonical.

---

## ADR 0002 — StatusList2021 for revocation

**Status:** Accepted

**Context.** Revocation needs to be checkable by verifiers without leaking which
specific credential is being checked and without an availability dependency on the
issuer for every verification. The main options are per-credential revocation
endpoints, OCSP-style responders, cryptographic accumulators, and bitstring status
lists.

**Decision.** Publish revocation as a StatusList2021 credential — a gzip-compressed,
base64url-encoded bitstring, itself a Verifiable Credential.

**Rationale.**
- **Privacy through aggregation.** One list covers many thousands of credentials, so
  fetching it reveals nothing about which credential a verifier cares about. A
  per-credential endpoint would leak exactly that.
- **Cacheable and offline-friendly.** The list is a static artifact a verifier can cache
  and re-check without contacting the issuer per verification.
- **Standardized and interoperable**, unlike a bespoke revocation endpoint.

**Consequences / trade-offs.**
- Revocation is coarse-grained in time: it is as fresh as the verifier's cache of the
  list. That is an acceptable and well-understood property.
- Accumulators offer stronger privacy but are far more complex and less widely
  supported; we judged StatusList2021 the right point on the curve.

---

## ADR 0003 — did:web for issuers, did:key for subjects

**Status:** Accepted

**Context.** Issuers and subjects have different needs. An issuer is a long-lived,
discoverable authority whose keys must be publishable and (eventually) rotatable.
A subject/holder is often ephemeral and should not require any hosting
infrastructure.

**Decision.** Identify issuers with `did:web` and subjects/holders with `did:key`.

**Rationale.**
- **`did:web`** anchors issuer trust in the existing web PKI: if you trust the TLS
  certificate for `issuer.example`, you trust the keys it serves at
  `/.well-known/did.json`. It supports rotation and multiple keys, which an issuer
  needs. No new global infrastructure is introduced.
- **`did:key`** is self-certifying: the identifier *is* the multibase-encoded public
  key, so subjects need no hosting and verifiers need no network round trip to resolve
  them.

**Consequences / trade-offs.**
- `did:web` inherits the trust assumptions (and failure modes) of DNS and the CA system.
  We consider that a pragmatic, familiar baseline rather than a flaw.
- `did:key` cannot rotate — a rotation is a new identifier. That is appropriate for
  ephemeral subject keys and inappropriate for issuers, which is exactly why issuers use
  `did:web`.

---

## ADR 0004 — Independent verification checks over a single boolean

**Status:** Accepted

**Context.** A verifier could return a single `valid` boolean. But different relying
parties have different policies: some tolerate a short expiry grace period, some treat
an unreachable status list as fail-open, others as fail-closed.

**Decision.** The verify endpoint returns `signature`, `notExpired`, and `notRevoked`
as separate booleans alongside a convenience `valid` that is their conjunction.

**Rationale.** Policy belongs to the relying party. Exposing the individual checks lets
callers implement their own policy without re-implementing verification.

**Consequences.** The `valid` flag remains for the common case; sophisticated callers
read the individual checks. This shaped the `VerificationResult` type in `sigil-core`
and the API response schema.

---

## ADR 0005 — Ed25519 signing keys are held in-process; store is optional

**Status:** Accepted

**Context.** For a smooth first-run experience the demo must work without operators
provisioning a database or a KMS. For production, key handling must be explicit.

**Decision.** The issuer key is an Ed25519 PKCS#8 DER blob supplied via
`SIGIL_ISSUER_KEY_PKCS8_B64`; if absent, one is generated at startup with a warning.
Persistence is behind a `Store` trait with a Postgres implementation and an in-memory
fallback selected when `DATABASE_URL` is unset.

**Rationale.**
- Zero-dependency startup makes the platform demonstrable and testable in one command,
  and keeps integration tests hermetic (they run against `MemoryStore`).
- A generated ephemeral key is obviously wrong for production — the warning and the
  documentation make that explicit — but perfect for local iteration.

**Consequences / trade-offs.**
- The generated-key path must never be relied on in production; restarting rotates the
  key and invalidates outstanding credentials. This is documented in `ARCHITECTURE.md`
  and `SECURITY.md`.
- Keeping the key in memory (not in the store) keeps signing material off the database
  and out of backups. A KMS/HSM-backed signer is the intended production upgrade.

---

## ADR 0006 — Selective disclosure deferred to SD-JWT-VC

**Status:** Proposed

**Context.** Bearer credentials over-share: presenting an `EmploymentCredential` reveals
every claim. Selective disclosure lets a holder reveal only what a verifier needs.

**Decision.** Defer selective disclosure and, when added, implement it as SD-JWT-VC
rather than BBS+/LD-proofs in the first instance.

**Rationale.** SD-JWT-VC stays within the JOSE world we already depend on (ADR 0001),
reuses our signing and resolution code, and is gaining the widest deployment momentum.
BBS+ remains attractive for unlinkable presentations and is tracked as a later,
LD-proof-based option.

**Consequences.** The core is structured so a second issuance/verification path can be
added without changing storage or HTTP layers; the `vc`-claim invariant from ADR 0001
must be preserved.
