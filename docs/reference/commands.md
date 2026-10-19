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
| `make transition-evidence`          | both RUST-007B acceptance crates                                     |
| `make rust-check`                   | fmt, build, test, clippy, and transition evidence                    |
| `make check`                        | Go unit tests, `rust-check`, and the web checks                      |

The clean-checkout crate stages a copy of the working tree and runs the
documented sequence from
[the usage guide](../guides/usage.md#clean-checkout-with-go-present):
`go build ./...`, `go test ./...`, `go vet ./...`, and `make rust-test`.
