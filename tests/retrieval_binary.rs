//! `rag-retrieval/v1` producer contract test for the real `retrieval` binary.
//!
//! Rows are written by the real `ingest` binary, so this proves the Rust
//! `ingest` + `retrieval` pair agree on tables, corpus tagging, and source
//! paths. Ollama is a scripted local HTTP server; PostgreSQL is the dedicated
//! database from `DATABASE_URL`. The database-backed cases fail clearly, never
//! skip, when `DATABASE_URL` is unavailable.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

const DEAD_DSN: &str = "postgres://rag:rag@127.0.0.1:1/rag?sslmode=disable";
const DEAD_OLLAMA: &str = "http://127.0.0.1:1";

fn dsn() -> String {
    match std::env::var("DATABASE_URL") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => panic!(
            "DATABASE_URL is not set: the retrieval binary contract test requires the \
             dedicated database. Load the repository .env; this test fails, it does not skip."
        ),
    }
}

fn unique_tag() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    format!("{}-{nanos}", std::process::id())
}

/// Serves `/api/embed` with one fixed 768-dimension vector, forever, on a
/// detached thread. Every chunk and query embeds to the same vector.
fn fake_ollama() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind the fake Ollama server");
    let url = format!(
        "http://{}",
        listener.local_addr().expect("fake Ollama address")
    );
    std::thread::spawn(move || {
        let body = format!("{{\"embeddings\":[[{}]]}}", vec!["0.1"; 768].join(","));
        for mut stream in listener.incoming().flatten() {
            if read_request(&mut stream).is_some() {
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        }
    });
    url
}

/// Reads one request's headers and body so the client sees a clean response.
fn read_request(stream: &mut TcpStream) -> Option<()> {
    let mut data = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let n = stream.read(&mut chunk).ok().filter(|&n| n > 0)?;
        data.extend_from_slice(&chunk[..n]);
        let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&data[..end]).to_ascii_lowercase();
        let length: usize = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(0);
        while data.len() < end + 4 + length {
            let n = stream.read(&mut chunk).ok().filter(|&n| n > 0)?;
            data.extend_from_slice(&chunk[..n]);
        }
        return Some(());
    }
}

/// Deletes only the test-owned source when dropped (including on panic).
struct Cleanup(String, String);

impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Ok(mut client) = postgres::Client::connect(&self.0, postgres::NoTls) {
            let _ = client.execute("DELETE FROM documents WHERE source = $1", &[&self.1]);
        }
    }
}

/// Ingests a small Markdown document with the real `ingest` binary.
fn ingest(dsn: &str, ollama: &str, corpus: &str, source: &str) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("policy.md");
    std::fs::write(
        &path,
        "# Policy\n\n## Retrieval fallback\n\nWhen vector search finds nothing, the retrieval \
         fallback uses keyword search over the same corpus.\n",
    )
    .expect("write the test document");
    let output = Command::new(env!("CARGO_BIN_EXE_ingest"))
        .arg(&path)
        .env("DATABASE_URL", dsn)
        .env("OLLAMA_URL", ollama)
        .env("RAG_CORPUS", corpus)
        .env("RAG_SOURCE", source)
        .output()
        .expect("spawn the ingest binary");
    assert!(
        output.status.success(),
        "ingest failed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Runs the binary with `request` on stdin and returns the decoded response.
/// Asserts the process exits 0 and stdout is exactly one JSON line whose
/// fields are the ones the SOP client decodes with unknown fields disallowed.
fn retrieve(dsn: &str, ollama: &str, allowed: &str, request: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_retrieval"))
        .env("DATABASE_URL", dsn)
        .env("OLLAMA_URL", ollama)
        .env("RAG_RETRIEVAL_ALLOWED_CORPORA", allowed)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the retrieval binary");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(format!("{request}\n").as_bytes())
        .expect("write the request");
    let output = child
        .wait_with_output()
        .expect("wait for the retrieval binary");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "non-zero exit; stderr:\n{stderr}");
    assert!(
        stdout.ends_with('\n') && stdout.trim_end().lines().count() == 1,
        "stdout must be exactly one JSON line; got:\n{stdout}\nstderr:\n{stderr}"
    );
    let response: Value = serde_json::from_str(&stdout).expect("stdout is one JSON value");

    let keys = |value: &Value| -> BTreeSet<String> {
        value.as_object().expect("object").keys().cloned().collect()
    };
    let expect = |names: &[&str]| -> BTreeSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    };
    assert_eq!(
        keys(&response),
        expect(&[
            "schema_version",
            "status",
            "evidence",
            "truncated",
            "diagnostics"
        ])
    );
    assert_eq!(response["schema_version"], "rag-retrieval/v1");
    for evidence in response["evidence"].as_array().expect("evidence array") {
        assert_eq!(
            keys(evidence),
            expect(&[
                "id",
                "corpus",
                "source",
                "document_hash",
                "section",
                "page",
                "text",
                "retrieval_method",
                "score",
                "rank",
            ])
        );
    }
    response
}

fn request(corpus: &str, filters: Value) -> Value {
    let mut filters = filters;
    filters["corpus"] = json!(corpus);
    json!({
        "schema_version": "rag-retrieval/v1",
        "query": "retrieval fallback",
        "top_k": 5,
        "filters": filters,
        "deadline_ms": 10000,
    })
}

fn assert_failure(response: &Value, code: &str) {
    assert_eq!(response["status"], code, "response: {response}");
    assert_eq!(response["diagnostics"]["error_code"], code);
    assert_eq!(response["evidence"], json!([]));
}

#[test]
fn allowed_corpus_returns_ranked_cited_evidence_from_rust_ingest() {
    let dsn = dsn();
    let tag = unique_tag();
    let corpus = format!("retrieval-binary-{tag}");
    let source = format!("docs/sop/retrieval-{tag}.md");
    let _cleanup = Cleanup(dsn.clone(), source.clone());
    let ollama = fake_ollama();
    ingest(&dsn, &ollama, &corpus, &source);
    let allowed = format!("other, {corpus}");

    let response = retrieve(&dsn, &ollama, &allowed, request(&corpus, json!({})));
    assert_eq!(response["status"], "OK", "response: {response}");
    assert!(response["diagnostics"].get("error_code").is_none());
    let evidence = response["evidence"].as_array().expect("evidence");
    assert!(!evidence.is_empty(), "no evidence: {response}");
    for (index, item) in evidence.iter().enumerate() {
        assert_eq!(item["corpus"], corpus.as_str());
        assert_eq!(item["source"], source.as_str());
        assert_eq!(item["section"], "Retrieval fallback");
        assert_eq!(item["retrieval_method"], "hybrid");
        assert_eq!(item["rank"], index + 1);
        assert!(!item["document_hash"].as_str().unwrap().is_empty());
        assert!(item["text"].as_str().unwrap().contains("keyword search"));
        assert!(item["score"].as_f64().unwrap() > 0.0);
    }

    // The metadata filters narrow in SQL: a matching prefix keeps the rows, a
    // non-matching prefix or an ingest window in the past leaves none.
    let kept = retrieve(
        &dsn,
        &ollama,
        &allowed,
        request(&corpus, json!({"source_prefix": "docs/sop/"})),
    );
    assert_eq!(kept["status"], "OK", "response: {kept}");
    for filters in [
        json!({"source_prefix": "docs/other/"}),
        json!({"ingested_to": "2000-01-01T00:00:00Z"}),
    ] {
        let narrowed = retrieve(&dsn, &ollama, &allowed, request(&corpus, filters));
        assert_eq!(narrowed["status"], "EMPTY", "response: {narrowed}");
    }

    let malformed = retrieve(
        &dsn,
        &ollama,
        &allowed,
        request(&corpus, json!({"ingested_from": "yesterday"})),
    );
    assert_failure(&malformed, "MALFORMED");
}

#[test]
fn allowed_corpus_without_rows_is_empty() {
    let corpus = format!("retrieval-binary-empty-{}", unique_tag());
    let response = retrieve(&dsn(), &fake_ollama(), &corpus, request(&corpus, json!({})));
    assert_eq!(response["status"], "EMPTY", "response: {response}");
    assert_eq!(response["evidence"], json!([]));
    assert!(response["diagnostics"].get("error_code").is_none());
}

#[test]
fn unauthorized_corpus_is_rejected_without_touching_dependencies() {
    // Both dependencies are unreachable: only a pre-dependency rejection can
    // answer UNAUTHORIZED_SCOPE here.
    for (allowed, corpus) in [("kb", "other"), ("kb", ""), ("", "kb"), ("kb2", "kb")] {
        let response = retrieve(DEAD_DSN, DEAD_OLLAMA, allowed, request(corpus, json!({})));
        assert_failure(&response, "UNAUTHORIZED_SCOPE");
    }
}

#[test]
fn unreachable_database_is_unavailable() {
    let response = retrieve(DEAD_DSN, &fake_ollama(), "kb", request("kb", json!({})));
    assert_failure(&response, "UNAVAILABLE");
}

#[test]
fn unreachable_ollama_is_unavailable() {
    let response = retrieve(&dsn(), DEAD_OLLAMA, "kb", request("kb", json!({})));
    assert_failure(&response, "UNAVAILABLE");
}
