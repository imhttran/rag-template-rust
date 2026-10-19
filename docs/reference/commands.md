# Commands

The project is driven by three commands, with convenience targets that wrap the same steps.

## Workflow

Three commands:

| Command      | What it does                                                      |
| ------------ | ----------------------------------------------------------------- |
| `cmd/ingest` | Read a document → split into chunks → embed each chunk → store it |
| `cmd/rag`    | Embed a question → retrieve the closest chunks → answer from them |
| `cmd/eval`   | Score retrieval (recall/precision) and compare a lexical reranker |

## Shortcuts

A `Makefile` wraps the same steps:

| Target             | Runs                                                  |
| ------------------ | ----------------------------------------------------- |
| `make db-up`       | start the database and apply all migrations           |
| `make db-down`     | `docker compose down`                                 |
| `make db-schema`   | apply all migrations (existing volume only)           |
| `make test`        | run the unit tests (no database needed)               |
| `make fmt-json`    | rewrite every tracked JSON file with `jq`             |
| `make integration` | run the integration tests against the database        |
| `make ingest`      | ingest `examples/loan-policy.md` (`FILE=…` to change) |
| `make ask Q="…"`   | ask a question (omit `Q` to be prompted)              |
| `make eval`        | score retrieval against `evals/retrieval.json`        |

## Rust transition targets

The Rust port lives under `rust/`; Go stays present during the transition. These
targets run the Rust suite and the RUST-007B transition evidence.

| Target                              | Runs                                                                 |
| ----------------------------------- | -------------------------------------------------------------------- |
| `make rust-fmt`                     | `cargo fmt --manifest-path rust/Cargo.toml --check`                  |
| `make rust-build`                   | `cargo build --locked --manifest-path rust/Cargo.toml`               |
| `make rust-test`                    | `cargo test --locked --manifest-path rust/Cargo.toml`                |
| `make rust-clippy`                  | `cargo clippy --locked --all-targets --manifest-path rust/Cargo.toml -- -D warnings` |
| `make rust-acceptance-evidence`     | `cargo test --locked --manifest-path rust/acceptance/evidence/Cargo.toml` |
| `make rust-acceptance-clean-checkout` | `cargo test --locked --manifest-path rust/acceptance/clean_checkout_go/Cargo.toml` |
| `make rust-acceptance-k3`          | K=3 Go/Rust evaluation parity against the frozen reference          |
| `make rust-acceptance-ingest-md-text` | production Markdown/plain-text ingestion and transaction safety   |
| `make rust-acceptance-live-rag`    | live Ollama + PostgreSQL ingest → query → cited-answer verification  |
| `make rust-acceptance-functional`  | K=3 parity + Markdown/plain-text ingestion (needs the database)      |
| `make rust-acceptance-api-contract` | deterministic Axum API contract (upload, status, query, health, refusal) |
| `make web-e2e`                      | real-browser Playwright upload → ingest → cited-answer verification   |
| `make transition-evidence`          | the RUST-007B acceptance crates                                     |
| `make rust-check`                   | fmt, build, test, clippy, and transition evidence                    |
| `make check`                        | Go unit tests, `rust-check`, and the web checks                      |

The clean-checkout crate stages a copy of the working tree and runs the
documented sequence from
[the usage guide](../guides/usage.md#clean-checkout-with-go-present):
`go build ./...`, `go test ./...`, `go vet ./...`, and `make rust-test`.

## Rust API and web UI (RUST-008A)

The Axum API binary (`rust/src/bin/api.rs`) serves the browser-facing contract
and calls the same Rust services as the CLI:

| Endpoint | Purpose |
| --- | --- |
| `GET /api/health` | Service, database, and model status; never returns credentials |
| `POST /api/documents/ingest` | Multipart `file` upload for `.md`, `.markdown`, `.txt`, `.pdf` |
| `GET /api/documents/{source}/status` | Stored chunks, sections, pages, corpus, and fingerprint |
| `POST /api/query` | `{question, top_k?}` -> answer, `insufficient`, and cited sources |

`API_HOST` defaults to `127.0.0.1` and `API_PORT` to `8080`. `RAG_CORPUS`
selects the corpus (default `trusted`). The Next.js app in `web/` proxies
same-origin `/api/*` server-side to `API_URL`; the browser never receives the
API URL or credentials. Run `make web-e2e` for the real-browser check.
