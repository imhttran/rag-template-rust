//! SQL-backed retrieval fusion shared by the production query and eval paths.
//!
//! The Go baseline fuses pgvector and PostgreSQL full-text search results with
//! reciprocal rank fusion (k = 60), orders ties by best rank and then by the
//! database row id. This module mirrors that ordering exactly so the Rust
//! production path and the Go reference agree on ranking and metrics.

use std::collections::HashMap;

use crate::document::Chunk;
use crate::retrieval::Candidate;
use crate::storage_pg::{SqlHit, StoredChunkRow};

/// Fuses ranked SQL hit lists with reciprocal rank fusion.
///
/// Each occurrence contributes `1 / (60 + rank)` (rank is 1-based). The result
/// is ordered by fused score descending, then best rank ascending, then row id
/// ascending, matching the Go `FuseRankings` contract. At most `top_k` results
/// are returned.
pub fn fuse(rankings: &[Vec<SqlHit>], top_k: usize) -> Vec<Candidate> {
    fuse_hits(rankings, top_k)
        .into_iter()
        .map(|(hit, score)| Candidate::new(chunk(&hit.row), score))
        .collect()
}

/// [`fuse`] that keeps each fused [`SqlHit`] (row id, content hash, corpus)
/// alongside its RRF score, for callers that report stored provenance.
pub fn fuse_hits(rankings: &[Vec<SqlHit>], top_k: usize) -> Vec<(SqlHit, f64)> {
    struct Entry {
        hit: SqlHit,
        score: f64,
        best_rank: usize,
    }

    let mut entries: HashMap<i64, Entry> = HashMap::new();
    for list in rankings {
        for (index, hit) in list.iter().enumerate() {
            let rank = index + 1;
            let entry = entries.entry(hit.row.id).or_insert_with(|| Entry {
                hit: hit.clone(),
                score: 0.0,
                best_rank: rank,
            });
            entry.score += 1.0 / (60.0 + rank as f64);
            if rank < entry.best_rank {
                entry.best_rank = rank;
            }
            if hit.similarity > entry.hit.similarity {
                entry.hit.similarity = hit.similarity;
            }
            if hit.keyword_score > entry.hit.keyword_score {
                entry.hit.keyword_score = hit.keyword_score;
            }
        }
    }

    let mut fused: Vec<Entry> = entries.into_values().collect();
    fused.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.best_rank.cmp(&b.best_rank))
            .then_with(|| a.hit.row.id.cmp(&b.hit.row.id))
    });
    fused.truncate(top_k.min(fused.len()));
    fused
        .into_iter()
        .map(|entry| (entry.hit, entry.score))
        .collect()
}

fn chunk(row: &StoredChunkRow) -> Chunk {
    Chunk {
        source: row.chunk.source.clone().unwrap_or_default(),
        section: row.chunk.section.clone().unwrap_or_default(),
        language: row.chunk.language.clone().unwrap_or_default(),
        chunk_index: row.chunk.chunk_index.max(0) as u32,
        content: row.chunk.content.clone(),
        page: row.chunk.page.and_then(|page| u32::try_from(page).ok()),
    }
}
