# RUST-007D — live-model closure requirement

**Status: UNMET.**

Criterion: generated answers meet the baseline groundedness and citation
criteria. This is a live-model closure requirement and cannot be satisfied by
scripted tests.

Reason it is unmet: no run against a real Ollama serving `nomic-embed-text` and
`llama3.2:3b` has been recorded. Whether such a server is available in this
environment is the informational UNKNOWN entry in the capability inventory of
`RUST-007D-bilingual-contract.md`. No metric values are recorded here, because
none have been observed.

## What does not close it

* `rust/acceptance/bilingual_parity` uses a deterministic Ollama stub for both
  embeddings and generation. It proves Go/Rust behavior parity, not model
  quality.
* The query-cache, prompt, and pinned-request unit tests in
  `rust/src/bilingual.rs` use a scripted generator.

## Required run

1. A real Ollama at `OLLAMA_URL` serving `nomic-embed-text` (768 dimensions)
   and `llama3.2:3b`.
2. A database ingested (through `cmd/ingest` or the Rust `ingest`) with the
   eight sources referenced by `evals/retrieval-vi-en-expanded.json`, in one
   corpus.
3. Run:

   ```sh
   ARM=all REPEATS=1 DIAG=1 \
   EVAL_DATASET=evals/retrieval-vi-en-expanded.json \
   OLLAMA_EMBED_MODEL=nomic-embed-text OLLAMA_CHAT_MODEL=llama3.2:3b \
   QUERY_CACHE=<path to a real query cache, e.g. docs/rq-006-query-cache.json> \
   RAG_CORPUS=<corpus> \
   cargo run --manifest-path rust/Cargo.toml --bin expbilingual
   ```

4. Answer generation for groundedness and citations, over the same corpus and
   models (for example the Rust `query` binary with `RAG_CITATION_VALIDATION`
   enabled), for the dataset questions.

## Artifacts to capture

* The `HEADER` line, every `GEN`, `DIAG` and `CASE` line, and the `FOOTER`.
* The written query cache file.
* For each generated answer: the answer text, its citations, and the
  groundedness and citation-validity observations against the baseline
  criteria.
* Model versions (`ollama list`), the commit, and the date of the run.

Until those artifacts are recorded, this criterion remains UNMET.
