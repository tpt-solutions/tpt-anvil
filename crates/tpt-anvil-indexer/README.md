# tpt-anvil-indexer

[![Crates.io](https://img.shields.io/crates/v/tpt-anvil-indexer.svg)](https://crates.io/crates/tpt-anvil-indexer)
[![Docs.rs](https://docs.rs/tpt-anvil-indexer/badge.svg)](https://docs.rs/tpt-anvil-indexer)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Local, language-agnostic code indexing and hybrid search for
[TPT Anvil](https://github.com/tpt-solutions/tpt-anvil).

Like [`tpt-anvil-providers`](../tpt-anvil-providers), this crate is deliberately decoupled from
`tpt-anvil-core` and `tpt-anvil-config` and defines its own `types.rs`, so it can be published
standalone.

## Features

- **Symbol extraction** — `extract_symbols` via Tree-sitter across Rust, Python, JavaScript,
  TypeScript, Go, Java, C, C++, Ruby, PHP, and C#.
- **Call graph** — `extract_call_edges`, `CallGraph`, `CallEdge`, with `callers_of` and
  `callees_of`.
- **AST outlines** — `outline_for_file` and `outline_stats` compress a file into a
  context-efficient signature summary.
- **Vector store** — `IndexStore` over SQLite (`rusqlite`), with `upsert_embeddings`,
  `all_embeddings`, `upsert_file`, `insert_symbols`, `upsert_fts`, `search_fts`, and
  `search_symbols`.
- **BM25 full-text search** — `tantivy`-backed.
- **Embeddings** — the `Embedder` trait with `HashingEmbedder` (offline feature hashing) and
  `OllamaEmbedder`, plus `cosine_similarity`.
- **Fusion** — `reciprocal_rank_fusion` and `RankedItem` / `FusedResult` to merge ranked lists.
- **Traversal and watching** — `walk_project`, `detect_language`, `content_hash`, and the
  `IndexWatcher` for incremental re-indexing.
- `Retriever` ties it together: `Retriever::new` plus `search`.

## Usage

```rust
use tpt_anvil_indexer::retriever::Retriever;
use tpt_anvil_indexer::types::IndexerConfig;

let root = std::path::Path::new("/path/to/project");
let retriever = Retriever::new(root, &IndexerConfig::default())?;

let hits = retriever.search("http retry with exponential backoff").await?;
for chunk in hits.iter().take(5) {
    println!("{} ({:?}, score {:.3})", chunk.file_path, chunk.chunk_type, chunk.score);
}
# Ok::<(), anyhow::Error>(())
```

Working with the pieces directly:

```rust
use tpt_anvil_indexer::symbols::extract_symbols;
use tpt_anvil_indexer::outline::outline_for_file;

let source = "fn main() { helper(); }\nfn helper() {}\n";

let symbols = extract_symbols(source, "rust", "src/main.rs");
println!("found {} symbols", symbols.len());

let outline = outline_for_file(source, "rust", "src/main.rs");
println!("{outline}");
```

## Dependencies

`serde`, `serde_json`, `anyhow`, `thiserror`, `tokio`, `async-trait`, `tracing`, `notify`,
`reqwest`, `futures-executor`, the `tree-sitter-*` grammar crates, `rusqlite` (bundled),
`tantivy`, `walkdir`, and `ignore`.

## License

Dual-licensed under [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE), at your option.

Part of the TPT Anvil project.
