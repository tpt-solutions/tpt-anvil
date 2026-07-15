// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json;

use crate::symbols::Symbol;

pub struct IndexStore {
    conn: Connection,
}

impl IndexStore {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                mtime INTEGER,
                content_hash TEXT
            );

            CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY,
                file_id INTEGER REFERENCES files(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                start_line INTEGER,
                end_line INTEGER,
                signature TEXT,
                doc_comment TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_id);

            CREATE VIRTUAL TABLE IF NOT EXISTS fts_content
            USING fts5(file_path, content, tokenize='porter ascii');
            ",
        )?;
        Ok(())
    }

    pub fn upsert_file(&self, path: &str, mtime: i64, hash: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO files (path, mtime, content_hash) VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET mtime=excluded.mtime, content_hash=excluded.content_hash",
            params![path, mtime, hash],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn file_id(&self, path: &str) -> Result<Option<i64>> {
        let mut stmt = self.conn.prepare("SELECT id FROM files WHERE path = ?1")?;
        let mut rows = stmt.query(params![path])?;
        Ok(rows.next()?.map(|r| r.get::<_, i64>(0).unwrap()))
    }

    pub fn insert_symbols(&self, file_id: i64, symbols: &[Symbol]) -> Result<()> {
        self.conn.execute("DELETE FROM symbols WHERE file_id = ?1", params![file_id])?;
        let mut stmt = self.conn.prepare(
            "INSERT INTO symbols (file_id, name, kind, start_line, end_line, signature, doc_comment)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for sym in symbols {
            stmt.execute(params![
                file_id,
                sym.name,
                serde_json::to_string(&sym.kind).unwrap_or_default(),
                sym.start_line,
                sym.end_line,
                sym.signature,
                sym.doc_comment,
            ])?;
        }
        Ok(())
    }

    pub fn upsert_fts(&self, file_path: &str, content: &str) -> Result<()> {
        self.conn.execute("DELETE FROM fts_content WHERE file_path = ?1", params![file_path])?;
        self.conn.execute(
            "INSERT INTO fts_content (file_path, content) VALUES (?1, ?2)",
            params![file_path, content],
        )?;
        Ok(())
    }

    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, snippet(fts_content, 1, '<b>', '</b>', '...', 20)
             FROM fts_content
             WHERE fts_content MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![query, limit as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn search_symbols(&self, name: &str, limit: usize) -> Result<Vec<Symbol>> {
        let pattern = format!("%{name}%");
        let mut stmt = self.conn.prepare(
            "SELECT s.name, s.kind, f.path, s.start_line, s.end_line, s.signature, s.doc_comment
             FROM symbols s JOIN files f ON s.file_id = f.id
             WHERE s.name LIKE ?1
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit as i64], |row| {
            Ok(Symbol {
                name: row.get(0)?,
                kind: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(crate::symbols::SymbolKind::Unknown),
                file_path: row.get(2)?,
                start_line: row.get(3)?,
                end_line: row.get(4)?,
                signature: row.get(5)?,
                doc_comment: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
