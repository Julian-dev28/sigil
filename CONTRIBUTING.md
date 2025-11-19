# Contributing to Sigil

Thanks for your interest in improving Sigil. This document covers how to set up a
development environment, the conventions we follow, and how changes get merged.

By participating you agree to abide by our [Code of Conduct](#code-of-conduct).

## Development setup

Prerequisites:

- A stable Rust toolchain (pinned in `rust-toolchain.toml`; `rustup` will honour it).
- Node.js 20+ and [pnpm](https://pnpm.io/).
- Optionally Docker and Docker Compose for the full stack.

Clone and build the backend:

```bash
cargo build
cargo test
cargo run -p sigil-server      # runs with an in-memory store, no database needed
```

Run the frontend:

```bash
cd web
pnpm install
pnpm dev
```

Bring up everything (Postgres + server + UI):

```bash
docker compose up --build
```

Backend configuration is via environment variables; copy
`crates/sigil-server/.env.example` to `.env` and adjust as needed.

## Before you open a pull request

Run the same checks CI runs. They must pass.

Backend:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Frontend:

```bash
cd web
pnpm typecheck
pnpm lint
pnpm test
```

Please add or update tests for any behaviour you change, and update the relevant docs
(`README.md`, `ARCHITECTURE.md`, `docs/openapi.yaml`) when you change the API or the
domain model. The Rust and TypeScript sides share a domain model — keep them in sync.

## Commit conventions

We use [Conventional Commits](https://www.conventionalcommits.org/). The commit summary
drives the changelog and version bumps.

```
<type>(<optional scope>): <description>

[optional body]

[optional footer(s)]
```

Common types:

| Type | Use for |
| --- | --- |
| `feat` | A new feature (minor version bump). |
| `fix` | A bug fix (patch version bump). |
| `docs` | Documentation only. |
| `refactor` | A change that neither fixes a bug nor adds a feature. |
| `perf` | A performance improvement. |
| `test` | Adding or correcting tests. |
| `build` | Build system, dependencies, Docker, CI-adjacent tooling. |
| `ci` | CI configuration. |
| `chore` | Maintenance that doesn't fit above. |

Examples:

```
feat(server): add StatusList2021 status endpoint
fix(core): reject JWT-VC with mismatched kid
docs: document did:web rotation in ARCHITECTURE
```

Breaking changes carry a `!` after the type/scope and a `BREAKING CHANGE:` footer:

```
feat(api)!: rename credentialType to type in list response

BREAKING CHANGE: clients reading the list endpoint must update field names.
```

Use scopes such as `core`, `server`, `web`, `infra`, `docs`, `api` where they add clarity.

## Pull request process

1. Fork and branch from `main`. Name branches descriptively (`feat/status-list-cache`).
2. Keep PRs focused; smaller PRs get reviewed faster.
3. Ensure all checks pass and the branch is up to date with `main`.
4. Fill in the PR description: what changed, why, and how you tested it. Link related
   issues.
5. A maintainer will review. Address feedback by pushing additional commits; we squash on
   merge, so intermediate history need not be tidy.
6. At least one maintainer approval and green CI are required to merge.

## Reporting bugs and requesting features

Open an issue with a clear title, reproduction steps (for bugs), the version you are on,
and what you expected. For security issues, follow [`SECURITY.md`](./SECURITY.md) instead
of opening a public issue.

## License

By contributing, you agree that your contributions are licensed under the
[Apache License 2.0](./LICENSE), the same license that covers the project.

## Code of Conduct

This project follows the
[Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/),
version 2.1. Report unacceptable behaviour to **conduct@sigil-id.org**. Maintainers are
responsible for clarifying standards and will take appropriate, fair action in response
to violations.
