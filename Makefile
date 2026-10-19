.PHONY: db-up db-down db-schema test fmt-json integration ingest ask eval sweep rust-build rust-test rust-fmt rust-clippy rust-acceptance-evidence rust-acceptance-clean-checkout transition-evidence rust-check check web-check-env web-install web-start web-stop web-status web-typecheck web-build web-check
# Convenience targets for the ingest/query workflow.
#
#   make db-up        start PostgreSQL and apply the schema
#   make db-schema    (re)apply every migration in migrations/
#   make test         unit tests (no database needed)
#   make fmt-json     rewrite every tracked JSON file with jq
#   make integration  tests against the docker-compose database (needs Docker)
#   make ingest       load examples/loan-policy.md
#   make ask Q="…"    ask a question (omit Q to be prompted)
#   make eval         score retrieval against evals/retrieval.json
#   make sweep        compare eval metrics across settings
#   make web-start    start the Next.js prototype on 127.0.0.1:3000 (background)
#   make web-stop     stop the prototype started by make web-start
#   make web-status   report the prototype's state, URL, and log path
#
# Override inputs on the command line, e.g.
#   make ingest FILE=examples/other.md
#   make ask Q="When is a late fee assessed?"

# Load local .env if present
-include .env
DATABASE_URL ?= postgres://rag:rag@127.0.0.1:5434/rag?sslmode=disable
export DATABASE_URL
FILE ?= examples/loan-policy.md
Q ?=
AXIS ?= all

# Start the database and make sure the schema is applied.
db-up:
	docker compose up -d --wait
	$(MAKE) db-schema

# Stop the database.
db-down:
	docker compose down


# Run the unit tests; no database required.
test:
	go test ./...

# Rewrite every tracked JSON file with jq's formatting. This expands the
# hand-compacted one-line arrays in .zed/settings.json and evals/retrieval.json,
# so the first run produces a large diff. Keys are left in the order written;
# add -S to jq below if you would rather sort them.
fmt-json:
	for f in $$(git ls-files -- '*.json'); do \
		jq . "$$f" > "$$f.tmp" || { rm -f "$$f.tmp"; echo "invalid json: $$f" >&2; exit 1; }; \
		mv "$$f.tmp" "$$f"; \
	done

# Run the PostgreSQL integration tests against the docker-compose database,
# starting it first. The tests truncate or delete from the documents table, so
# override DATABASE_URL to point at a throwaway database instead of the dev one.
#
# -p 1 runs the packages one at a time: internal/retrieval truncates the whole
# table, which would fight internal/ingestion's tests if they ran in parallel.
integration: db-up
	RAG_INTEGRATION=1 go test ./internal/retrieval/ ./internal/ingestion/ -run Integration -p 1 -v

# Chunk FILE and store it (safe to re-run).
ingest:
	go run ./cmd/ingest $(FILE)

# Ask a question; omit Q to be prompted.
ask:
	go run ./cmd/rag "$(Q)"

# Score retrieval against evals/retrieval.json.
eval:
	go run ./cmd/eval

# Compare cmd/eval metrics across a matrix of settings (see scripts/sweep.sh).
sweep:
	./scripts/sweep.sh $(AXIS)

db-schema:
	@for f in migrations/*.sql; do \
		psql "$$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$$f" || exit 1; \
	done

# ---- Web UI prototype (Next.js, UI-002) ----------------------------------------
# Manage the local prototype on 127.0.0.1:3000. PID and log live under web/.run/
# (git-ignored). Node/npm are found on PATH or via the Zed-bundled installation;
# override with NODE_BIN=... / NPM_BIN=... The prototype serves mock data only.
web-start:
	./scripts/web-control.sh start

web-stop:
	./scripts/web-control.sh stop

web-status:
	./scripts/web-control.sh status

# Resolve and validate the Node/npm toolchain (overrides -> PATH -> Zed bundled).
web-check-env:
	./scripts/web-control.sh check-env

# Install web dependencies with the resolved toolchain.
web-install:
	./scripts/web-control.sh install

# ---- Rust checks (the port under rust/) ----------------------------------------
# Reused by .githooks so the hooks and the Makefile never diverge. The Rust
# package still lives at rust/ until RUST-007 moves it to the root.
rust-fmt:
	cargo fmt --manifest-path rust/Cargo.toml --check

rust-build:
	cargo build --locked --manifest-path rust/Cargo.toml

rust-test:
	cargo test --locked --manifest-path rust/Cargo.toml

rust-clippy:
	cargo clippy --locked --all-targets --manifest-path rust/Cargo.toml -- -D warnings

# RUST-007B acceptance: the frozen transition record and the documented
# clean-checkout sequence. Both crates read the repository relative to their own
# manifest, so they run alongside the Go reference while it remains present.
rust-acceptance-evidence:
	cargo test --locked --manifest-path rust/acceptance/evidence/Cargo.toml

rust-acceptance-clean-checkout:
	cargo test --locked --manifest-path rust/acceptance/clean_checkout_go/Cargo.toml

transition-evidence: rust-acceptance-evidence rust-acceptance-clean-checkout

rust-check: rust-fmt rust-build rust-test rust-clippy transition-evidence

# ---- Web checks (the Next.js prototype under web/) -----------------------------
web-typecheck:
	./scripts/web-control.sh typecheck

web-build:
	./scripts/web-control.sh build

web-check: web-typecheck web-build

# Everything: the Go reference unit tests, the Rust checks, and the web checks.
check: test rust-check web-check
