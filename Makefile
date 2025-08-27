# Sigil developer Makefile.
# Common workflows for the Rust workspace, the web app, and the local stack.

# Postgres URL used by `migrate`; override on the CLI, e.g.
#   make migrate DATABASE_URL=postgres://user:pass@host/db
DATABASE_URL ?= postgres://sigil:sigil@localhost:5432/sigil
COMPOSE ?= docker compose

.DEFAULT_GOAL := help

.PHONY: help dev up down logs build fmt lint test migrate web-install clean

help: ## Show this help
	@grep -hE '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "} {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

dev: ## Start the full local stack (db + server + web) with build
	$(COMPOSE) up --build

up: ## Start the stack in the background
	$(COMPOSE) up -d --build

down: ## Stop the stack and remove containers
	$(COMPOSE) down

logs: ## Tail logs from the running stack
	$(COMPOSE) logs -f

build: ## Build the Rust workspace and the web app
	cargo build --release --workspace
	cd web && pnpm install --frozen-lockfile && pnpm build

fmt: ## Format Rust and check web formatting
	cargo fmt --all
	cd web && pnpm install --frozen-lockfile && pnpm lint --fix || true

lint: ## Run clippy and web lint/typecheck
	cargo clippy --all-targets --all-features -- -D warnings
	cd web && pnpm install --frozen-lockfile && pnpm typecheck && pnpm lint

test: ## Run Rust and web test suites
	cargo test --workspace --all-features
	cd web && pnpm install --frozen-lockfile && pnpm test

migrate: ## Apply database migrations (requires sqlx-cli)
	@command -v sqlx >/dev/null 2>&1 || { \
		echo "sqlx-cli not found. Install with: cargo install sqlx-cli --no-default-features --features native-tls,postgres"; \
		exit 1; }
	sqlx migrate run --source migrations --database-url "$(DATABASE_URL)"

web-install: ## Install web dependencies
	cd web && pnpm install

clean: ## Remove build artifacts
	cargo clean
	rm -rf web/dist web/node_modules
