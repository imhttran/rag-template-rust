# Rust Migration and Learning Plan

Repository: https://github.com/imhttran/rag-template-rust
Target path: `docs/plans/PLAN-RUST-MIGRATION.md`
Created: 2026-10-17
Status: DRAFT — execution has not started.

## Project

rag-template-rust — port the implemented Go RAG project to Rust.

## Summary

Convert the implemented Go RAG project to idiomatic Rust. Preserve observable behavior and use each phase to learn Rust. Keep the Go implementation as the reference until Rust passes parity checks. Scope and the execution contract below are canonical; the stages under "## Tasks" are the machine plan SOP executes.

## Goal

Convert the implemented Go RAG project to idiomatic Rust. Preserve observable behavior and use each phase to learn Rust. Keep the Go implementation as the reference until Rust passes parity checks.

The existing `docs/requirements/PRD.md` remains the product requirements source. Use `docs/architecture/OVERVIEW.md` and the configuration, commands, capabilities, and testing references to establish the migration contract. Confirm them against code before execution.

## Scope

Port all implemented capabilities used by `cmd/ingest`, `cmd/rag`, `cmd/eval`, and `cmd/expbilingual`. Preserve SQL migrations, environment variable names and defaults, dataset formats, metadata, provider selection, and documented failure behavior.

Exclude OCR, genealogy entity extraction, additional providers, retrieval redesign, and performance optimization. Do not introduce a new RAG framework. Reuse established libraries for infrastructure where they preserve the required contracts.

RUST-008 adds a new Next.js + Axum user interface that is **not** part of the Go baseline (which has no browser UI). It is a deliberate addition, not a port: it calls the same Rust services as the CLI and reuses the reference web pattern from the `inventory` project (Next.js -> Axum -> PostgreSQL; the browser talks only to Next.js; `/api/*` is proxied server-side) plus the baseline's `rag-retrieval/v1` query contract where practical.

## Execution contract

- Work in an isolated migration branch or worktree. Preserve unrelated changes.
- Keep Go runnable during migration. Do not remove it before RUST-007 passes.
- Complete tasks in dependency order. Continue automatically through implementation, verification, and documentation until completion or a real blocker.
- Make routine implementation decisions within scope. Record material tradeoffs.
- Use one Cargo package with library modules and thin command binaries initially. Add crates only when a concrete boundary requires them.
- Pin the Go reference commit and fixtures. Never regenerate expected outputs from Rust to make a failing parity check pass.
- Use mocks and fixed vectors for deterministic checks. Use separate, explicitly selected test databases for writes.
- Never use production data, reset a shared database, or change the schema destructively as part of verification.
- Record task evidence and a short Rust learning note after each task.
- Preserve repository approval rules for publishing and merging. Do not assume this file authorizes either action.

The task contracts below were converted to SOP's canonical plan schema
(stages restated as `- **Objective:**`, `- **Depends on:**`, and
`- **Acceptance:**` fields) on 2026-10-17 without changing any criterion.
Exactly RUST-001 through RUST-007 and their dependencies are retained; RUST-005
was later split into RUST-005A (storage foundation) and RUST-005B (ingestion
pipeline) on 2026-10-17, preserving all of its acceptance criteria, with RUST-006
depending on RUST-005B. The machine plan was validated with SOP's own
`planner.PlanFromMarkdown` and `Plan.Validate` before execution.

## Tasks

### RUST-001 — Establish the Go baseline

- **Objective:** Establish and freeze the Go reference baseline: read repository instructions and inspect implemented behavior; record the reference commit, toolchain, commands, configuration, and dependencies; run the existing Go unit and required integration checks; map every implemented capability to a Rust task and verification method; freeze representative English, Vietnamese, and page-provenance fixtures; and record evaluation settings and baseline metrics when services are available.
- **Depends on:** none
- **Acceptance Checks:** `go build ./...`
- **Acceptance Checks:** `go test ./...`
- **Acceptance:** The reference SHA and environment are recorded in migration evidence.
- **Acceptance:** Baseline checks pass, or pre-existing failures are isolated and documented.
- **Acceptance:** Every implemented capability has an assigned task and verification method.
- **Acceptance:** Fixtures capture outputs from the pinned Go implementation.
- **Acceptance:** Exact-match fields, float tolerances, tie ordering, and evaluation thresholds are set before Rust comparisons. Deterministic retrieval metrics must not regress on the fixed corpus.

Work steps:

1. Read repository instructions and inspect implemented behavior.
2. Record the reference commit, toolchain, commands, configuration, and dependencies.
3. Run the existing Go unit and required integration checks.
4. Map every implemented capability to a Rust task and verification method.
5. Freeze representative English, Vietnamese, and page-provenance fixtures.
6. Record evaluation settings and baseline metrics when services are available.

Verification: Run `go test ./...` and repository-prescribed checks. Record exact integration and evaluation commands discovered from the repository. Mark unavailable checks BLOCKED; do not count them as passed.

Learning focus: Map Go packages, interfaces, and errors to Rust modules, traits, and `Result`.

### RUST-002 — Create the Rust foundation

- **Objective:** Create a Cargo package with library modules and thin binaries; define section, chunk, and retrieved-document types; port configuration parsing, defaults, and validation; select compatible maintained dependencies from official documentation; and add Rust checks alongside the existing Go checks. Includes the Rust toolchain and Zed development-environment setup below.
- **Depends on:** RUST-001
- **Acceptance Checks:** `cargo build --manifest-path rust/Cargo.toml`
- **Acceptance:** Rust builds without removing the Go commands.
- **Acceptance:** Configuration names, defaults, and invalid-value behavior match the baseline.
- **Acceptance:** Dependency choices and toolchain requirements are documented and locked.
- **Acceptance:** Domain types preserve source, section, language, chunk index, and optional page metadata.

Work steps:

1. Create a Cargo package with library modules and thin binaries.
2. Define section, chunk, and retrieved-document types.
3. Port configuration parsing, defaults, and validation.
4. Select compatible maintained dependencies from official documentation.
5. Add Rust checks alongside the existing Go checks.

Verification: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`. Add `--all-features` only if all optional features can run together.

Learning focus: Structs, enums, modules, traits, `Option`, and `Result`.

**Rust toolchain and Zed setup (part of RUST-002).** Use Zed as the Rust development environment. This requirement covers editing, diagnostics, running tests, and debugging this project. Building a Zed extension or changing Zed itself is outside scope.

- **Acceptance:** The selected toolchain and components work in a normal shell and Zed's integrated terminal.
- **Acceptance:** Completion, go-to-definition, hover information, and diagnostics work in Rust files.
- **Acceptance:** Saving a Rust file formats it. A temporary lint issue produces a Clippy diagnostic; remove the issue after verification.
- **Acceptance:** A unit test runs from Zed, and a debugging smoke check succeeds.
- **Acceptance:** Setup instructions support the user's macOS and Linux environments. Record each platform as verified or unverified; do not infer success on an untested machine.
- **Acceptance:** Existing Zed and agent settings remain intact. Missing access to the user's desktop is recorded as pending manual verification, not a successful check.

Setup steps:

1. Check for an existing Rust installation with `rustup show`, `rustc --version`, and `cargo --version`. Preserve existing toolchains.
2. If Rust is absent, install it on the development machine with the official command:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Open a new shell after installation. Confirm that Cargo is available in Zed's integrated terminal.
4. Select a stable Rust release compatible with the chosen dependencies. Record its exact version in `rust-toolchain.toml` and use that version in CI. Do not leave the project on a moving `stable` pin.
5. Add the project toolchain components with `rustup component add rustfmt clippy rust-src rust-analyzer` from the project directory after creating the toolchain file.
6. Verify the platform linker and native build prerequisites with a small Cargo build. Install missing platform tools only when required.
7. Open the directory containing `Cargo.toml` in Zed. Confirm rust-analyzer starts. Zed has native Rust support and can use rust-analyzer from PATH or download its own version.
8. Merge project settings into `.zed/settings.json`. Preserve existing settings and agent configuration. Enable format-on-save for Rust. Use the language server formatter and enable Clippy diagnostics through rust-analyzer:

   ```json
   {
     "languages": {
       "Rust": {
         "format_on_save": "on"
       }
     },
     "lsp": {
       "rust-analyzer": {
         "initialization_options": {
           "check": {
             "command": "clippy"
           }
         }
       }
     }
   }
   ```

9. Use Zed's built-in Cargo tasks first. Add project tasks for existing migration checks only when needed. Keep their commands consistent with CI.
10. Run one unit test from Zed. Debug a test or binary with CodeLLDB. Confirm a breakpoint is reached and local variables can be inspected.

References: [Rust installation](https://rust-lang.org/learn/get-started/), [Zed Rust support](https://zed.dev/docs/languages/rust), and [Zed language configuration](https://zed.dev/docs/configuring-languages). Recheck settings against current documentation during implementation.

### RUST-003 — Port document processing

- **Objective:** Port Markdown and plain-text loaders; port section parsing and chunk overlap behavior; preserve metadata through all transformations; compare shared fixtures against Go; and port PDF text-layer loading with a compatible existing library.
- **Depends on:** RUST-002
- **Acceptance Checks:** `cargo test --manifest-path rust/Cargo.toml`
- **Acceptance:** Sections and chunks match the baseline fixtures exactly for deterministic fields.
- **Acceptance:** Checks cover empty input, Unicode, Vietnamese text, boundary sizes, overlap, and invalid settings.
- **Acceptance:** Page provenance is preserved through PDF sections and chunks.
- **Acceptance:** Scanned PDFs retain the baseline explicit rejection behavior.
- **Acceptance:** Any PDF extraction differences are resolved against representative fixtures before closure.

Work steps:

1. Port Markdown and plain-text loaders.
2. Port section parsing and chunk overlap behavior.
3. Preserve metadata through all transformations.
4. Compare shared fixtures against Go.
5. Port PDF text-layer loading with a compatible existing library.

Verification: Run Rust checks and the shared Go/Rust fixture comparisons without requiring Ollama or PostgreSQL for text processing.

Learning focus: Ownership, borrowing, UTF-8 boundaries, iterators, and file I/O.

### RUST-004 — Port Ollama and provider integration

- **Objective:** Implement the shared HTTP client; port embedding and generation requests; port provider registration and resolution; and preserve request settings, timeouts, cancellation, and response validation.
- **Depends on:** RUST-003
- **Acceptance Checks:** `cargo test --manifest-path rust/Cargo.toml`
- **Acceptance:** Mock-server checks confirm endpoint paths and JSON request/response contracts.
- **Acceptance:** Unknown providers fail during configuration loading as in Go.
- **Acceptance:** Malformed responses, HTTP failures, dimension mismatches, and timeouts return useful errors.
- **Acceptance:** Live embedding and generation smoke checks pass with recorded model identifiers.

Work steps:

1. Implement the shared HTTP client.
2. Port embedding and generation requests.
3. Port provider registration and resolution.
4. Preserve request settings, timeouts, cancellation, and response validation.

Verification: Run deterministic mock-server tests and Rust checks. Run a separate live Ollama smoke check. Do not require exact generated wording.

Learning focus: Async execution, serialization, trait objects or generics, and error propagation.

### RUST-005A — Port storage foundation (schema, DB client, dimension guard)

- **Objective:** Reuse the existing SQL migrations and schema; port the database client and connection pool; port the embedding-dimension guard; and verify Go/Rust data interoperability at the storage layer.
- **Depends on:** RUST-004
- **Acceptance Checks:** `cargo test --manifest-path rust/Cargo.toml`
- **Acceptance:** Rust reads Go-ingested records and Go reads Rust-ingested records.
- **Acceptance:** Dimension validation occurs before writes.
- **Acceptance:** Provenance and language/page metadata match the baseline.

Work steps:

1. Reuse the existing SQL migrations and schema.
2. Port the database client and connection pool.
3. Port the embedding-dimension guard.
4. Verify Go/Rust data interoperability.

Verification: Run integration tests against a dedicated disposable PostgreSQL/pgvector database. Inspect records. Run Rust checks.

Learning focus: Database pools, SQL types, and resource lifetimes.

### RUST-005B — Port ingestion pipeline (fingerprints, transactional replacement)

- **Objective:** Port fingerprints and unchanged-source detection, and transactional source replacement with rollback on failure, including the pre-effect ingest guard and its caller-level integration check.
- **Depends on:** RUST-005A, RUST-003
- **Acceptance Checks:** `cargo test --manifest-path rust/Cargo.toml`
- **Acceptance:** Repeated ingestion matches the baseline unchanged-source behavior.
- **Acceptance:** A forced failure during replacement rolls back the complete transaction.
- **Acceptance:** Rust ingestion rejects incomplete PDF extraction, fully scanned PDFs, and fatal loader errors before embedding or database writes. For incomplete PDFs it reports the affected pages. Existing stored data is preserved on failure.
- **Acceptance:** A caller-level test shows a mixed text/scanned PDF causes no embedding calls and no database writes.

Work steps:

1. Port fingerprints and unchanged-source detection.
2. Port transactional source replacement.
3. Reject incomplete/scanned/fatal loader outcomes before any embedding or database write (the pre-effect ingest guard), reporting the affected pages for a partial PDF.
4. Add the caller-level integration test (mixed text/scanned PDF -> no embedding calls, no database writes).
5. Verify rollback on forced failure.

Verification: Run integration tests against a dedicated disposable PostgreSQL/pgvector database. Inspect rollback outcomes. Run Rust checks. The integration check depends on RUST-003's loader status contract (`PdfExtraction.incomplete_pages`); completed loader parity is kept separate from this pending caller parity.

Learning focus: Transactions and resource lifetimes.

### RUST-006 — Port retrieval and answering

- **Objective:** Port vector search and PostgreSQL full-text search; port rank fusion, similarity filtering, deduplication, and section expansion; port reranking and fallback behavior; port context budgeting and query rewriting; port answerability checks, prompts, and citation validation/repair; and wire the Rust ingest and question-answer commands.
- **Depends on:** RUST-005B
- **Acceptance Checks:** `cargo test --manifest-path rust/Cargo.toml`
- **Acceptance:** Fixed vectors and mocked model responses produce equivalent ranking, context, and fallback behavior within the frozen comparison rules.
- **Acceptance:** Both answering and evaluation use one Rust retrieval implementation.
- **Acceptance:** Prompt construction preserves separation of instructions and untrusted retrieved text.
- **Acceptance:** Insufficient evidence produces the baseline refusal behavior.
- **Acceptance:** Citation parsing, validation, repair, and page rendering pass shared cases.
- **Acceptance:** The live ingestion → retrieval → answer path works on the test corpus.

Work steps:

1. Port vector search and PostgreSQL full-text search.
2. Port rank fusion, similarity filtering, deduplication, and section expansion.
3. Port reranking and fallback behavior.
4. Port context budgeting and query rewriting.
5. Port answerability checks, prompts, and citation validation/repair.
6. Wire the Rust ingest and question-answer commands.

Verification: Run parity checks, database integration tests, Rust checks, and a live end-to-end smoke check. Report live model variability separately from deterministic differences.

Learning focus: Composition, module boundaries, collections, and async orchestration.

### RUST-007 — Port evaluation and complete the transition

- **Objective:** Port retrieval evaluation and the bilingual experiment harness; preserve experiment arms, dataset formats, cache semantics, and generation settings; compare both implementations using the frozen corpus and configuration; update the README, command reference, setup, Makefile, hooks, and CI; verify setup from a clean checkout; and remove Go only after all required capability checks pass and repository rules permit the change.
- **Depends on:** RUST-006
- **Acceptance Checks:** `cargo test --manifest-path rust/Cargo.toml`
- **Acceptance:** All four commands have working Rust equivalents and documented invocation mappings.
- **Acceptance:** Experiment arms, deterministic metrics, and cache behavior meet the frozen parity contract.
- **Acceptance:** Generated answers meet the baseline groundedness and citation criteria.
- **Acceptance:** No required checks remain BLOCKED or unverified.
- **Acceptance:** Formatting, linting, unit, integration, and end-to-end checks pass.
- **Acceptance:** A clean checkout follows the documented setup successfully.
- **Acceptance:** Migration evidence records the final capability matrix, remaining limitations, and rollback reference.
- **Acceptance:** The final repository is Rust-only: the Rust package sits at the repository root and no Go source, `go.mod`, `go.sum`, or Go-dependent tooling (including the Go fixture generator) remains.
- **Acceptance:** Frozen fixtures and the baseline provenance (pinned Go commit SHA and source repository URL) are preserved, and the Rust suite runs with no Go toolchain installed.
- **Acceptance:** Shared assets are preserved: `migrations/*.sql`, `evals/`, `examples/`, and `docker-compose.yml`.
- **Acceptance:** CI, `Makefile`, `.zed/` tasks, SOP validation checks, and documentation describe the Rust-only layout.
- **Acceptance:** A clean checkout builds and tests with Go absent.

Work steps:

1. Port retrieval evaluation and the bilingual experiment harness.
2. Preserve experiment arms, dataset formats, cache semantics, and generation settings.
3. Compare both implementations using the frozen corpus and configuration.
4. Update the README, command reference, setup, Makefile, hooks, and CI.
5. Verify setup from a clean checkout.
6. Keep Go until the required parity checks pass; then move the Rust package from `rust/` to the repository root and remove the Go source and Go-dependent tooling. Use a separate checkout of the pinned Go commit for any future reference checks; do not retain a duplicate Go implementation in the final tree.

Verification: Run the complete Rust validation set, shared parity suite, evaluation harnesses, and clean-checkout setup. Run Go reference checks before removal. Confirm CI covers the final Rust workflow.

Learning focus: Reproducible experiments, packaging, tooling, and integration testing.

### RUST-008 — Next.js UI and Axum API

- **Objective:** Add a Next.js front end and an Axum API over the existing Rust RAG services: document upload, ingestion status, question entry, answers with source citations and page references, and health. The CLI and the API call the same Rust application services; ingestion, retrieval, and generation stay in Rust. Both the Next.js server and the Axum API bind to loopback (`127.0.0.1`) by default; `API_URL` remains a server-side setting and the browser only makes same-origin `/api` requests. Keep the database and model credentials server-side. Reuse the layout, interaction patterns, and API contracts of the `inventory` reference (Next.js -> Axum -> PostgreSQL; the browser talks only to Next.js; `/api/*` is proxied server-side), and the baseline's `rag-retrieval/v1` query contract where practical. Record intentional differences.
- **Depends on:** RUST-007
- **Acceptance Checks:** `cargo test --manifest-path rust/Cargo.toml`
- **Acceptance:** Startup commands and the browser URL are documented.
- **Acceptance:** Upload -> ingest -> question -> cited-answer verification completes end to end.
- **Acceptance:** The UI shows loading, empty, connection-failure, and extraction-error states.
- **Acceptance:** Incomplete PDFs are rejected before embedding or database writes.
- **Acceptance:** UI/API contracts are verified and existing CLI behavior is preserved.
- **Acceptance:** The final project requires no Go toolchain.
- **Acceptance:** Both the Next.js server and the Axum API bind loopback (`127.0.0.1`) by default; `API_URL` stays server-side and the browser makes only same-origin `/api` requests.

Work steps:

1. Expose the shared Rust application services so the CLI and the API call the same functions (load/upload, ingest, retrieve, answer).
2. Add the Axum API: upload, ingestion, query, and health endpoints, bound to `127.0.0.1` by default.
3. Add the Next.js UI: document upload, ingestion status, question entry, answers, source citations, and page references, proxying `/api/*` server-side; bind the Next.js server to loopback (`127.0.0.1`) as well.
4. Keep database and model credentials and `API_URL` server-side; the browser receives neither and makes only same-origin `/api` requests.
5. Verify the end-to-end flow and the loading, empty, connection-failure, and extraction-error states.
6. Record intentional differences from the reference.

Verification: Run Rust checks and the API contract tests; exercise the browser flow (upload -> ingest -> question -> cited answer) and the four UI states; confirm the CLI behavior is unchanged and no Go toolchain is required.

Learning focus: Axum handlers and shared state, server-side proxying, and UI state handling.

Intentional differences from the reference: both servers bind `127.0.0.1` (loopback-only, not `0.0.0.0`); `API_URL` is server-side only and the browser stays on same-origin `/api`; routes are RAG-specific (upload/ingest/query/health); no authentication surface in this scope. The Node.js UI cannot be an SOP acceptance check (Node.js is outside the command policy), so its verification is **operator-run evidence** recorded separately, while the Rust-side API contract is checked by `cargo test --manifest-path rust/Cargo.toml`.

## Stop conditions

Pause the affected task when:
- Required behavior cannot be established from code, tests, or product requirements.
- A deterministic parity difference remains unexplained.
- A dependency cannot preserve required behavior without a scope change.
- A destructive schema change or shared-data operation is required.
- Required services or credentials are unavailable and prevent acceptance checks.

Record the blocker, evidence, attempted resolution, and the smallest decision or action needed. Continue independent work when possible. Never mark an unverified task complete.

## Evidence and progress

Keep migration evidence under `docs/reports/rust-migration/`. Record the baseline SHA, toolchains, dependency decisions, fixtures, exact commands, results, and unresolved differences. Avoid secrets and machine-specific credentials.

| Task | Status | Evidence | Learning note |
| --- | --- | --- | --- |
| RUST-001 | PLANNED | Pending | Pending |
| RUST-002 | PLANNED | Pending | Pending |
| RUST-003 | PLANNED | Pending | Pending |
| RUST-004 | PLANNED | Pending | Pending |
| RUST-005A | PLANNED | Pending | Pending |
| RUST-005B | PLANNED | Pending | Pending |
| RUST-006 | PLANNED | Pending | Pending |
| RUST-007 | PLANNED | Pending | Pending |
| RUST-008 | PLANNED | Pending | Pending |

First learning milestone: complete RUST-001 through RUST-003, with working Rust parsing and chunking and demonstrated fixture parity. Continue to later tasks unless a stop condition applies.

## Completion report

Report completed tasks, reference and final commits, verification results, behavior differences, dependency tradeoffs, remaining limitations, and the main Rust concepts learned. State publication or merge status separately from local completion.
