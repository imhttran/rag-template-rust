# RUST-007B transition evidence (Go present)

Frozen evidence for the Go→Rust transition while the Go reference implementation
remains present in this repository. This document is the authoritative transition
record. Go is not removed by RUST-007B.

The evidence crate `rust/acceptance/evidence` reads this file from
`docs/reference/rust-transition-evidence.md` and asserts every section below.

## Capability matrix

| Capability | Go reference | Rust port | Status |
| --- | --- | --- | --- |
| Build | `go build ./...` | `cargo build --locked --manifest-path rust/Cargo.toml` | Both present |
| Unit tests | `go test ./...` | `cargo test --locked --manifest-path rust/Cargo.toml` | Both present |
| Vet / lint | `go vet ./...` | `cargo clippy --locked --all-targets --manifest-path rust/Cargo.toml -- -D warnings` | Both present |
| Format check | `gofmt -l .` | `cargo fmt --manifest-path rust/Cargo.toml --check` | Both present |
| Ingest | `go run ./cmd/ingest` | Rust ingest CLI (`rust/acceptance/ingest_cli`) | Go authoritative, Rust ported |
| Ask | `go run ./cmd/rag` | Rust retrieval CLI (`rust/acceptance/retrieval_cli`) | Go authoritative, Rust ported |
| Eval | `go run ./cmd/eval` | Rust eval parity (`rust/acceptance/eval_parity`) | Go authoritative, Rust ported |
| Sweep | `./scripts/sweep.sh` | Rust eval parity harness | Go authoritative, Rust partial |
| Bilingual export | `cmd/expbilingual` | Rust bilingual contract (`rust/acceptance/bilingual_parity`) | Go authoritative, Rust ported |
| Migration evidence | this document | `rust/acceptance/evidence` | Frozen |
| Clean checkout | accepted Go checks | `rust/acceptance/clean_checkout_go` | Frozen |

The Makefile keeps the Go targets (`test`, `integration`, `ingest`, `ask`,
`eval`, `sweep`) and the Rust targets (`rust-fmt`, `rust-build`, `rust-test`,
`rust-clippy`, `rust-check`). `make check` runs `test rust-check web-check`.

## Remaining limitations

- The Go reference implementation stays authoritative for ingest, ask, eval,
  sweep, and bilingual export until a later task removes it.
- The full `internal/cli` suite stall is still open; see
  `docs/reports/rust-migration/cli-fullsuite-timeout-evidence.md`.
- RUST-007D live-model groundedness/citation closure is **verified** and accepted
  by the operator (`sop verify RUST-007D` → `VERIFIED`); the preserved audit is
  under `.agent-sdlc/runs/RUST-007D/`.
- The Rust package still lives at `rust/` and is not moved to the repository
  root.
- The Rust port has no complete replacement for the `scripts/sweep.sh` settings
  matrix.
- Original K=3 parity, Markdown/plain-text ingestion, a live production
  ingest → query → cited answer flow, and the real Axum/Next.js browser end-to-end
  flow remain open.
- A clean checkout needs both the Go and the Rust toolchains plus `make`.

## Rollback reference

To restore the Go reference implementation as the only implementation:

1. Revert the Rust acceptance harness crates and this evidence document.
2. Restore `Makefile` so `check` runs `test` only.
3. Restore `.githooks/pre-commit` and `.githooks/pre-push` to the Go-only steps.
4. Remove `.github/workflows/ci.yml` (the transition CI definition).
5. Rebuild the Go reference: `go build ./...`, `go test ./...`, `go vet ./...`.
6. The Go sources under `cmd/` and `internal/` were never removed, so no source
   recovery is needed. Restoring Go is a revert of tooling only.

The Go reference tree is frozen at commit SHA
`d72fb398e8959eea83ed24be03f6df8cdeca86a2`.

## Pinned Go commit SHA

`d72fb398e8959eea83ed24be03f6df8cdeca86a2`

## Source repository URL

`https://github.com/tomtran/rag-template`

## Fixture provenance

Frozen fixtures and shared assets. RUST-007B does not modify any of them.

| Asset | Purpose |
| --- | --- |
| `migrations/*.sql` | PostgreSQL + pgvector schema; applied by `make db-schema`. Read-only inputs. |
| `evals/` | Frozen retrieval evaluation set (`evals/retrieval.json`). Read-only input. |
| `examples/` | Frozen sample documents (for example `examples/loan-policy.md`). Read-only input. |
| `docker-compose.yml` | PostgreSQL + pgvector service definition. Unchanged. |
| `rust/fixtures/` | Frozen Rust fixture set (`en/`, `vi/`, `page/`). Unchanged. |

Each asset keeps the provenance recorded in `RUST-001-go-baseline.md` (fixture
manifest) and `RUST-003-fixture-parity.md` (section/chunk acceptance matrix).

## CI configuration

`.github/workflows/ci.yml` defines the transition layout while Go remains
present:

- `go-reference` runs `go build ./...`, `go test ./...`, `go vet ./...`, and a
  `gofmt` check with Go installed from `go.mod`.
- `rust-transition` installs the pinned toolchain from
  `rust/rust-toolchain.toml`, runs Rust fmt/build/test/Clippy, then
  `make rust-acceptance-evidence` and
  `make rust-acceptance-clean-checkout`. The clean-checkout crate runs the Go
  commands too, so this job installs Go as well.

The local equivalents are the `.githooks/` hooks, which delegate to the same
Makefile targets.
