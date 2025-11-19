# Security Policy

## Supported versions

Sigil is pre-1.0. Security fixes are applied to the latest released minor version
only. Please upgrade to the newest `0.x` release before reporting an issue.

| Version | Supported |
| --- | --- |
| 0.5.x | Yes |
| 0.4.x | Critical fixes only |
| < 0.4 | No |

## Reporting a vulnerability

Please report suspected vulnerabilities privately. Do **not** open a public issue for
a security problem.

- Use GitHub's **Report a vulnerability** flow under the repository's *Security* tab
  (private vulnerability reporting), or
- Email **security@sigil-id.org** with a description, affected version, and reproduction
  steps.

We aim to acknowledge reports within 3 business days and to provide a remediation plan
or fix timeline within 10 business days. We will coordinate a disclosure date with you
and credit reporters who wish to be named.

## Threat model summary

Sigil issues, stores, and verifies Verifiable Credentials. The assets and adversaries we
design against:

- **Issuer signing key (highest value).** Compromise lets an attacker mint credentials
  that verify as genuine. Mitigations: keep the key out of the database and logs, supply
  it explicitly in production, and back it with a KMS/HSM (roadmap). The
  auto-generated-key convenience path is for development only.
- **Credential forgery / tampering.** Bounded by Ed25519. Any modification to a JWT-VC's
  header or payload invalidates the signature and is rejected at the `signature` check.
- **Replay and staleness.** Mitigated by `nbf`/`exp` time bounds and by StatusList2021
  revocation. Verifiers applying custom policy should consult the individual checks
  (`signature`, `notExpired`, `notRevoked`) rather than only the aggregate `valid` flag.
- **Revocation privacy.** StatusList2021 batches many credentials into one bitstring, so
  fetching a status list does not reveal which credential is being checked.
- **Issuer trust.** `did:web` roots issuer trust in web PKI (TLS + DNS). Relying parties
  decide which issuer DIDs they accept; Sigil does not ship a trust registry.
- **Transport.** All production traffic must be over TLS. `did:web` resolution and issuer
  metadata are only meaningful when served over HTTPS.

Out of scope for the current model: holder binding / proof of possession (credentials
are bearer tokens today), unlinkable presentations, and multi-tenant issuer isolation.
These are tracked on the roadmap.

## Key handling guidance

- Provide the issuer key via `SIGIL_ISSUER_KEY_PKCS8_B64` (base64 PKCS#8 DER) from a
  secret manager, not a checked-in file.
- Never rely on the auto-generated key in production: a restart rotates it and
  invalidates every outstanding credential.
- Rotate keys on a schedule and on suspected compromise. Until versioned DID documents
  land, rotation requires publishing the new key in `/.well-known/did.json` and
  re-issuing affected credentials.
- `.pem`, `.key`, and `.der` files and all `.env` files (except `.env.example`) are
  gitignored. Keep it that way.

## Handling of a compromised issuer key

1. Revoke or re-issue affected credentials as appropriate.
2. Publish the new key in the issuer DID document and remove the compromised one.
3. Notify relying parties out of band, since already-verified credentials cannot be
   retroactively un-trusted by verifiers that cached them.

## Dependencies

We track advisories for Rust (`cargo audit`) and npm dependencies and update promptly on
security releases. CI builds with `-D warnings`.
