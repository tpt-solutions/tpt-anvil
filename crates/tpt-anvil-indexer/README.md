# tpt-anvil-indexer

Local, language-agnostic code indexing and hybrid search for TPT Anvil.

- Tree-sitter based symbol extraction and call-graph construction across Rust, Python, TypeScript/JavaScript, Go, Java, C/C++, Ruby, PHP, and C#.
- Local vector storage (SQLite) and BM25 full-text search (Tantivy), fused via reciprocal rank fusion.
- File-watcher based incremental re-indexing.

Part of the [TPT Anvil](https://github.com/tpt-solutions/tpt-anvil) project. Dual-licensed under MIT OR Apache-2.0.
