# SOP Development Process — Rust migration

How Agentic SOP was applied to the Go → Rust migration in this repository, and
what it cost. Counts are reconstructed from SOP telemetry
(`.agent-sdlc/runs/**`) and the operator conversation. Nothing here is estimated:
values that cannot be established are marked UNKNOWN, and agent calls are never
equated with user turns.

- First recorded: 2026-10-17
- Last updated: 2026-10-17 (hooks finished: read-only formatting checks, ST1012 fixed, both hooks pass; controlled failures block; external development work)
- SOP: `sop` on PATH, `sop version` = `sop dev` (source HEAD `a46187b`,
  `/home/tomtran/go/bin/sop`)

## 1. Counts

Each count is defined so the categories stay separate.

### User turns

Operator messages that direct or unblock migration execution:

| # | Operator message | Role |
| - | ---------------- | ---- |
| U1 | "prep this repo for a GO to Rush conversion" | Directs the goal |
| U2 | "I installed the Rust MCP" | Unblock attempt (tooling) — no tool was actually available |
| U3 | "I just did the curl install" | Unblocks (installs the Rust toolchain) |
| U4 | attach `sop-end-to-end` skill + `docs/plans/PLAN-RUST-MIGRATION.md` | Directs SOP execution |
| U5 | "Proceed end to end with the validation and plan repair." | Directs repair; authorizes resume |
| U6 | "Also record how SOP supports the migration…" | Directs tracking |
| U7 | "Finish independent configuration… pause at the acceptance-check blocker" | Directs scope change and pause |

- **User turns total: 7.** All 7 either directed or unblocked migration work.

### Manual continuations

Operator instructions needed to *resume* execution after a SOP stop (a subset of
user turns, counted separately, not double-counted as new work):

| When | Stop being resumed | Instruction |
| ---- | ------------------ | ----------- |
| After U2 | Prerequisite gap (no Rust toolchain) | U3 (install toolchain) |
| After the plan-validation failure and the RUST-005 `CONTINUE` stop | Run had stopped | U5 (repair + resume) |
| After the acceptance-check discovery | Execution paused by design | U7 (pause and document) |

- **Manual continuations: 2 unambiguous resumes (U3, U5)**, plus U7 as a
  deliberate *course change / pause* rather than a resume. Recorded as
  **3 continuation-class instructions (1 is a pause)**.

### SOP invocations

Each actual `sop run` launch, including failed launches:

| # | Command | Outcome |
| - | ------- | ------- |
| I1 | `sop run docs/plans/PLAN-RUST-MIGRATION.md` | Plan validation failed (`summary is empty`); no work executed |
| I2 | `sop run docs/plans/PLAN-RUST-MIGRATION.md` | Compiled 7 tasks; ran RUST-001–004 (`LOCAL_DONE`); RUST-005 `CONTINUE`; process exited |

- **SOP invocations: 2** (1 failed launch, 1 execution).

### Agent calls

Two different populations, counted separately and never equated with user turns.

**SOP-internal agent calls** (reported by SOP telemetry, `sop report`):

- **11**, invocation I2 only (RUST-001: 2, RUST-002: 2, RUST-003: 2, RUST-004: 2,
  RUST-005: 2, plan-compile repair: 1).
- Invocation I1: **UNKNOWN** (not reported; it failed before execution).

**External development-agent calls** (model calls by the coding agents doing the
migration work outside `sop run`): **UNKNOWN** — not instrumented by SOP and no
countable telemetry exists for them.

### Task attempts

Initial attempts and retries, per task:

| Task | Attempts | Notes |
| ---- | -------- | ----- |
| RUST-001 | 1 | `LOCAL_DONE` |
| RUST-002 | 1 | `LOCAL_DONE`; SOP spent 1 internal *plan repair* inside synthesis |
| RUST-003 | 1 | `LOCAL_DONE` |
| RUST-004 | 1 | `LOCAL_DONE` |
| RUST-005 | 1 | `CONTINUE (INCOMPLETE_IMPLEMENTATION)` → requeued |
| RUST-006 | 0 | Never ran (blocked behind RUST-005) |
| RUST-007 | 0 | Never ran (blocked behind RUST-005) |

- **Task attempts: 5** (4 completed, 1 requeued, 2 never attempted).

### Approval requests

- **Approval requests: 0.** `sop approvals --json` was empty at every checkpoint.
  (The per-task line "human approval required before commit" is repository
  policy, not an open SOP gate.)

### Plan repairs

Changes needed to make the plan executable or verifiable:

| # | Repair | Purpose | Status |
| - | ------ | ------- | ------ |
| P1 | Canonicalize `PLAN-RUST-MIGRATION.md` to SOP's plan schema (Objective/Depends on/Acceptance fields) | Executable | Done (validated with SOP's compiler) |
| P2 | SOP internal plan repair during RUST-002 synthesis (capability status `UNKNOWN`) | Executable | Done by SOP (reported "Plan repairs: 1") |
| P3 | Add `acceptance_checks` to each stage so criteria are verifiable | Verifiable | **Blocked** (see §6) |
| P4 | Split RUST-005 into RUST-005A (storage) + RUST-005B (ingestion); RUST-006 now depends on RUST-005B | Bounded tasks | Done (validated; 8 stages, all 5 criteria preserved) |

- **Plan repairs: 3 done (P1, P2, P4); 1 blocked (P3).**

### Configuration repairs (tracked separately from plan repairs)

| # | Repair | Status |
| - | ------ | ------ |
| C1 | Add Rust build/test/Clippy/fmt to `.agent-sdlc/config.yaml` validation (Go checks preserved) | Done |
| C2 | Fix `rust/rust-toolchain.toml` pin (channel `1.83.0` + nonexistent `lldb-preview` component) | Done by agent (config repair) — flagged |
| C3 | Regenerate the corrupt `rust/Cargo.lock` (fabricated `thiserror` checksum) | Done — regenerated against crates.io; real checksums |
| C4 | Fix the Rust build/test/lint gaps (`E0277` `Debug` derives; unused `page_id`; missing `PartialEq` test comparisons; `take_while` unit-parsing bug in `parse_duration`; clippy `type_complexity`) | Done — `cargo build`/`test`/`clippy`/`fmt` all pass |

## 2. How SOP was applied

| Stage | What happened |
| ----- | ------------- |
| Discovery | Operator named the plan explicitly; SOP resolved `docs/plans/PLAN-RUST-MIGRATION.md` as the source and assigned Plan ID `plan-rust-migration`. |
| Preflight | `sop status` reported "project is not initialized"; `sop run` bootstrapped `.agent-sdlc/` (config + state.db). No pending approvals. |
| Plan compilation | SOP deterministically parsed the Markdown (`planner.PlanFromMarkdown`) and validated it (`Plan.Validate`). I1 failed here; after P1 it compiled 7 stages. |
| Task scheduling | `taskbuilder` persisted 7 tasks with the dependency chain RUST-001→…→RUST-007; executed in dependency order. |
| Model routing | Provider `ollama`, model `deepseek-v4.1-flash:cloud`, source `configuration`, harness `tool`. No model-routing layer configured (the `models:` block is commented out); class not recorded → UNKNOWN. |
| Implementation | IMPLEMENT ran per task (RUST-001–004); evidence in `.agent-sdlc/runs/RUST-00X/`. |
| Validation | Invocation I2 ran the configured validation (Go-only at the time): 4 validation runs; it never exercised the Rust deliverable. C1 (2026-10-17) added Rust build/test/Clippy/fmt; the full configured sequence now runs green locally (Go + Rust). No SOP invocation has re-run validation since C1. |
| Review | Engine `self`; 0 findings, 0 review runs. |
| Continuation | RUST-005 classified `CONTINUE (INCOMPLETE_IMPLEMENTATION, HIGH)`; autonomy `BALANCED` → `AUTO_CONTINUE`; requeued; then the invocation exited. |
| Checkpoint recovery | SOP state persisted in `.agent-sdlc/state.db`; plan state `ACTIVE`. Not resumed (paused at the acceptance-check gap). |

## 3. Per-task record

"Reported completion" is SOP's task state; "verified acceptance" is whether the
objective's required criteria were proven by an executed passing check.

| Task | Attempts | Validation | Disposition (SOP) | Verified acceptance | Human intervention | Evidence |
| ---- | -------- | ---------- | ----------------- | ------------------- | ------------------ | -------- |
| RUST-001 | 1 | Go validation PASS | `LOCAL_DONE` (outcome `PARTIAL`) | **No** — criteria unverifiable (no checks); and Rust not built | Plan canonicalization (P1) | `.agent-sdlc/runs/RUST-001/` (`report.md`, `outcome.json`, `validation.json`, `gate.json`, `diff.patch`) |
| RUST-002 | 1 | Go validation PASS | `LOCAL_DONE` (outcome `PARTIAL`) | **No** — criteria unverifiable | None during run | `.agent-sdlc/runs/RUST-002/` |
| RUST-003 | 1 | Go validation PASS | `LOCAL_DONE` (outcome `PARTIAL`) | **No** — criteria unverifiable | None during run | `.agent-sdlc/runs/RUST-003/` |
| RUST-004 | 1 | Go validation PASS | `LOCAL_DONE` (outcome `PARTIAL`) | **No** — criteria unverifiable | None during run | `.agent-sdlc/runs/RUST-004/` |
| RUST-005 | 1 | None | `CONTINUE`, requeued | **No** — no change made | None | `.agent-sdlc/runs/RUST-005/` (`classification.json`, `continuations.txt`, `report.md`) |
| RUST-006 | 0 | — | `PLANNED` | No | — | — |
| RUST-007 | 0 | — | `PLANNED` | No | — | — |

Note: every `LOCAL_DONE` was `PARTIAL`. Because `lifecycle.verdict_gate` is off,
a `PARTIAL` verdict still yields `LOCAL_DONE` — so reported completion is **not**
verified acceptance anywhere in this migration.

## 4. Observed benefits

- One command (`sop run <PLAN>`) bootstrapped state, compiled the plan, generated
  the task graph, scheduled it in dependency order, and executed four tasks with
  per-task evidence artifacts — with no per-task operator orchestration.
- Deterministic plan validation prevented execution on a malformed plan (I1).
- Per-task artifacts (`report.md`, `outcome.json`, `validation.json`, `gate.json`,
  `diff.patch`, `trace.json`) made the run auditable after the fact.
- SOP state (`state.db`) preserved progress across the stop without regeneration.
- Failure classification was specific and actionable (the RUST-005 `CONTINUE`
  reason named the exact validation-contract gap).

## 5. Observed friction

- **Initial schema failure.** I1 rejected the plan (`summary is empty`): the
  hand-written plan was recognized-but-invalid, and SOP (correctly) refuses to
  silently repair a recognized plan. Fixed by P1.
- **Go-only validation.** The configured validation ran only Go checks, so it
  never touched the Rust deliverable. Addressed by C1 (config) — but see §6.
- **Missing acceptance checks.** No stage declared `acceptance_checks`, so every
  required criterion was "unverifiable: no deterministic check supplied."
- **PARTIAL reaching LOCAL_DONE.** With the verdict gate off, four unverified
  tasks became `LOCAL_DONE`. This is the core risk: reported ≠ verified.
- **RUST-005 continuation stop.** The task exceeded its mutation window and was
  requeued; downstream tasks never ran.
- **Unbuildable Rust artifacts (resolved).** As produced by the run, the Rust code
  did not compile (`E0277`), shipped a fabricated `Cargo.lock`, and carried a
  broken toolchain pin. The Go-only validation let this reach `LOCAL_DONE`.
  Repaired 2026-10-17 (C2, C3, C4) and supplemented by Rust checks (C1). This is
  direct evidence that a `LOCAL_DONE` with no Rust check was not trustworthy.

## 6. Blocker: native Rust acceptance checks (tool capability gap)

**Classification: tool capability gap.** SOP cannot execute `cargo` as a
deterministic acceptance check, so no Rust criterion can be proven by a passing
check on the current build. Confirmed against the installed binary after the
update finished.

- **Version:** `sop version` → `sop dev`; binary `/home/tomtran/go/bin/sop`,
  mtime 2026-10-17 09:27; source HEAD `a46187b`.
- **Policy code paths:**
  - `internal/commandpolicy/commandpolicy.go` — `Classify` recognizes only `git`
    (read-only subcommands) and `go` (`test`, `build`, `vet`, `fmt`, `list`) as
    `SAFE`; the `default` branch returns `REQUIRES_APPROVAL`. A project `Policy`
    can only *tighten* (`Approve`/`Deny`), never widen.
  - `internal/cli/outcome.go` — `authorizeAcceptanceCommand` tokenizes with
    `toolharness.SplitCommand` (which rejects shell syntax: chaining, pipes,
    redirection, substitution, globs), rejects dangerous flag forms, then requires
    `commandpolicy.Classify == SAFE`. Non-SAFE ⇒ the check is recorded as a
    **boundary** and never executed.
  - `internal/outcome/outcome.go` — an unmet boundary criterion sets the outcome to
    `HOLD`, and `ValidateWaiver` states "a HOLD (an authority boundary crossed) is
    not waivable"; `ResolveLifecycle` maps `HOLD` → `NEEDS_HUMAN`.
- **Reproduction (deterministic):**
  ```
  sop version                                              # sop dev
  grep -n 'case "git"\|case "go"\|default:' internal/commandpolicy/commandpolicy.go
  # classify a candidate (source-level, no run required):
  #   commandpolicy.Classify(["cargo","test"]) -> REQUIRES_APPROVAL
  #   commandpolicy.Classify(["go","test","./..."]) -> SAFE
  #   toolharness.SplitCommand("cd rust && cargo test") -> error (shell metacharacters)
  ```
- **HOLD behavior:** a `cargo …` acceptance check authorizes to a boundary, never
  runs, forces `HOLD`, and `HOLD` is not waivable → `NEEDS_HUMAN`. It can never be
  a passing check.

Consequences recorded, not worked around:

- Cargo commands are **not** disguised as Go commands, not wrapped in a shell to
  bypass policy, and no Rust criterion is marked accepted from an unrelated Go
  check. Those would each be an integrity violation.

Task state and run evidence were preserved (`.agent-sdlc/state.db` and
`.agent-sdlc/runs/**` untouched). See
[`sop-enhancement-rust-acceptance-checks.md`](sop-enhancement-rust-acceptance-checks.md)
for the proposed SOP enhancement.

## 7. Pending / unverified

- RUST-001–004: reported `LOCAL_DONE`; **acceptance unverified** (no passing
  checks). The Rust artifacts they produced did not build; repaired afterwards
  (C3, C4).
- RUST-005A/B–007: not completed (RUST-005 split into two bounded stages, P4).
- Rust checks (build/test/Clippy/fmt): **passing** (C3, C4 done; C1 wired them
  into validation): 64 lib + 10 integration + 1 doc test pass; clippy `-D
  warnings` clean; `cargo fmt --check` clean.
- Required Go reference checks: run during validation and passing.
- Deterministic parity evidence: **none produced** (requires a passing Rust build).
- Acceptance-check capability: blocked (§6).

## 8. Cost note

No time-saving claim is made: there is no measured before/after comparison for
this migration. The counts in §1 are effort accounting, not a productivity delta.

## 9. Update log

### 2026-10-17 — validation repair pass (this update)

Trigger: the operator re-issued the U5+U6 directive (repair + tracking); the
earlier repair pass had stopped at the acceptance-check gap.

- **C1 re-applied.** `.agent-sdlc/config.yaml` validation now runs Rust build,
  test, Clippy, and formatting from `rust/` alongside the unchanged Go checks.
- **C3, C4 done.** Regenerated the fabricated `Cargo.lock`; fixed the compile,
  test, Clippy, and formatting gaps in `rust/`.
- **P4 done.** Split RUST-005; recompiled the plan with SOP's own
  `planner.PlanFromMarkdown`/`Plan.Validate` → 8 stages, chain
  RUST-004→005A→005B→006→007, all 5 criteria preserved.
- **C1–C4 verified locally:** the full configured validation sequence passes
  (`go build/test/vet`, `gofmt`, `cargo build/test/fmt/clippy`).
- **Counts unchanged:** SOP invocations still 2; agent calls still 11; task
  attempts still 5; approval requests 0. No `sop run` was launched in this pass
  because the acceptance-check blocker (§6) still prevents a verified completion,
  so a run could only re-produce `PARTIAL`/`HOLD`.
- **Still blocked:** P3 (acceptance checks). See
  [`sop-enhancement-rust-acceptance-checks.md`](sop-enhancement-rust-acceptance-checks.md).

## 10. Native cargo acceptance-check enhancement (implemented, pending installation)

### 10.1 Base and upstream

- Worktree `~/agentic-workspace/agentic-sop-rust-acceptance`, branch
  `sop-rust-acceptance-checks`, base `a46187b`
  (`a46187bbdc0179964e48e04fcca5c27982a5d145`). Uncommitted work preserved — the
  worktree was **not** rebased.
- `git fetch origin` advanced `origin/main` to `286fa14`. The only commits missing
  from the base are the deceleval-determinism fix (PR #13):
  `internal/deceleval/*` and `evals/decision-model/README.md`.
  **No upstream change touches `commandpolicy`, `config`, or `cli`**, so no
  relevant fix is missing.

### 10.2 Change (branch diff, 8 files + 2 new tests)

| File | Change |
| --- | --- |
| `internal/commandpolicy/commandpolicy.go` | `Policy.AllowCargo` + `isAllowedCargo` (scoped `cargo build\|test\|clippy`, and `fmt` only with `--check`) |
| `internal/config/config.go` | `validation.acceptance_checks.cargo` (default false) + `Config.CargoAcceptanceChecksEnabled()` |
| `internal/cli/outcome.go` | `deriveOutcome`/`authorizeAcceptanceCommand` take the policy (from the already-threaded `cfg`); cargo `--config`/`-Z`/`--target-dir` rejected; `acceptanceCheckPolicy` |
| `internal/commandpolicy/commandpolicy_test.go` | +3 tests (allow-list, tighten-wins, default never widens) |
| `internal/cli/cargo_acceptance_check_test.go` | authorization seam |
| `internal/cli/cargo_acceptance_flow_test.go` | real-cargo outcome flow (`-tags acceptance`) |
| `docs/specs/SECURITY.md`, `docs/plans/ACCEPTANCE-CHECKS-AUTHORING.md` | documentation |

### 10.3 Verification

- `go build ./...`, `gofmt -l` (clean), `go vet ./...`: pass.
- Affected packages in isolation: `commandpolicy`, `config`, `outcome`,
  `taskbuilder`, `toolharness`, `planner`: **ok**.
- End-to-end outcome flow (real cargo, `-tags acceptance`): **PASS** — passing
  check → `VERIFIED` (with argv/exit evidence); failing check → `PARTIAL` (check
  unmet); no check → `PARTIAL` (`objective-acceptance` unmet); rejected commands
  (`cargo run\|install\|fmt`, shell syntax) → `HOLD` boundary; un-opted-in cargo
  check → `HOLD`; a `go` check → `VERIFIED` (unchanged).
- `internal/cli` whole-package run is very slow in this environment and did not
  complete in the observation window. It stalls once at
  `TestRunTasksMalformedPlan` in SQLite `createSchema` (`ExecContext`); that test
  passes in isolation in this worktree (1.4s) and on `main` (0.55s), so the stall
  is environmental, not from this change. The affected-package and flow tests
  above are the evidence used.

### 10.4 Intervention counts (this phase)

| Count | This phase | Total |
| --- | --- | --- |
| User turns | 1 | 8 |
| Manual continuations | 1 | 4 (1 is a pause) |
| SOP invocations | 0 | 2 |
| Agent calls (SOP-reported) | 0 | 11 |
| Task attempts | 0 | 5 |
| Approval requests | 0 | 0 |
| Plan repairs | 0 | 3 done, 1 blocked |

No `sop run` was launched: this phase is SOP **source** work, not a run.

### 10.5 Residual

- The enhancement spec's §1 working-directory field is **not implemented**:
  `--manifest-path` covers a crate in a subdirectory argv-safely, so no
  shell/`cd` workaround is needed. Path *containment* for that argument **is**
  implemented (see 10.6).
- Not committed, pushed, merged, or installed.

### 10.6 Verification added (path containment, nested crates, never-execute)

- **Path containment.** `rejectUnsafeCargoPaths` + `containedRepoPath`
  (`internal/cli/outcome.go`) confine `--manifest-path` to the repo root: it
  refuses an empty value, an absolute path, any `..` component, and a path whose
  **nearest existing ancestor** symlink-resolves outside the root. `--manifest-path`
  is the only path-selecting cargo argument that can reach execution
  (`--target-dir`, `--config`, `-Z` are rejected as dangerous forms). Both
  `--manifest-path VALUE` and `--manifest-path=VALUE` are handled.
  Test: `TestAuthorizeCargoManifestPathContainment` — allowed (in-repo, both forms)
  and rejected (absolute, `..`, symlink escape, empty, missing value): PASS.
- **Nested crate, all four commands.** `TestCargoNestedCrateCommandsVerify` runs a
  real cargo against a crate at `rust/` and asserts `VERIFIED` for
  `cargo build|test|clippy --manifest-path rust/Cargo.toml` and
  `cargo fmt --manifest-path rust/Cargo.toml --check` (plus the `=` forms): PASS.
- **Never execute.** `TestRejectedCargoCommandNeverExecutes` puts a marker-writing
  fake `cargo` on PATH: the allowed control runs (marker appears); `cargo run`,
  `cargo build --config x=y`, and a `../` manifest path do not: PASS.
- **Default-off and Deny-wins.** Default-off is asserted at the authorization seam
  and at the outcome level; Deny-wins over `AllowCargo` is asserted in
  `TestPolicyAllowCargoOnlyWidensScopedShapes`.

### 10.7 Unresolved: full `internal/cli` suite stalls

- **Command:** `go test -count=1 -tags acceptance -timeout 8m ./internal/cli/`
  (in the worktree) → `FAIL ... 480.253s`.
- **Stack:** `store.Open → Store.migrate → createSchema → database/sql.(*DB).ExecContext`
  (`internal/store/sqlite.go:40/79/125`), driver `modernc.org/sqlite`. The
  blocking test varies between runs (`TestRunTasksMalformedPlan`, then
  `TestMemoryAddAndList`).
- **Cause: UNRESOLVED.** Established evidence: it **reproduces on the clean
  `main` worktree** (`a46187b`, no enhancement changes) with the same stack, so it
  is pre-existing and environmental, not from this change; individual tests pass
  in isolation (`TestRunTasksMalformedPlan` 1.4s here, 0.55s on `main`); it is not
  file-descriptor related (`ulimit -n` = 1048576) and not fixed by `-parallel 1 -p 1`;
  the `./...` suite and the full `cli` package both exceed the window. No
  established root cause.
- **Impact:** the full `internal/cli` package suite cannot be run to completion in
  this environment. Affected-package coverage is provided by the isolated,
  green runs (`commandpolicy`, `config`, `outcome`, `taskbuilder`, `toolharness`,
  `planner`) and the targeted `cli` tests (cargo authorization, path containment,
  never-execute, and the acceptance-tagged cargo flow), which all pass.

### 10.8 Default-deny cargo argument policy (review findings F1/F2/F3)

- **F1/F2 resolved.** `internal/commandpolicy` now validates the whole argv against
  a **default-deny allow-list** (`cargoPolicies` / `cargoFlagsAllowed`):
  `build`/`test`/`clippy` accept only `--locked`, `--all-targets`, and
  `--manifest-path` (each at most once; `clippy` may end in the exact suffix
  `-- -D warnings`); `fmt` requires `--check` and accepts `--all` and
  `--manifest-path`. Everything else is refused: `--target` (which also names a
  custom target-spec path), `--target-dir`, `--artifact-dir`, `--config`, `-Z`,
  unknown flags, a `+toolchain` override, positional arguments, a repeated flag,
  and any argument after `--`. The redundant cargo flag block was removed from
  `rejectDangerousForm`; the allow-list is the single source of truth, and under it
  `--manifest-path` is genuinely the only path selector that can reach execution.
  Verified by table-driven `TestPolicyAllowCargo` (42 cases) and the extended
  never-execute test.
- **F3 documented, not wired.** No configuration supplies a project
  `Policy.Approve`/`Deny` for acceptance checks, so `docs/specs/SECURITY.md` now
  states it MUST be honored *when present* and MUST NOT be described as enforced
  today; Deny-wins remains tested at the policy level only. No new policy
  configuration system was added.
- The SQLite full-suite stall remains the unresolved baseline failure (§10.7 /
  [`cli-fullsuite-timeout-evidence.md`](cli-fullsuite-timeout-evidence.md)); it was
  not re-run.

## 11. RUST-003 fixture parity (this phase)

Deterministic parity evidence for the document-processing port; see
[`RUST-003-fixture-parity.md`](RUST-003-fixture-parity.md) for the full matrix.

- **Expected outputs come from the pinned Go reference.** `tools/genfixtures`
  (a read-only consumer of `internal/document`, `internal/loader`,
  `internal/chunking`) writes `rust/fixtures/parity/section_chunk_parity.json`,
  recording provenance (`go1.27.1`, commit `d72fb39`). Expected values are never
  edited to match Rust.
- **Repairs (implementation, not fixtures):** R1 section parser drops the
  pre-heading block; R2 chunker is word-based (was byte-based) and returns no
  chunks for `size == 0` or `overlap >= size`; R3 plain-text loader yields one
  section titled by file name; R4 PDF sections titled `Page N`.
- **Result: 18/18 cases match** the reference on section title/text, chunk
  content/boundaries, ordering, document-global index, page metadata, and loader
  status/incomplete pages (`cargo test --test section_chunk_parity`).
  `source`/`language` propagation is checked separately.
- **PDF extraction status now matches (R5).** `load_pdf` returns
  `PdfExtraction { sections, incomplete_pages }`, mirroring Go's
  `([]Section, error)` shape without losing readable pages: a partial extraction
  keeps its sections **and** names the OCR pages (`pdf-mixed` → sections
  `[Page 1]`, `incomplete_pages [2]`), a fully scanned PDF is `Err(Scanned)`, and a
  hard failure is `Err(Invalid)`. Asserted per case by
  `pdf_extraction_status_matches_reference`; the difference is not documented away.
- **Pre-effect guard + caller check (RUST-005B).** Added `rust/src/ingest.rs`
  (`gate_pdf` / `gate_sections`): fatal loader errors, fully scanned PDFs, partial
  PDFs (reporting the affected pages), and empty sources are rejected **before** any
  embedding call or database write. `rust/tests/ingest_caller_rejects_incomplete_pdf.rs`
  proves a mixed text/scanned PDF causes zero embedding calls and zero database
  writes (control: a complete PDF embeds each chunk and writes once). The plan gained
  an explicit RUST-005B acceptance criterion and a cross-task dependency
  (`Depends on: RUST-005A, RUST-003`), re-validated with SOP's compiler (8 stages,
  VALID). Loader parity (complete) is kept separate from caller parity (the
  production ingest binary is still a stub; wiring is RUST-005B).
- **Checks:** `cargo fmt/test/clippy` and Go `build/gofmt/vet/test` all pass.
- **Counts unchanged:** SOP invocations 2; SOP-internal agent calls 11 (external
  development-agent calls UNKNOWN); task attempts 5; approval requests 0; plan
  repairs 3 done + 1 blocked.
- **SOP state (read with `sop status` / `sop verify`, never edited):** RUST-001–004
  are `LOCAL_DONE`; RUST-003's recorded outcome is `PARTIAL` (its deterministic-
  checks gate passed, but all five criteria are "unverifiable: no deterministic
  check supplied"), so SOP does not count its acceptance as verified. This
  fixture-parity evidence is **external** to SOP's acceptance path; nothing is
  marked `VERIFIED`. (An earlier draft of this section mis-stated the lifecycle as
  `PLANNED`; that value is only the rendered plan's evidence table, not the
  persisted state.)

## 12. SOP Cargo-support integration onto current origin/main

- **Source SHAs.** Reviewed patch base `a46187bbdc0179964e48e04fcca5c27982a5d145`;
  integration base (current `origin/main`) `a9f0495d4d47be4db708101b430a848c03a03cbf`
  (32 commits ahead of the patch base).
- **Integration.** A clean, isolated worktree (`agentic-sop-cargo-integration`,
  branch `sop-cargo-integration`, based on `origin/main`) received the reviewed
  patch. `git apply` was **clean — no conflicts**, and the build required **no
  changes beyond the original patch** (upstream did not touch
  `deriveOutcome`/`authorizeAcceptanceCommand`/`commandpolicy`, so there is no
  signature drift). The original worktree `agentic-sop-rust-acceptance` is intact.
- **Integrated patch.** `docs/reports/rust-migration/sop-cargo-support-integrated.patch`
  — base `a9f0495`, 12 files, 737(+)/16(−), SHA-256
  `48530f34c6998c13bfab3c387c182f6697df87b4cea53beb36122b677e964ac6`. The original
  patch (`sop-cargo-support.patch`, SHA-256 `79dd707e…b893`) remains valid against
  its base `a46187b`.
- **Checks (integrated).** `go build ./...` OK; `go vet` OK; `gofmt -l` clean for
  the touched files; `commandpolicy` + `config` ok; focused `cli`
  (`TestAuthorizeCargo*`, `TestContainedRepoPathRejectsWhitespace`,
  `TestCargoManifestPathSymlinkEscapeRejected`, `TestRejectedCargoCommandNeverExecutes`)
  PASS; acceptance-tagged `TestCargoNestedCrateCommandsVerify` +
  `TestCargoAcceptanceCheckOutcomeFlow` + `TestOperatorAcceptanceACC*` ok. Windows
  **compilation** (`GOOS=windows go build`/`go vet`) OK; Windows **runtime** is not
  exercised here.
- **Full `internal/cli` suite (once).**
  `go test -count=1 -tags acceptance -timeout 8m ./internal/cli/` →
  `FAIL … 480.430s`. Stack: blocked in the **`fsync` syscall**
  (`modernc.org/libc.Xfsync` → `sqlite._full_fsync` → `_syncJournal` →
  `_pagerCommitPhaseOne`) beneath `store.createSchema`. `internal/store` is
  unchanged upstream. Refined hypothesis: a blocking `fsync` (I/O), not a
  deadlock. Not re-run; see
  [`cli-fullsuite-timeout-evidence.md`](cli-fullsuite-timeout-evidence.md).
- **Counts.** SOP invocations 2; SOP-internal agent calls 11; external
  development-agent calls UNKNOWN; task attempts 5; approvals 0; plan repairs 3
  done + 1 blocked.
- **Remaining gaps.** The full `cli` suite still stalls (fsync); Windows runtime is
  unverified; RUST-008 and the rest of the Rust migration remain unimplemented and
  paused. Migration changes stay separate from this SOP enhancement.

### 12.1 Candidate binary smoke test (disposable project)

- **Equivalence.** The integrated patch's 12 resulting files are byte-identical to
  the reviewed patch's, except `internal/config/config.go`, where the only
  difference is upstream's unrelated one-line doc-comment edit (`test -z "$(gofmt
  -l .)"` → `gofmt -l .`) at ~line 1082; the patch's own hunk is identical in both
  (7 occurrences). So the changes are equivalent, with only base/context differences.
- **Candidate binary.** Built with `go build -o /tmp/sop-candidate ./cmd/sop` from
  the integrated worktree (branch `sop-cargo-integration`, base `a9f0495` +
  integrated patch). SHA-256
  `1178a54c168883c801a30092bebf9fec3ca8903d6fb14248cee0541df18596af`. The installed
  binary (`~/go/bin/sop`, SHA-256 `ed6dc1519c221ab1f0e8df1748bd4adbdf08c6eb94dbd9e09013f8ffd33353ef`)
  was **not** replaced.
- **Disposable project.** `/tmp/sop-cargo-trial` (throwaway git repo, no commit).
  Config opts in with `validation.acceptance_checks.cargo: true`; the single stage
  uses `Execution: verify-first`, so no model is invoked.
- **Smoke results.**
  - cargo enabled + valid check → **VERIFIED**; `check-1` argv
    `["cargo","test","--manifest-path","rust/Cargo.toml"]`, exit 0; **agent calls
    0** ("verified first").
  - cargo **default-off** → **HOLD**: boundary `check-1` "not authorized
    (REQUIRES_APPROVAL)".
  - rejected commands (enabled) → **HOLD**: `cargo run` (command rejection) and
    `cargo test --manifest-path ../Cargo.toml` (path containment: "traverses to a
    parent directory").
  - outcome recording confirmed via `.agent-sdlc/runs/TRIAL-001/outcome.json`
    (per-criterion `Command`/`Boundary`/`Met`).
- **Gaps retained.** Full `cli` suite fsync timeout; Windows runtime unverified.
  The candidate is not installed; no commit/push/merge.

### 12.2 Local installation (trial)

- **Installed** the exact verified candidate over `~/go/bin/sop`.
- **Backup / rollback point:** `~/go/bin/sop.pre-cargo-support-20261017T183800Z`,
  SHA-256 `ed6dc1519c221ab1f0e8df1748bd4adbdf08c6eb94dbd9e09013f8ffd33353ef` (the
  pre-install binary; created with `cp -p`, no overwrite).
- **Install method:** copied the candidate beside the target, set `0755`, then
  atomically renamed over `~/go/bin/sop`. Installed SHA-256
  `1178a54c168883c801a30092bebf9fec3ca8903d6fb14248cee0541df18596af`; `sop version`
  prints `sop dev`.
- **Post-install smoke (installed binary):** enabled + valid check → **VERIFIED**
  (`check-1` argv `["cargo","test","--manifest-path","rust/Cargo.toml"]`);
  default-off → **HOLD** ("not authorized (REQUIRES_APPROVAL)"); rejections →
  **HOLD** (`cargo run`; `--manifest-path ../Cargo.toml` "traverses to a parent
  directory").
- **Rollback:** `cp ~/go/bin/sop.pre-cargo-support-20261017T183800Z ~/go/bin/sop`.
- **Gaps retained.** Full `cli` suite fsync timeout; Windows runtime unverified.

## 13. Migration resume with the installed candidate (this run)

- **Binary.** Installed `~/go/bin/sop`, SHA-256
  `1178a54c168883c801a30092bebf9fec3ca8903d6fb14248cee0541df18596af` (verified).
- **Branch.** Migration work moved to branch `rust-migration` (not `main`); all
  uncommitted work preserved.
- **Config.** `validation.acceptance_checks.cargo: true` enabled for this project.
- **Plan.** Executable acceptance checks added to all 9 stages and mapped to their
  criteria (`cargo test --manifest-path rust/Cargo.toml [--test <name>]`; RUST-001
  uses the Go reference checks). Re-validated: 9 stages, **VALID**. Build success is
  not treated as behaviour parity — RUST-003/004/005B map to the parity and caller
  tests, not to `cargo build`.
- **Policy limitation.** RUST-008's Next.js UI has **no authorizable acceptance
  check**: the command policy allows only `go`/read-only `git`/scoped `cargo`, so
  Node.js/`next` checks cannot be declared; only the Rust-side API contract can be
  checked (`--test api_contract`). Reported, not bypassed.
- **Reconcile (operator-directed).** `sop reconcile … --accept-changed RUST-001..004`:
  added `RUST-005A`, `RUST-005B`, `RUST-008`; removed `RUST-005`; updated
  `RUST-006`, `RUST-007`; accepted the changed definitions of the executed
  `RUST-001`–`RUST-004`.
- **Run.** `sop run docs/plans/PLAN-RUST-MIGRATION.md` executed `RUST-005A` →
  **`FAIL`/`BLOCKED (IMPLEMENT_NO_PROGRESS, HIGH)`**: provider
  `ollama`/`deepseek-v4.1-flash:cloud`, 11 tool calls, **0 repository mutations, 0
  changed files**, `termination=no_progress`; validation **NOT RUN**. SOP did not
  auto-continue (it would repeat under unchanged conditions) and did not retry.
  Artifact: `.agent-sdlc/runs/RUST-005A/report.md`.
- **Approvals.** None pending (`sop approvals --json` empty); the block is operator
  intervention, not an approval gate.
- **Counts (this run).** SOP invocations +1 (total **3**); SOP-internal agent calls
  +2 (total **13**); task attempts: `RUST-005A` ×1 (blocked); approvals 0.
- **Blockers.** (1) `IMPLEMENT_NO_PROGRESS` — the model produced no changes for
  RUST-005A; a retry may succeed. (2) **Environment** — `docker` is missing and
  Postgres `:5434` is unreachable, so the pgvector storage/ingestion acceptance
  checks cannot pass in this environment.
- **Gaps.** `RUST-001`–`RUST-004` remain `LOCAL_DONE`/`PARTIAL` (not re-verified);
  `RUST-005B`–`RUST-008` unstarted; full `cli` fsync timeout; Windows runtime
  unverified; the UI is not built, so no browser URL is reported. No commit, push,
  or merge.

## 14. Blocker-resolution attempt (this run)

- **PostgreSQL inspection.** A native PostgreSQL 16 cluster is online at
  `127.0.0.1:5432` (owner `postgres`). The configured `:5434` is the Docker
  `pgvector/pgvector:pg17` image, and **Docker is not installed** — so `:5434` is
  not the correct endpoint here; `:5432` is the suitable instance.
- **Credentials blocker.** No `~/.pgpass`, no `PG*` env; peer role `tomtran` does
  not exist; `sudo` requires a password; `/etc/postgresql/16/main/pg_hba.conf` is
  unreadable; the configured `myuser:mypassword` fails. I therefore **cannot
  authenticate**, so a dedicated migration database cannot be created. Stopped per
  the privileges/credentials stop condition. Existing databases/data were not
  touched; Docker was not installed to satisfy the original assumption.
- **Acceptance checks fixed (step 3).** `--test <name>` is outside the allowed Cargo
  flag set (policy allows only `--locked`, `--all-targets`, `--manifest-path`), so
  those checks would have been unauthorized boundaries. Replaced with the allowed
  whole-suite `cargo test --manifest-path rust/Cargo.toml`. Re-validated with SOP's
  compiler and the command policy: plan **VALID**; all 10 acceptance commands
  **SAFE**. Criteria were not weakened (the whole suite is at least as strong as a
  single test target).
- **Commands execute (step 4).** `go build ./...`, `go test ./...` (16 ok),
  `cargo build --manifest-path rust/Cargo.toml`, `cargo test --manifest-path
  rust/Cargo.toml` (73 lib + 3 caller + 10 integration + 5 parity + 1 doc) — all
  exit 0.
- **Revalidation unsupported (step 5).** No supported SOP operation re-runs a
  `LOCAL_DONE` task: `retry` requeues only `BLOCKED`; `verify` is read-only;
  `reconcile --accept-changed` records the change without re-executing ("SOP will
  not silently replace the definition of an executed task"). RUST-001–004 keep
  their `PARTIAL` verdicts; their new checks are recorded but unexecuted.
  `state.db` was not edited; no verdict invented; manual Zed checks stay pending.
- **RUST-005A no-progress (step 6).** 11 tool calls, 0 repository mutations,
  `termination=no_progress` — the model performed only discovery and cargo
  commands. Proposed bound/split (to apply once a database is available):
  **RUST-005A1** schema + migrations + dimension guard (unit-only);
  **RUST-005A2** DB client/pool + Go↔Rust interop (integration). Requirements and
  dependencies preserved. Not restructured now — the database dependency still
  blocks acceptance.
- **UI (step 8).** RUST-008's Node.js UI cannot be an SOP acceptance check; recorded
  as operator-run evidence (plan note added). No browser URL: the UI is not built
  and the end-to-end flow is unverified.
- **Counts (this run).** SOP invocations +0 (reconcile is not a run); SOP-internal
  agent calls +0; task attempts +0; approvals 0; plan repairs +1 (acceptance checks
  re-authored to the allowed flag set).
- **Stop condition.** Missing database credentials/privileges (a listed stop
  condition). Next action is operator-supplied: create a Postgres role/database (or
  provide credentials) so the migration database and its acceptance checks can run.

## 15. UI plan (PLAN-RAG-WEB-UI) — canonicalized, blocked on the active plan

- **UI plan canonicalized.** `docs/plans/PLAN-RAG-WEB-UI.md` did not compile as a SOP
  plan (`project is empty`, `criteria=0`, `deps=[]`). It was converted to the
  canonical schema (`## Project`/`## Summary` + `## Tasks` stages with inline
  `- **Objective:**` / `- **Depends on:**` / `- **Acceptance:**`), preserving every
  criterion verbatim; the only authorizable check added was
  `cargo test --manifest-path rust/Cargo.toml` (on UI-003). Re-validated: 5 stages,
  deps UI-001→UI-002/UI-003, UI-002+UI-003→UI-004, UI-004→UI-005, **VALID**.
- **SOP invocation** `sop run docs/plans/PLAN-RAG-WEB-UI.md` → **`NEEDS_HUMAN: a
  different plan is already active`**. Active `plan-rust-migration` still has
  unresolved work; SOP will not mix two plans and will not switch away from
  unfinished work. SOP's own resolution: complete or reconcile the active plan, then
  rerun. This is a genuine human boundary — **not crossed**. UI-001/UI-002 were not
  executed, and no manual implementation was substituted for the delegated work.
- **Policy limitation (UI).** UI-001/UI-002 carry **no SOP-authorizable acceptance
  check** (Node.js is outside the command policy). Their verification is operator-run
  evidence and is not counted as SOP-verified.
- **Verification-only follow-up tasks (supported).** SOP supports verification-only
  stages via `Execution: verify-first` + deterministic `acceptance_checks`: such a
  stage runs the checks and derives the outcome without invoking a model. Proposal
  (**not applied**): add `RUST-001V..004V` verify-first stages carrying the recorded
  checks, so RUST-001–004 criteria can be re-verified without rerunning
  implementation. Applying it needs a reconcile of the active plan (operator
  decision); `state.db` was not edited.
- **Database setup instructions prepared** at
  `docs/reports/rust-migration/database-setup.md` (dedicated `rag_migration` role +
  `rag_migration_test` database on `127.0.0.1:5432`, pgvector required). The
  privileged steps were not executed; no passwords requested in chat.
- **Counts (this run).** SOP invocations +1 (total **4**; the UI run stopped at the
  active-plan boundary); SOP-internal agent calls +0; task attempts +0; approvals 0;
  plan repairs +1 (UI plan canonicalized).
- **Preserved.** RUST-001–004 remain `LOCAL_DONE`/`PARTIAL`; the migration plan stays
  active with RUST-005A `BLOCKED`. No commit/push/merge. No browser URL — the UI was
  not run.

## 16. UI-001/UI-002 — external development work (not SOP-executed)

The operator explicitly authorized implementing UI-001 and UI-002 **outside SOP**
while the migration plan stays active. The UI plan was **not** launched through SOP
(SOP refused a second active plan — §15).

- **UI-001 — API contracts** (deliverable):
  `docs/reports/rust-migration/web-ui/API-CONTRACTS.md`. Schemas for
  `GET /api/health`, `POST /api/documents/ingest`, `POST /api/query`; states;
  formats (`.md`/`.markdown`/`.txt`/`.pdf`, no OCR); a 10 MiB upload limit;
  proxy/request timeouts; citation fields (`source`/`section`/`page` nullable/`text`);
  intentional differences. Documentation — not SOP-verified.
- **UI-002 — Next.js prototype** (deliverable): `web/` (Next.js 15 App Router,
  React 19, TypeScript). Reuses inventory conventions (`AppShell`/`PageHeader`/
  `PageTitle`/`PageFooter`/`FormStatus`/`SystemStatus`, `@/*` paths, same-origin
  `/api` rewrite with server-side `API_URL`). **Mock mode is visibly labeled** on
  every screen; responses are simulated and do not come from the Rust pipeline.
- **Mock scenarios**: success, empty, insufficient-evidence, connection-failure,
  invalid-file, incomplete-pdf, loading.
- **Checks (operator-run)**: `npm install` ok; `npm run typecheck` (`tsc --noEmit`)
  clean; `npm run build` ok (Next.js 15.5.28). No SOP-authorizable check exists for
  Node.js, so these are operator-run evidence, not SOP-verified.
- **Running**: `npm run dev` → `http://127.0.0.1:3000` (listening on `127.0.0.1`
  only), HTTP 200, title "RAG Web UI (prototype)", "Mock mode" present.
- **Pending / not implemented**: UI-003 (Axum API), UI-004 (real wiring), UI-005
  (verification). The prototype does **not** use the real Rust pipeline; no Cargo
  check is claimed as evidence for the UI/API criteria. Database integration stays
  pending.
- **Counts (this work)**: SOP invocations +0; SOP-internal agent calls +0; task
  attempts +0; approvals 0. Recorded as **external development** work, distinct from
  SOP-managed execution.

## 17. Prototype operations (Makefile) — external development work

Development outside SOP; SOP state and the active migration plan are unchanged.

- **Changed files**: `Makefile` (added `web-start`/`web-stop`/`web-status`; all
  existing targets preserved), `scripts/web-control.sh` (new control script),
  `web/.gitignore` (`/.run`), `web/README.md` (documented the targets).
- **Behaviour**: starts the Next.js prototype in the background on `127.0.0.1:3000`;
  tracks the PID and log under git-ignored `web/.run/`; resolves Node/npm from PATH,
  the Zed-bundled installation, or `NODE_BIN`/`NPM_BIN`; refuses a duplicate start and
  refuses when port 3000 is held by another service (never stops it); stops only this
  project's process group (npm + next-server) and handles a stale PID file safely;
  reports state, URL, and log path.
- **Verification**: `make web-start` → started (pid), URL, log; `GET / → 200`; a
  second `make web-start` → "already running … not starting a duplicate";
  `make web-status` → running + URL + log; `make web-stop` → stopped, port free; a
  second `make web-stop` → "not running (no pid file)"; with a dummy service on port
  3000, `make web-start` → "port 3000 is already in use; refusing to start (another
  service is left untouched)" and the dummy was left running; `make web-start` again
  → restarted. Left running on `http://127.0.0.1:3000`.
- **Counts (this work)**: SOP invocations +0; SOP-internal agent calls +0; task
  attempts +0; approvals 0. External development work.

## 18. Git hooks for Rust + Next.js — external development work

- **Inspected**: `.githooks/pre-commit` (Go: `go mod tidy`/`fmt`/`vet`/`staticcheck`/
  `govulncheck`/`test`/`build` + a `jq` JSON check) and `.githooks/pre-push` (Go
  integration tests). `core.hooksPath` was **unset** (hooks inactive). The Makefile's
  `.PHONY` named `rust-*` targets that did not exist (dangling).
- **Changed files**: `Makefile` (defined the missing
  `rust-fmt|rust-build|rust-test|rust-clippy|rust-check`, added
  `web-typecheck|web-build|web-check` and `check`), `scripts/web-control.sh` (added
  `typecheck`/`build` subcommands using the same Node/npm resolution),
  `.githooks/pre-commit` (added `make rust-fmt` + `make web-typecheck`),
  `.githooks/pre-push` (added `make rust-build rust-test rust-clippy` + `make
  web-build`), `docs/guides/agent-scripts.md` (installation + the new checks).
- **Existing checks preserved**: every Go check remains in both hooks; none removed.
- **Installed / active**: `git config core.hooksPath .githooks` → **`.githooks`**.
- **Verification**:
  - `make rust-fmt` and `make web-typecheck` clean.
  - `pre-push` in the clean state → **exit 0** (Rust build/test/clippy + Next.js
    production build).
  - **Blocking**: with a deliberately bad Rust line, `make rust-fmt` → exit 2 and
    `.githooks/pre-push` → exit 2 (clippy `-D warnings`). A `git commit --allow-empty`
    was **aborted with no commit created** (HEAD unchanged at `d72fb39`).
  - A **pre-existing** Go `staticcheck` failure (`internal/rag/reliability_test.go`
    ST1012) currently blocks pre-commit before the Rust step; noted, not fixed
    (out of scope).
  - Side effect observed and reverted: the hook's `go fmt ./...` reformatted
    `cmd/retrieval/main.go`; restored to HEAD.
- **Counts (this work)**: SOP invocations +0; SOP-internal agent calls +0; task
  attempts +0; approvals 0. External development work.

## 19. Shared Node/npm toolchain check — external development work

- **Inspected first**: `Makefile`, `web/package.json`, the lockfile
  (`web/package-lock.json`), and `scripts/web-control.sh`.
- **Declared** in `web/package.json`: `engines` (`node >=20.0.0 <25.0.0`,
  `npm >=10.0.0 <12.0.0`), `packageManager: npm@11.19.0`, and `testedWith`
  (`node 24.21.0`, `npm 11.19.0`).
- **Single discovery point**: `scripts/web-control.sh` resolves Node/npm as
  explicit overrides -> PATH -> Zed's bundled Node (glob over any `node-*/bin`,
  Linux and macOS locations; no user name or single Zed version hard-coded), then
  validates both majors against `engines` (a pure-shell parse of the `engines`
  block, so an old node is still judged correctly) and exports the resolved Node
  dir on `PATH` so npm and its children share the toolchain. Missing or incompatible
  tools stop with setup instructions; nothing is installed or upgraded.
- **Reuse**: `check-env` gates `install`, `typecheck`, `build`, and `start`;
  `make web-check-env` / `make web-install` added; both Git hooks call
  `make web-check-env` (pre-commit before typecheck, pre-push before the build).
- **Verification**: default -> **Zed fallback** (`node-v24.21.0`); **PATH discovery**
  (`/tmp/pathbin/node`); **explicit overrides** honoured; **missing tools** -> setup
  hint + non-zero; **incompatible** (node v18 / npm 9) -> "outside the supported
  range" + non-zero; `make web-typecheck` and `make web-build` both exit 0 with the
  resolved toolchain.
- **Counts (this work)**: SOP invocations +0; SOP-internal agent calls +0; task
  attempts +0; approvals 0. External development work.

## 20. Hook + toolchain finish — external development work

- **ST1012 fixed (no behaviour change)**: renamed the sentinel in
  `internal/rag/reliability_test.go`, `failingGeneratorErr` ->
  `errFailingGenerator` (staticcheck ST1012 wants `errFoo`).
  `go test -count=1 ./internal/rag/` passes; `staticcheck ./internal/rag/` clean.
- **Read-only formatting checks**: `pre-commit` no longer runs `go mod tidy` /
  `go fmt ./...` (which rewrote files). It now runs `go mod tidy -diff` and a
  `gofmt -l .` check that reports unformatted files and exits non-zero. Hooks no
  longer modify source files. The pre-existing gofmt drift in
  `cmd/retrieval/main.go` was fixed (behaviour-neutral) so the read-only check
  passes.
- **Toolchain / vulnerabilities**: the flagged stdlib vulns (GO-2026-6617,
  GO-2026-6613) are fixed only in go1.27.2. A global `toolchain go1.27.2` pin broke
  the prebuilt `staticcheck` (older export-data reader), so the pin is scoped to the
  govulncheck step (`GOTOOLCHAIN=go1.27.2 govulncheck ./...`); the rest of the hook
  keeps the default toolchain. `govulncheck ./...` -> "No vulnerabilities found". No
  tools were installed or upgraded.
- **Build robustness**: `web-build` now clears its isolated dist dir first, so a
  stale `.next-build` cannot fail the build; `make web-build` leaves the running UI
  healthy (`GET / -> 200`).
- **Verification**:
  - Both hooks pass on clean source: `pre-commit` exit 0, `pre-push` exit 0.
  - Controlled failures block each hook: with a bad Rust line, `pre-commit` exit 2
    (rust-fmt) and `pre-push` exit 2 (clippy `-D warnings`).
  - Disposable checkout (`rsync` copy) with a misformatted Go file: `git commit
    --allow-empty` was **blocked** by the read-only gofmt check and **no commit was
    created** (HEAD unchanged); the file was left unmodified; the checkout was
    removed and the real repo untouched.
- **Counts (this work)**: SOP invocations +0; SOP-internal agent calls +0; task
  attempts +0; approvals 0. External development work.
