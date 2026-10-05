# Changelog — tpt-anvil-indexer

All notable changes to `tpt-anvil-indexer` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This crate adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- `syntax` module for toolchain-free syntax validation: `syntax_errors`,
  `is_syntactically_valid`, `grammar_for`, and `format_errors`. Parses source
  with tree-sitter and reports `ERROR`/`MISSING` nodes with 1-based positions,
  capped at 10 reported errors. Returns `None` when no grammar is bundled for
  the language, which callers must treat as "cannot check" rather than "clean".
- Tree-sitter symbol extraction for Rust, Python, JavaScript, TypeScript, Go, Java, C, C++,
  Ruby, PHP, and C#.
- Call graph construction: `extract_call_edges`, `CallGraph`, `CallEdge`, `callers_of`, and
  `callees_of`.
- `outline` module for AST outline compression of code summaries, plus `outline_stats`.
- `IndexStore` over SQLite for vector storage and FTS, with `upsert_embeddings`,
  `all_embeddings`, `upsert_file`, `insert_symbols`, `upsert_fts`, `search_fts`, and
  `search_symbols`.
- BM25 full-text search via `tantivy`.
- `Embedder` trait with `HashingEmbedder` (offline feature hashing) and `OllamaEmbedder`,
  plus `cosine_similarity`.
- Hybrid retrieval: `reciprocal_rank_fusion` over `RankedItem` and `FusedResult`.
- `walk_project`, `detect_language`, and `content_hash` for traversal.
- `IndexWatcher` for incremental re-indexing on file change.

### Fixed
- Pinned `tree-sitter-python` to `0.23` to match the other grammar crates. Under `0.25`
  the generated grammar's `LANGUAGE` constant is incompatible with `tree-sitter 0.24`,
  so `set_language` failed and Python symbol/call-edge extraction silently returned
  nothing.

### Changed
- `store::blob_to_vector` now uses `slice::as_chunks` instead of `chunks_exact`, silencing
  the `chunks_exact` constant-chunk-size lint that failed `clippy -D warnings`.

### Notes
- This crate is deliberately decoupled from `tpt-anvil-core` and `tpt-anvil-config` so it can
  be published standalone to crates.io. It defines its own `types.rs`. Please do not re-add a
  core dependency.

[Unreleased]: https://github.com/tpt-solutions/tpt-anvil/commits/master/crates/tpt-anvil-indexer