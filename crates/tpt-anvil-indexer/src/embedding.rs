// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Local embedding generation for vector search.
//!
//! Anvil supports two embedding sources:
//!
//! * [`HashingEmbedder`] — a dependency-free, deterministic embedder that maps
//!   text into a fixed-dimensional vector using feature hashing over token
//!   n-grams. It requires no model download and always works offline, making it
//!   a reasonable default and an excellent test fixture.
//! * [`OllamaEmbedder`] — calls a local Ollama server's `/api/embeddings`
//!   endpoint (e.g. `nomic-embed-text` / `nomic-embed-code`) to produce true
//!   neural embeddings when available.
//!
//! Both implement the [`Embedder`] trait so the retriever can be configured to
//! use whichever is available.

use anyhow::Result;
use async_trait::async_trait;

/// Produces dense vector embeddings for text.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Dimensionality of the produced vectors.
    fn dimensions(&self) -> usize;

    /// Embed a single piece of text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Embed a batch of texts. Default implementation embeds sequentially.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut out = Vec::with_capacity(texts.len());
        for t in texts {
            out.push(self.embed(t).await?);
        }
        Ok(out)
    }
}

/// Cosine similarity between two equal-length vectors. Returns 0.0 for
/// mismatched lengths or zero-magnitude vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Deterministic, offline feature-hashing embedder.
///
/// Tokens are lowercased word characters; each token is hashed into one of
/// `dims` buckets with a sign, producing an L2-normalized bag-of-words vector.
pub struct HashingEmbedder {
    dims: usize,
}

impl HashingEmbedder {
    pub fn new(dims: usize) -> Self {
        Self { dims: dims.max(1) }
    }

    fn hash_token(token: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        token.hash(&mut h);
        h.finish()
    }
}

impl Default for HashingEmbedder {
    fn default() -> Self {
        Self::new(384)
    }
}

#[async_trait]
impl Embedder for HashingEmbedder {
    fn dimensions(&self) -> usize {
        self.dims
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut v = vec![0.0f32; self.dims];
        let mut token = String::new();
        let flush = |token: &mut String, v: &mut [f32]| {
            if token.is_empty() {
                return;
            }
            let h = Self::hash_token(token);
            let idx = (h % v.len() as u64) as usize;
            let sign = if (h >> 63) & 1 == 1 { -1.0 } else { 1.0 };
            v[idx] += sign;
            token.clear();
        };
        for ch in text.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                token.extend(ch.to_lowercase());
            } else {
                flush(&mut token, &mut v);
            }
        }
        flush(&mut token, &mut v);

        // L2 normalize.
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in v.iter_mut() {
                *x /= norm;
            }
        }
        Ok(v)
    }
}

/// Neural embedder backed by an Ollama server (`/api/embeddings`).
pub struct OllamaEmbedder {
    base_url: String,
    model: String,
    dims: usize,
    client: reqwest::Client,
}

impl OllamaEmbedder {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>, dims: usize) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            dims,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Embedder for OllamaEmbedder {
    fn dimensions(&self) -> usize {
        self.dims
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[derive(serde::Serialize)]
        struct Req<'a> {
            model: &'a str,
            prompt: &'a str,
        }
        #[derive(serde::Deserialize)]
        struct Resp {
            embedding: Vec<f32>,
        }
        let url = format!("{}/api/embeddings", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&Req {
                model: &self.model,
                prompt: text,
            })
            .send()
            .await?
            .error_for_status()?
            .json::<Resp>()
            .await?;
        Ok(resp.embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hashing_embedder_is_deterministic() {
        let e = HashingEmbedder::new(64);
        let a = e.embed("hello world").await.unwrap();
        let b = e.embed("hello world").await.unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[tokio::test]
    async fn similar_text_has_higher_similarity() {
        let e = HashingEmbedder::new(256);
        let q = e.embed("parse the json config file").await.unwrap();
        let close = e.embed("parse json config").await.unwrap();
        let far = e.embed("render the gpu shader pipeline").await.unwrap();
        assert!(cosine_similarity(&q, &close) > cosine_similarity(&q, &far));
    }

    #[test]
    fn cosine_edge_cases() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
    }
}
