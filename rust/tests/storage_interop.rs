//! RUST-005C database interoperability test (real database behaviour).
//!
//! This test **fails clearly** when credentials or pgvector are unavailable; it
//! never skips to a pass, so a missing database can never yield a green suite.
//! It writes a chunk through the production Rust store, reads it back, and then
//! verifies only its own rows so existing data is preserved.

use rag_foundation::storage::{DocumentStore, StoredChunk, EMBEDDING_DIM};
use rag_foundation::storage_pg::PgStore;

fn dsn() -> String {
    match std::env::var("DATABASE_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => panic!(
            "DATABASE_URL is not set: database acceptance requires credentials. \
             Load the repository .env and export DATABASE_URL. This test fails (never skips)."
        ),
    }
}

fn probe_pgvector(dsn: &str) -> String {
    let mut client = postgres::Client::connect(dsn, postgres::NoTls)
        .unwrap_or_else(|e| panic!("connect to the migration database failed: {e}"));
    let row = client
        .query_opt(
            "SELECT extversion FROM pg_extension WHERE extname = 'vector'",
            &[],
        )
        .unwrap_or_else(|e| panic!("pgvector probe failed: {e}"));
    match row {
        Some(r) => r.get::<_, String>(0),
        None => panic!("pgvector is not available in this database; the acceptance check fails, it does not skip"),
    }
}

#[test]
fn rust_store_round_trips_and_reads_go_schema_columns() {
    let dsn = dsn();
    let version = probe_pgvector(&dsn);
    assert!(!version.is_empty(), "pgvector must report a version");

    let store = PgStore::connect(&dsn).expect("open the Rust store");

    let tag = format!("{}{}", std::process::id(), "-rt");
    let source = format!("interop://rust/{tag}");
    let hash = format!("interop-rust-{tag}");

    let mut embedding = vec![0.0_f64; EMBEDDING_DIM];
    embedding[0] = 0.5;
    embedding[EMBEDDING_DIM - 1] = 1.0;

    let chunk = StoredChunk {
        content: "interop row written by Rust".to_string(),
        source: Some(source.clone()),
        section: Some("Interop".to_string()),
        chunk_index: 0,
        language: Some("english".to_string()),
        page: Some(1),
        corpus: Some("interop".to_string()),
        content_hash: hash.clone(),
        embed_model: "test-model".to_string(),
        embedding_dim: EMBEDDING_DIM as i32,
        chunker_config: "{}".to_string(),
        ingested_at: "2026-10-17T00:00:00Z".to_string(),
        embedding,
    };
    store.write_chunk(&chunk).expect("write the interop row");

    // Rust reads back the full column set a foreign writer (Go) defines.
    let rows = store
        .find_by_content_hash(&hash, Some(source.as_str()))
        .expect("read back by content hash");
    assert_eq!(rows.len(), 1, "exactly one row must round-trip");
    let back = &rows[0];
    assert_eq!(back.chunk.source.as_deref(), Some(source.as_str()));
    assert_eq!(back.chunk.section.as_deref(), Some("Interop"));
    assert_eq!(back.chunk.page, Some(1));
    assert_eq!(back.chunk.language.as_deref(), Some("english"));
    assert_eq!(back.chunk.chunk_index, 0);
    assert_eq!(back.chunk.embedding_dim, EMBEDDING_DIM as i32);
    assert_eq!(back.chunk.embedding_dim(), EMBEDDING_DIM);

    let by_id = store
        .find_by_id(back.id)
        .expect("read by id")
        .expect("the row exists");
    assert_eq!(by_id.chunk.content, "interop row written by Rust");

    // The dimension guard must reject without inserting anything.
    let bad = StoredChunk {
        content: "should never be stored".to_string(),
        source: Some(source.clone()),
        section: Some("Bad".to_string()),
        chunk_index: 0,
        language: None,
        page: None,
        corpus: None,
        content_hash: format!("{hash}-bad"),
        embed_model: "test-model".to_string(),
        embedding_dim: 3,
        chunker_config: "{}".to_string(),
        ingested_at: "2026-10-17T00:00:00Z".to_string(),
        embedding: vec![1.0, 2.0, 3.0],
    };
    assert!(
        store.write_chunk(&bad).is_err(),
        "a dimension mismatch must error before any write"
    );
    let bad_rows = store
        .find_by_content_hash(&format!("{hash}-bad"), Some(source.as_str()))
        .expect("read the rejected hash");
    assert!(bad_rows.is_empty(), "a rejected write must insert no row");

    // Preserve existing data: remove only this test's rows.
    let mut client = postgres::Client::connect(&dsn, postgres::NoTls).expect("cleanup connect");
    let removed = client
        .execute("DELETE FROM documents WHERE source = $1", &[&source])
        .expect("cleanup");
    assert!(removed >= 1, "cleanup removed this test's rows");
}

// ---- Cross-language directions (Go <-> Rust) --------------------------------

fn go_helper(mode: &str, hash: &str) -> String {
    let out = std::process::Command::new("go")
        .args(["run", "./tools/interop", mode, hash])
        .current_dir("..")
        .output()
        .unwrap_or_else(|e| panic!("Go toolchain required for the Go side of interop: {e}"));
    if !out.status.success() {
        panic!(
            "go interop {mode} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn cleanup(dsn: &str, hash_like: &str) {
    let mut c = postgres::Client::connect(dsn, postgres::NoTls).expect("cleanup connect");
    c.execute(
        "DELETE FROM documents WHERE content_hash LIKE $1",
        &[&hash_like],
    )
    .expect("cleanup");
}

#[test]
fn rust_reads_a_go_written_row_with_metadata_and_embedding() {
    let dsn = dsn();
    probe_pgvector(&dsn);
    let tag = format!("{}{}", std::process::id(), "-go");
    let hash = format!("interop-go-{tag}");
    go_helper("write", &hash);

    let store = PgStore::connect(&dsn).expect("open the Rust store");
    let rows = store
        .find_by_content_hash(&hash, Some(&format!("interop://go/{hash}")))
        .expect("read the Go-written row");
    assert_eq!(rows.len(), 1, "Rust must see the Go-written row");
    let c = &rows[0].chunk;
    assert_eq!(c.content, "interop row written by Go");
    assert_eq!(c.section.as_deref(), Some("Interop"));
    assert_eq!(c.language.as_deref(), Some("english"));
    assert_eq!(c.page, Some(1));
    assert_eq!(c.embed_model, "go-test-model");
    assert_eq!(c.embedding_dim(), EMBEDDING_DIM);

    go_helper("delete", &hash);
    cleanup(&dsn, &format!("interop-go-{tag}%"));
}

#[test]
fn go_reads_a_rust_written_row_with_metadata_and_embedding() {
    let dsn = dsn();
    probe_pgvector(&dsn);
    let tag = format!("{}{}", std::process::id(), "-rgo");
    let source = format!("interop://rust/{tag}");
    let hash = format!("interop-rust-go-{tag}");

    let store = PgStore::connect(&dsn).expect("open the Rust store");
    let mut embedding = vec![0.0_f64; EMBEDDING_DIM];
    embedding[0] = 0.5;
    let chunk = StoredChunk {
        content: "interop row written by Rust (read by Go)".to_string(),
        source: Some(source.clone()),
        section: Some("Interop".to_string()),
        chunk_index: 0,
        language: Some("english".to_string()),
        page: Some(1),
        corpus: Some("interop".to_string()),
        content_hash: hash.clone(),
        embed_model: "rust-test-model".to_string(),
        embedding_dim: EMBEDDING_DIM as i32,
        chunker_config: "{}".to_string(),
        ingested_at: "2026-10-17T00:00:00Z".to_string(),
        embedding,
    };
    store.write_chunk(&chunk).expect("Rust writes the row");

    let out = go_helper("read", &hash);
    assert!(
        out.contains("\"embed_model\":\"rust-test-model\""),
        "go read: {out}"
    );
    assert!(out.contains("\"embedding_dim\":768"), "go read: {out}");
    assert!(
        out.contains(&format!("\"embedding_len\":{EMBEDDING_DIM}")),
        "go read: {out}"
    );
    assert!(out.contains("\"section\":\"Interop\""), "go read: {out}");

    cleanup(&dsn, &format!("interop-rust-go-{tag}%"));
}
