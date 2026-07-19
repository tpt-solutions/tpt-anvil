// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Hybrid retrieval: Reciprocal Rank Fusion (RRF) of BM25 lexical results and
//! vector cosine-similarity results.
//!
//! RRF combines multiple ranked result lists without needing to normalize the
//! heterogeneous scores (BM25 rank vs. cosine similarity). For each document
//! `d` its fused score is:
//!
//! ```text
//! RRF(d) = sum over lists L of 1 / (k + rank_L(d))
//! ```
//!
//! where `rank_L(d)` is the 1-based rank of `d` in list `L` and `k` is a
//! smoothing constant (60 is the value from the original Cormack et al. paper).

use std::collections::HashMap;

/// A candidate produced by one retrieval stage, already sorted best-first by
/// the caller. Only the ordering matters to RRF, not the raw score.
#[derive(Debug, Clone)]
pub struct RankedItem {
    /// Stable identity used to align the same document across lists
    /// (e.g. "path#chunk_index").
    pub key: String,
    /// Original text/snippet carried through to the fused output.
    pub content: String,
    /// File the item originates from.
    pub file_path: String,
    /// The stage's own score (informational only; not used by RRF).
    pub raw_score: f32,
}

/// A fused result after RRF.
#[derive(Debug, Clone)]
pub struct FusedResult {
    pub key: String,
    pub content: String,
    pub file_path: String,
    pub rrf_score: f32,
}

/// The standard RRF smoothing constant.
pub const DEFAULT_RRF_K: f32 = 60.0;

/// Fuse any number of ranked lists using Reciprocal Rank Fusion.
///
/// Each inner slice must be pre-sorted best-first. Returns results sorted by
/// descending fused score, truncated to `top_k`.
pub fn reciprocal_rank_fusion(lists: &[Vec<RankedItem>], k: f32, top_k: usize) -> Vec<FusedResult> {
    let mut scores: HashMap<String, f32> = HashMap::new();
    let mut meta: HashMap<String, (String, String)> = HashMap::new();

    for list in lists {
        for (rank, item) in list.iter().enumerate() {
            let contribution = 1.0 / (k + (rank as f32 + 1.0));
            *scores.entry(item.key.clone()).or_insert(0.0) += contribution;
            meta.entry(item.key.clone())
                .or_insert_with(|| (item.content.clone(), item.file_path.clone()));
        }
    }

    let mut fused: Vec<FusedResult> = scores
        .into_iter()
        .map(|(key, rrf_score)| {
            let (content, file_path) = meta.remove(&key).unwrap_or_default();
            FusedResult {
                key,
                content,
                file_path,
                rrf_score,
            }
        })
        .collect();

    fused.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.key.cmp(&b.key))
    });
    fused.truncate(top_k);
    fused
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(key: &str) -> RankedItem {
        RankedItem {
            key: key.into(),
            content: format!("content-{key}"),
            file_path: format!("{key}.rs"),
            raw_score: 0.0,
        }
    }

    #[test]
    fn document_in_both_lists_ranks_highest() {
        let bm25 = vec![item("a"), item("b"), item("c")];
        let vector = vec![item("b"), item("d"), item("a")];
        let fused = reciprocal_rank_fusion(&[bm25, vector], DEFAULT_RRF_K, 10);
        // "a" (ranks 1 & 3) and "b" (ranks 2 & 1) appear in both lists and
        // should outrank single-list docs "c" and "d".
        let top_two: Vec<&str> = fused.iter().take(2).map(|f| f.key.as_str()).collect();
        assert!(top_two.contains(&"a"));
        assert!(top_two.contains(&"b"));
    }

    #[test]
    fn respects_top_k() {
        let l = vec![item("a"), item("b"), item("c")];
        let fused = reciprocal_rank_fusion(&[l], DEFAULT_RRF_K, 2);
        assert_eq!(fused.len(), 2);
    }

    #[test]
    fn empty_lists_produce_empty_output() {
        let fused = reciprocal_rank_fusion(&[], DEFAULT_RRF_K, 5);
        assert!(fused.is_empty());
    }

    #[test]
    fn single_list_preserves_order() {
        let l = vec![item("x"), item("y"), item("z")];
        let fused = reciprocal_rank_fusion(&[l], DEFAULT_RRF_K, 10);
        assert_eq!(fused[0].key, "x");
        assert_eq!(fused[1].key, "y");
        assert_eq!(fused[2].key, "z");
    }
}
