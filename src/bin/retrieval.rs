//! Retrieval-only producer for the Agentic SOP `rag-retrieval/v1` contract.
//!
//! Port of the removed Go `cmd/retrieval`. It reads one bounded JSON request
//! from stdin and writes one JSON response to stdout. It never generates an
//! answer. stdout carries only the response; diagnostics go to stderr. Every
//! outcome, including a failure, is a well-formed response and exit status 0.
//!
//! Corpus isolation: the requested corpus MUST be in the trusted allowlist
//! (`RAG_RETRIEVAL_ALLOWED_CORPORA`, comma-separated). An unknown or missing
//! corpus is rejected with `UNAUTHORIZED_SCOPE` before any database or Ollama
//! work, and every search is corpus-scoped in SQL before ranking and `LIMIT`.
//!
//! Retrieval reuses the shared stages: [`PgStore`] vector + keyword search
//! (with the request's metadata filters as a SQL pre-filter), RRF fusion at
//! `top_k`, then one result per `(source, section)`. Section expansion is
//! skipped: the Go producer only fell back to it when fusion was empty, which
//! leaves nothing to expand.

use std::collections::HashSet;
use std::env;
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

use r2d2_postgres::postgres::{Config as PgConfig, NoTls};
use r2d2_postgres::PostgresConnectionManager;
use serde::{Deserialize, Serialize};

use rag_foundation::config::Config;
use rag_foundation::embedding::{Embedder, OllamaEmbedder};
use rag_foundation::ollama::{HttpTransport, OllamaClient};
use rag_foundation::retrieval_pg;
use rag_foundation::storage_pg::{PgStore, SearchFilter};

const SCHEMA_VERSION: &str = "rag-retrieval/v1";
const MAX_REQUEST_BYTES: usize = 1 << 20;
const ALLOWED_ENV: &str = "RAG_RETRIEVAL_ALLOWED_CORPORA";
const DEFAULT_TOP_K: usize = 5;
const MAX_TOP_K: i64 = 50;
const DEFAULT_DEADLINE: Duration = Duration::from_secs(30);

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Request {
    schema_version: String,
    query: String,
    top_k: i64,
    filters: Filters,
    deadline_ms: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Filters {
    corpus: String,
    source_prefix: String,
    language: String,
    ingested_from: String,
    ingested_to: String,
}

#[derive(Debug, Serialize)]
struct Evidence {
    id: String,
    corpus: String,
    source: String,
    document_hash: String,
    /// `sha256:<hex>` of the exact UTF-8 bytes of `text`. SOP recomputes it and
    /// rejects a mismatch (PROPOSAL-RAG-EVIDENCE-HASH.md, option A).
    chunk_hash: String,
    section: String,
    page: i32,
    text: String,
    retrieval_method: &'static str,
    score: f64,
    rank: usize,
}

#[derive(Debug, Serialize)]
struct Diagnostics {
    elapsed_ms: u128,
    #[serde(skip_serializing_if = "str::is_empty")]
    error_code: &'static str,
}

#[derive(Debug, Serialize)]
struct Response {
    schema_version: &'static str,
    status: &'static str,
    evidence: Vec<Evidence>,
    truncated: bool,
    diagnostics: Diagnostics,
}

impl Response {
    /// A failure response: `status` and `error_code` both carry `code`.
    fn error(code: &'static str, elapsed_ms: u128) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            status: code,
            evidence: Vec::new(),
            truncated: false,
            diagnostics: Diagnostics {
                elapsed_ms,
                error_code: code,
            },
        }
    }
}

fn main() {
    let response = run(io::stdin().lock());
    let mut stdout = io::stdout().lock();
    if let Err(err) = serde_json::to_writer(&mut stdout, &response)
        .map_err(io::Error::from)
        .and_then(|()| writeln!(stdout))
    {
        eprintln!("retrieval: encode response: {err}");
    }
}

fn run(stdin: impl Read) -> Response {
    let start = Instant::now();
    let mut raw = Vec::new();
    if let Err(err) = stdin
        .take(MAX_REQUEST_BYTES as u64 + 1)
        .read_to_end(&mut raw)
    {
        eprintln!("retrieval: read request: {err}");
        return Response::error("MALFORMED", 0);
    }
    if raw.len() > MAX_REQUEST_BYTES {
        return Response::error("OVERSIZE", 0);
    }
    let request: Request = match serde_json::from_slice(&raw) {
        Ok(request) => request,
        Err(err) => {
            eprintln!("retrieval: decode request: {err}");
            return Response::error("MALFORMED", 0);
        }
    };
    if request.schema_version != SCHEMA_VERSION {
        return Response::error("UNSUPPORTED_SCHEMA", 0);
    }
    let corpus = request.filters.corpus.trim();
    if corpus.is_empty() || !allowed(corpus, &env::var(ALLOWED_ENV).unwrap_or_default()) {
        return Response::error("UNAUTHORIZED_SCOPE", 0);
    }
    let filter = match parse_filters(&request.filters) {
        Ok(filter) => filter,
        Err(err) => {
            eprintln!("retrieval: {err}");
            return Response::error("MALFORMED", 0);
        }
    };

    let top_k = if request.top_k <= 0 || request.top_k > MAX_TOP_K {
        DEFAULT_TOP_K
    } else {
        request.top_k as usize
    };
    let deadline = if request.deadline_ms > 0 {
        Duration::from_millis(request.deadline_ms as u64)
    } else {
        DEFAULT_DEADLINE
    };

    match retrieve(&request.query, corpus, top_k, &filter, start, deadline) {
        Ok(evidence) => Response {
            schema_version: SCHEMA_VERSION,
            status: if evidence.is_empty() { "EMPTY" } else { "OK" },
            evidence,
            truncated: false,
            diagnostics: Diagnostics {
                elapsed_ms: start.elapsed().as_millis(),
                error_code: "",
            },
        },
        Err((code, message)) => {
            eprintln!("retrieval: {message}");
            Response::error(code, start.elapsed().as_millis())
        }
    }
}

type Failure = (&'static str, String);

/// Embeds the query and runs the hybrid search, bounded by `deadline` from
/// `start`. Connect and embed failures are `UNAVAILABLE`; a search failure is
/// `TIMEOUT` once the deadline has passed, `INTERNAL` otherwise.
fn retrieve(
    query: &str,
    corpus: &str,
    top_k: usize,
    filter: &SearchFilter,
    start: Instant,
    deadline: Duration,
) -> Result<Vec<Evidence>, Failure> {
    // Zero durations are rejected by r2d2 and reqwest; keep at least 1 ms.
    let remaining = || {
        deadline
            .saturating_sub(start.elapsed())
            .max(Duration::from_millis(1))
    };
    let config = Config::load().map_err(|err| ("INTERNAL", format!("configuration: {err}")))?;

    let transport = Arc::new(HttpTransport::new(remaining()));
    let client = OllamaClient::new(config.ollama_url.clone(), transport, remaining());
    let embedder = OllamaEmbedder::new(client, config.embed_model.clone())
        .with_embed_dim(config.embed_dim as usize);

    let store = connect(&config.database_url, remaining()).map_err(|err| {
        (
            "UNAVAILABLE",
            format!("connect to the document store: {err}"),
        )
    })?;

    let vector = embedder
        .embed(query)
        .map_err(|err| ("UNAVAILABLE", format!("embed query: {err}")))?;

    let search_failed = |err: &dyn std::fmt::Display| {
        let code = if start.elapsed() >= deadline {
            "TIMEOUT"
        } else {
            "INTERNAL"
        };
        (code, format!("search: {err}"))
    };
    let language = filter.language.as_deref().unwrap_or("");
    // Go `KeepSimilar` with the producer's floor of 0.
    let vector_hits = store
        .vector_search_filtered(corpus, &vector, top_k * 4, filter)
        .map_err(|err| search_failed(&err))?
        .into_iter()
        .filter(|hit| hit.similarity >= 0.0)
        .collect();
    let keyword_hits = store
        .keyword_search_filtered(corpus, query, top_k * 4, language, filter)
        .map_err(|err| search_failed(&err))?;

    let mut seen = HashSet::new();
    let evidence = retrieval_pg::fuse_hits(&[vector_hits, keyword_hits], top_k)
        .into_iter()
        .filter(|(hit, _)| {
            seen.insert((hit.row.chunk.source.clone(), hit.row.chunk.section.clone()))
        })
        .enumerate()
        .map(|(index, (hit, score))| Evidence {
            id: hit.row.id.to_string(),
            corpus: corpus.to_string(),
            source: hit.row.chunk.source.unwrap_or_default(),
            document_hash: hit.row.chunk.content_hash,
            chunk_hash: chunk_hash(&hit.row.chunk.content),
            section: hit.row.chunk.section.unwrap_or_default(),
            page: hit.row.chunk.page.unwrap_or(0),
            text: hit.row.chunk.content.clone(),
            retrieval_method: "hybrid",
            score,
            rank: index + 1,
        })
        .collect();
    Ok(evidence)
}

/// `sha256:<hex>` over the exact bytes of `text`, the value SOP recomputes.
fn chunk_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(text.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("sha256:{hex}")
}

/// Opens the store with every database wait bounded by `timeout`. One direct
/// connection is made first because the r2d2 pool retries an unreachable
/// database until its timeout instead of failing at once.
fn connect(dsn: &str, timeout: Duration) -> Result<PgStore, String> {
    let mut pg: PgConfig = dsn.parse().map_err(|err| format!("invalid DSN: {err}"))?;
    let options = format!(
        "{} -c statement_timeout={}",
        pg.get_options().unwrap_or_default(),
        timeout.as_millis()
    );
    pg.connect_timeout(timeout).options(options.trim());
    pg.connect(NoTls).map_err(|err| err.to_string())?;
    let pool = r2d2::Pool::builder()
        .max_size(1)
        .connection_timeout(timeout)
        .build_unchecked(PostgresConnectionManager::new(pg, NoTls));
    Ok(PgStore::from_pool(pool))
}

/// Reports whether `corpus` is one of the comma-separated `allowlist` entries.
fn allowed(corpus: &str, allowlist: &str) -> bool {
    allowlist.split(',').any(|entry| entry.trim() == corpus)
}

/// Maps the request filters onto the search filter. A non-empty timestamp that
/// is not RFC 3339 is an error: silently dropping it would widen the result set
/// instead of failing the malformed request.
fn parse_filters(raw: &Filters) -> Result<SearchFilter, String> {
    let timestamp = |name: &str, value: &str| -> Result<Option<String>, String> {
        if value.is_empty() {
            return Ok(None);
        }
        let value = value.trim();
        if is_rfc3339(value) {
            Ok(Some(value.to_string()))
        } else {
            Err(format!("{name}: {value:?} is not an RFC 3339 timestamp"))
        }
    };
    let non_empty = |value: &str| (!value.is_empty()).then(|| value.to_string());
    Ok(SearchFilter {
        source_prefix: non_empty(&raw.source_prefix),
        language: non_empty(raw.language.trim()),
        ingested_from: timestamp("ingested_from", &raw.ingested_from)?,
        ingested_to: timestamp("ingested_to", &raw.ingested_to)?,
    })
}

/// Strict RFC 3339 check matching Go's `time.Parse(time.RFC3339, ...)`:
/// `YYYY-MM-DDTHH:MM:SS[.frac](Z|±HH:MM)` with real calendar ranges.
fn is_rfc3339(value: &str) -> bool {
    let b = value.as_bytes();
    let num = |range: std::ops::Range<usize>| -> Option<u32> {
        let digits = b.get(range)?;
        digits
            .iter()
            .all(u8::is_ascii_digit)
            .then(|| digits.iter().fold(0, |n, d| n * 10 + u32::from(d - b'0')))
    };
    let shape = b.len() >= 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':';
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        num(0..4),
        num(5..7),
        num(8..10),
        num(11..13),
        num(14..16),
        num(17..19),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    if !shape || day == 0 || day > days || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let mut rest = &b[19..];
    if rest.first() == Some(&b'.') {
        let digits = rest[1..].iter().take_while(|d| d.is_ascii_digit()).count();
        if digits == 0 {
            return false;
        }
        rest = &rest[1 + digits..];
    }
    match rest {
        b"Z" => true,
        [sign, h1, h2, b':', m1, m2]
            if (*sign == b'+' || *sign == b'-')
                && [h1, h2, m1, m2].iter().all(|d| d.is_ascii_digit()) =>
        {
            (h1 - b'0') * 10 + (h2 - b'0') <= 23 && (m1 - b'0') * 10 + (m2 - b'0') <= 59
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    /// Shared SHA-256 vector, also pinned in agentic-sop
    /// (internal/ragclient/chunk_hash_test.go). Expected value computed with
    /// `printf 'chunk hash vector: café 日本語\n' | shasum -a 256`.
    #[test]
    fn chunk_hash_matches_shared_vector() {
        assert_eq!(
            chunk_hash("chunk hash vector: café 日本語\n"),
            "sha256:74f755664e399d5fec1e9e418c2d9dc684fe5abcab30c6fe2f9f60767bb620a8"
        );
    }

    use super::*;

    fn filters(from: &str, to: &str) -> Filters {
        Filters {
            ingested_from: from.to_string(),
            ingested_to: to.to_string(),
            ..Filters::default()
        }
    }

    // Go TestParseFiltersRejectsMalformedTimestamps: an unparsable filter must
    // not be silently dropped, because dropping it would widen the result set.
    #[test]
    fn parse_filters_rejects_malformed_timestamps() {
        for raw in [
            filters("not-a-time", ""),
            filters("", "2026-13-40T00:00:00Z"),
            filters("   ", ""),
            filters("2026-02-29T00:00:00Z", ""),
            filters("2026-10-01T00:00:00", ""),
            filters("2026-10-01 00:00:00Z", ""),
        ] {
            assert!(parse_filters(&raw).is_err(), "accepted {raw:?}");
        }
    }

    // Go TestParseFiltersParsesValidTimestampAndMetadata.
    #[test]
    fn parse_filters_parses_valid_timestamps_and_metadata() {
        let got = parse_filters(&Filters {
            corpus: " kb ".to_string(),
            source_prefix: "docs/".to_string(),
            language: " de ".to_string(),
            ingested_from: "2026-10-01T00:00:00Z".to_string(),
            ingested_to: " 2026-10-18T12:30:00.5+07:00 ".to_string(),
        })
        .expect("valid filters");
        assert_eq!(got.source_prefix.as_deref(), Some("docs/"));
        assert_eq!(got.language.as_deref(), Some("de"));
        assert_eq!(got.ingested_from.as_deref(), Some("2026-10-01T00:00:00Z"));
        assert_eq!(
            got.ingested_to.as_deref(),
            Some("2026-10-18T12:30:00.5+07:00")
        );

        let empty = parse_filters(&Filters {
            corpus: "kb".to_string(),
            ..Filters::default()
        })
        .expect("empty filters");
        assert_eq!(empty, SearchFilter::default());
    }

    #[test]
    fn allowlist_matches_trimmed_entries_exactly() {
        assert!(allowed("kb", "other, kb ,x"));
        assert!(!allowed("kb", "kb2,k"));
        assert!(!allowed("kb", ""));
    }

    // These requests are answered before configuration, Ollama, or the database
    // is touched, so they run without either service.
    #[test]
    fn rejects_requests_before_any_dependency() {
        let cases: [(&[u8], &str); 5] = [
            (b"not json", "MALFORMED"),
            (
                br#"{"schema_version":"rag-retrieval/v1","query":"q","filters":{"corpus":"kb"},"extra":1}"#,
                "MALFORMED",
            ),
            (
                br#"{"schema_version":"rag-retrieval/v0","query":"q","filters":{"corpus":"kb"}}"#,
                "UNSUPPORTED_SCHEMA",
            ),
            (
                br#"{"schema_version":"rag-retrieval/v1","query":"q","filters":{}}"#,
                "UNAUTHORIZED_SCOPE",
            ),
            (
                br#"{"schema_version":"rag-retrieval/v1","query":"q","filters":{"corpus":"definitely-not-allowed-corpus"}}"#,
                "UNAUTHORIZED_SCOPE",
            ),
        ];
        for (input, want) in cases {
            let response = run(input);
            assert_eq!(
                response.status,
                want,
                "input {:?}",
                String::from_utf8_lossy(input)
            );
            assert_eq!(response.diagnostics.error_code, want);
            assert!(response.evidence.is_empty());
        }
        let oversize = vec![b' '; MAX_REQUEST_BYTES + 1];
        assert_eq!(run(oversize.as_slice()).status, "OVERSIZE");
    }
}
