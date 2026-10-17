//! rag-foundation: document processing port of the Go baseline.
//!
//! Provides the domain types ([`document`]), configuration ([`config`]), the
//! document-processing pipeline: Markdown/plain-text loaders, `## ` section
//! parsing, chunking with overlap, and PDF text-layer loading with 1-based page
//! provenance, and the provider integration: a shared Ollama HTTP client
//! ([`ollama`]), embedding ([`embedding`]) and generation ([`generation`])
//! request ports, and provider registration/resolution ([`provider`]).

pub mod chunker;
pub mod config;
pub mod document;
pub mod embedding;
pub mod generation;
pub mod ingest;
pub mod loader;
pub mod ollama;
pub mod provider;
pub mod section;
