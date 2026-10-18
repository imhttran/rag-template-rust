# RUST-007D — live-model closure requirement

**Status: EVIDENCE RECORDED — operator acceptance decision pending.**

Criterion (`criterion-4`): generated answers meet the baseline groundedness and
citation criteria. This is a live-model closure requirement and cannot be
satisfied by scripted tests.

This note records the actual live run against a real Ollama serving
`nomic-embed-text` and `llama3.2:3b`, the judge results for the Rust answers, and
the Go reference answers over the same corpus, questions, model, and judge. The
SOP gate for RUST-007D remains untouched by this note.

## 1. Run identity

| Item | Value |
| ---- | ----- |
| Date (UTC offset local) | 2026-10-18 |
| Repository branch / HEAD | `rust-007d-resume` / `85d771c` plus the uncommitted citation/prompt port and this evidence |
| Ollama version | `0.40.4` (`/api/version`) |
| Embedding model | `nomic-embed-text:latest` (768 dimensions) |
| Chat model | `llama3.2:3b` |
| Corpus | `rust-007d-live-20261018` (8 sources, 83 chunks), ingested with the Go `cmd/ingest` reference |
| Dataset | `evals/retrieval-vi-en-expanded.json` (39 cases: 33 answerable, 6 unanswerable) |
| Judge | the same `llama3.2:3b` with the Go `groundednessPrompt`, `temperature 0`, `seed 0` |

The citation/prompt port under test is the working-tree change to
`rust/src/answer/{citation,prompt,render,service}.rs`: Go's
`[source - section]` / `[source - section - p.N]` contract, source/section/page
provenance in the context block, and the matching citation repair/validation.

## 2. Configuration

```text
RAG_CORPUS=rust-007d-live-20261018
OLLAMA_URL=http://127.0.0.1:11434
OLLAMA_EMBED_MODEL=nomic-embed-text
OLLAMA_CHAT_MODEL=llama3.2:3b
EMBED_DIM=768
MIN_SIMILARITY=0.6
TOP_K=4
FINAL_K=3
EXPAND_LIMIT=20
CONTEXT_BUDGET=4000
QUERY_REWRITE=false
RAG_CITATION_VALIDATION=true
REQUEST_TIMEOUT=5m
```

## 3. Commands

```sh
# 1. Ingest the eight dataset sources into one live corpus (Go reference).
./cmd/ingest <each source>            # RAG_CORPUS=rust-007d-live-20261018

# 2. Rust production query over every dataset question.
cargo build --locked --manifest-path rust/Cargo.toml
rust/target/debug/query "<question>"  # env above; 39 invocations

# 3. Go reference query over the same questions and corpus.
go build -o /tmp/rag-go ./cmd/rag
/tmp/rag-go "<question>"              # env above + RAG_ANSWERABILITY_GATE=false

# 4. Groundedness judge and citation validity over the captured evidence.
python3 /tmp/judge-007d-rust2.py
python3 /tmp/judge-007d-go.py
```

The exact query context each Rust answer was generated from was captured by an
out-of-tree diagnostic crate that calls the same shared retrieval modules
(`retrieval_pg::fuse`, `retrieval::dedup`, `retrieval::expand`,
`answer::context_budget`); it is not part of the repository.

## 4. Results

### 4.1 Rust production `query` (live model)

| Measure | Result |
| ------- | ------ |
| Questions run | 39 / 39 exited 0 |
| Substantive answers | 13 |
| Refusals | 26 — 6 correct for unanswerable cases, 18 after a retrieval miss, 2 despite the expected section being retrieved |
| Groundedness (judge, substantive answers) | **12 / 13 GROUNDED** (0.92); 1 `NOT_GROUNDED` (`eval-en-en-02`) |
| Rendered citations | **6 / 6 valid** (100%); no invalid citation was rendered |
| Substantive answers with at least one citation | **5 / 13** |
| Empty-citation defect (`(as per )`, `(page )`) | not observed after the port |

### 4.2 Go reference `cmd/rag` (same corpus, questions, models, judge)

| Measure | Result |
| ------- | ------ |
| Questions run | 39 / 39 |
| Substantive answers | 18 |
| Refusals | 21 |
| Groundedness (judge, substantive answers) | **12 / 18 GROUNDED** (0.67); 6 `NOT_GROUNDED` |
| Rendered citations | **11 / 11 valid** (100%) |
| Substantive answers with at least one citation | **10 / 18** |

The Go reference used its single bounded citation repair prompt; the Rust
`AnswerService` strips invalid citations without the extra re-prompt. That
difference is visible in citation coverage, not in citation validity.

## 5. Observed failures (preserved)

* `eval-en-en-02` (Rust): the answer states a late fee may be assessed at the
  servicer's discretion, while the retrieved evidence says a late fee may be
  **waived** at the servicer's discretion. Judged `NOT_GROUNDED`.
* `eval-en-en-02`, `eval-en-en-06`, `eval-vi-vi-01`, `eval-vi-vi-04`,
  `eval-vi-vi-07`, `eval-en-vi-02`, `eval-en-vi-03`, `eval-en-vi-04` (Rust):
  factual answers with no citation. The model omitted the citation the prompt
  requires.
* `eval-vi-vi-10` and `eval-misleading-02` (Rust): the model refused although
  the retrieved context contained the expected section.
* Go reference failures are recorded in `live-007d/go-baseline.json` (6
  `NOT_GROUNDED`, including 1 cited answer).

## 6. Artifacts

All under `docs/reports/rust-migration/live-007d/`:

| File | Contents |
| ---- | -------- |
| `ingest.log` | Go ingestion of the eight sources (83 chunks) |
| `expbilingual.log` | live bilingual harness run (117 `CASE`, 234 `GEN`, footer) |
| `query-cache.json` | query cache written by the live bilingual run |
| `answers.jsonl` | 39 live Rust `query` answers, exit codes, stdout/stderr |
| `answers-progress.log`, `answers-run.log` | progress and completion of the answer loop |
| `contexts.json` | exact budgeted context each Rust answer was generated from |
| `groundedness.json` | per-case refusal/citation/judge result for the Rust answers |
| `go-baseline.json` | per-case Go reference answers, citation counts, judge results |
| `rust-eval-ranking.log` | Rust retrieval ranking over the same dataset |

`query-cache.json` may be regenerated and is not required for the evidence;
the answer/context/judge artifacts are the load-bearing set.

## 7. Assessment

On the baseline's own measured criteria (grounded answers among judged answers,
and rendered-citation validity), the Rust answers are at least
baseline-comparable: groundedness 12/13 versus the Go reference's 12/18, and
citation validity 6/6 versus 11/11. The original empty-citation defect is gone.

Two shortfalls remain visible rather than hidden:

1. one Rust answer is not grounded in its retrieved context; and
2. the Rust answer path has lower citation coverage than the Go path because it
   does not perform Go's single bounded citation re-prompt.

Whether citation coverage is part of "the baseline groundedness and citation
criteria" is an operator decision. This note does not claim `VERIFIED`; the
RUST-007D gate is still pending and must be resolved through the acceptance
mechanism in §8.

## 8. Supported acceptance action (not run)

The recorded outcome maps `criterion-4` to manual evidence at attempt 1.
`sop outcome --accept` records a designated approver's explicit decision and
never rewrites the outcome artifact. This project currently has **no**
designated approvers (`human.designated_approvers` is commented out), so the
command fails closed until the operator adds one.

Exact actions for the operator's decision:

```yaml
# .agent-sdlc/config.yaml
human:
  designated_approvers:
    - operator
```

```sh
sop outcome RUST-007D --accept criterion-4 \
  --evidence "docs/reports/rust-migration/RUST-007D-live-model-evidence.md: live llama3.2:3b run; Rust grounded 12/13, citations 6/6 valid, 5/13 answers cited; Go reference grounded 12/18, citations 11/11 valid; artifacts in docs/reports/rust-migration/live-007d/" \
  --by operator

sop approve RUST-007D --by operator
```

The acceptance decision is bound to the current criterion text, manual-evidence
description, and attempt. A criterion-scoped waiver
(`sop outcome RUST-007D --waive criterion-4 ...`) is a separate, deliberately
different action and is not recommended here.
