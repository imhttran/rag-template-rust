//! Production question-answer entry point.
//!
//! Loads configuration, builds the real Ollama embedder/generator and the
//! PostgreSQL corpus reader, then answers one question through the shared
//! retrieval modules and the single [`AnswerService`] implementation.
//!
//! Usage: `query <question>` (or set `QUESTION`). `RAG_CORPUS` selects the
//! stored corpus (default `trusted`). Dependencies are reported distinctly; a
//! retrieval, database, embedding, or generation failure exits non-zero.

use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use rag_foundation::answer::service::{AnswerService, RetrievalSource};
use rag_foundation::config::Config;
use rag_foundation::document::Chunk;
use rag_foundation::embedding::{Embedder, OllamaEmbedder};
use rag_foundation::generation::OllamaGenerator;
use rag_foundation::ollama::{HttpTransport, OllamaClient};
use rag_foundation::retrieval::{dedup, expand, Candidate};
use rag_foundation::retrieval_pg;
use rag_foundation::storage_pg::PgStore;

const DEFAULT_CORPUS: &str = "trusted";
const DEFAULT_CONTEXT_BUDGET: usize = 4000;

/// Retrieves ranked candidates for a question from the PostgreSQL corpus.
struct DbRetrieval<'a> {
    store: &'a PgStore,
    embedder: &'a OllamaEmbedder,
    corpus: &'a str,
    min_similarity: f64,
    final_k: usize,
    candidate_k: usize,
    expand_limit: u32,
    language: &'a str,
}

impl RetrievalSource for DbRetrieval<'_> {
    fn retrieve(&self, question: &str) -> Result<Vec<Candidate>, String> {
        let rows = self
            .store
            .rows_for_corpus(self.corpus)
            .map_err(|err| format!("load corpus {:?}: {err}", self.corpus))?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let chunks: Vec<Chunk> = rows.iter().map(stored_chunk).collect();

        let query = self
            .embedder
            .embed(question)
            .map_err(|err| format!("embed question: {err}"))?;

        let vector_hits: Vec<_> = self
            .store
            .vector_search(self.corpus, &query, self.candidate_k)
            .map_err(|err| format!("vector search: {err}"))?
            .into_iter()
            .filter(|hit| hit.similarity >= self.min_similarity)
            .collect();
        let keyword_hits = self
            .store
            .keyword_search(self.corpus, question, self.candidate_k, self.language)
            .map_err(|err| format!("keyword search: {err}"))?;
        let mut candidates = retrieval_pg::fuse(&[vector_hits, keyword_hits], self.candidate_k);
        candidates = dedup::dedup(candidates);
        candidates = expand::expand(&candidates, &chunks, self.expand_limit);
        candidates.truncate(self.final_k);
        Ok(candidates)
    }
}

fn stored_chunk(row: &rag_foundation::storage_pg::StoredChunkRow) -> Chunk {
    Chunk {
        source: row.chunk.source.clone().unwrap_or_default(),
        section: row.chunk.section.clone().unwrap_or_default(),
        language: row.chunk.language.clone().unwrap_or_default(),
        chunk_index: row.chunk.chunk_index.max(0) as u32,
        content: row.chunk.content.clone(),
        page: row.chunk.page.and_then(|page| u32::try_from(page).ok()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("query: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let config = Config::load().map_err(|err| format!("configuration error: {err}"))?;
    let question = question(&config)?;
    let corpus = env::var("RAG_CORPUS")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_CORPUS.to_string());

    let embed_transport = Arc::new(HttpTransport::new(config.request_timeout));
    let embed_client = OllamaClient::new(
        config.ollama_url.clone(),
        embed_transport,
        config.request_timeout,
    );
    let embedder = OllamaEmbedder::new(embed_client, config.embed_model.clone())
        .with_embed_dim(config.embed_dim as usize);

    let gen_transport = Arc::new(HttpTransport::new(config.request_timeout));
    let gen_client = OllamaClient::new(
        config.ollama_url.clone(),
        gen_transport,
        config.request_timeout,
    );
    let generator = OllamaGenerator::new(gen_client, config.chat_model.clone());

    let store = PgStore::connect(&config.database_url)
        .map_err(|err| format!("connect to the document store: {err}"))?;

    let retrieval = DbRetrieval {
        store: &store,
        embedder: &embedder,
        corpus: &corpus,
        min_similarity: config.min_similarity,
        final_k: config.final_k.max(1) as usize,
        candidate_k: config.top_k.max(1) as usize,
        expand_limit: config.expand_limit.max(0) as u32,
        language: &config.language,
    };
    let budget = if config.context_budget > 0 {
        config.context_budget as usize
    } else {
        DEFAULT_CONTEXT_BUDGET
    };
    let service = AnswerService::new(&retrieval, &generator, budget)
        .with_citation_validation(config.rag_citation_validation);
    let answer = service
        .answer(&question)
        .map_err(|err| format!("answer failed: {err}"))?;

    println!("{}", answer.text);
    for reference in &answer.references {
        println!("source: {reference}");
    }
    Ok(ExitCode::SUCCESS)
}

fn question(config: &Config) -> Result<String, String> {
    let mut args = env::args().skip(1);
    let from_args = args.next();
    if args.next().is_some() {
        return Err("usage: query <question> (exactly one argument)".to_string());
    }
    let question = from_args
        .or_else(|| {
            let from_config = config.question.trim();
            (!from_config.is_empty()).then(|| from_config.to_string())
        })
        .ok_or_else(|| "usage: query <question>".to_string())?;
    if config.max_question_bytes > 0 && question.len() > config.max_question_bytes as usize {
        return Err(format!(
            "question exceeds MAX_QUESTION_BYTES ({})",
            config.max_question_bytes
        ));
    }
    Ok(question)
}
