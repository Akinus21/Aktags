use anyhow::Result;
use chrono::{DateTime, Utc};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub type DbPool = Pool<SqliteConnectionManager>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub extension: String,
    pub size_bytes: i64,
    pub file_hash: Option<String>,
    pub category: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub tagged_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchFilter {
    pub query: Option<String>,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

impl Default for SearchFilter {
    fn default() -> Self {
        Self { query: None, tags: vec![], category: None, limit: 500, offset: 0 }
    }
}

#[derive(Debug, Clone)]
pub struct DbStats {
    pub total: i64,
    pub errors: i64,
    pub untagged: i64,
    pub by_category: Vec<(String, i64)>,
}

pub fn create_pool(db_path: &Path) -> Result<DbPool> {
    std::fs::create_dir_all(db_path.parent().unwrap())?;
    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::new(manager)?;
    init_schema(&pool)?;
    Ok(pool)
}

fn init_schema(pool: &DbPool) -> Result<()> {
    let conn = pool.get()?;
    conn.execute_batch(r#"
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS files (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            path        TEXT UNIQUE NOT NULL,
            filename    TEXT NOT NULL,
            extension   TEXT,
            size_bytes  INTEGER,
            file_hash   TEXT,
            category    TEXT,
            summary     TEXT,
            tags        TEXT DEFAULT '[]',
            tagged_at   TEXT,
            modified_at TEXT,
            indexed_at  TEXT DEFAULT (datetime('now')),
            error       TEXT
        );

        CREATE TABLE IF NOT EXISTS tag_index (
            tag     TEXT NOT NULL,
            file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
            PRIMARY KEY (tag, file_id)
        );

        CREATE INDEX IF NOT EXISTS idx_files_path      ON files(path);
        CREATE INDEX IF NOT EXISTS idx_files_category  ON files(category);
        CREATE INDEX IF NOT EXISTS idx_files_extension ON files(extension);
        CREATE INDEX IF NOT EXISTS idx_tag_index_tag   ON tag_index(tag);

        CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
            filename, summary, tags, content=files, content_rowid=id
        );

        CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files BEGIN
            INSERT INTO files_fts(rowid, filename, summary, tags)
            VALUES (new.id, new.filename, new.summary, new.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files BEGIN
            INSERT INTO files_fts(files_fts, rowid, filename, summary, tags)
            VALUES ('delete', old.id, old.filename, old.summary, old.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE ON files BEGIN
            INSERT INTO files_fts(files_fts, rowid, filename, summary, tags)
            VALUES ('delete', old.id, old.filename, old.summary, old.tags);
            INSERT INTO files_fts(rowid, filename, summary, tags)
            VALUES (new.id, new.filename, new.summary, new.tags);
        END;
    "#)?;
    Ok(())
}

pub fn file_hash(path: &Path) -> String {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else { return String::new(); };
    let mut ctx = md5::Context::new();
    let mut buf = [0u8; 65536];
    loop {
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => ctx.consume(&buf[..n]),
            Err(_) => break,
        }
    }
    format!("{:x}", ctx.compute())
}

pub fn needs_reindex(pool: &DbPool, path: &str, hash: &str) -> Result<bool> {
    let conn = pool.get()?;
    let result: Option<String> = conn.query_row(
        "SELECT file_hash FROM files WHERE path=?",
        params![path],
        |row| row.get(0),
    ).ok();
    Ok(result.map(|h| h != hash).unwrap_or(true))
}

pub fn upsert_file(
    pool: &DbPool,
    path: &str,
    category: &str,
    summary: &str,
    tags: &[String],
    size_bytes: i64,
    hash: &str,
    error: Option<&str>,
) -> Result<i64> {
    let conn = pool.get()?;
    let tags_json = serde_json::to_string(tags)?;
    let now = Utc::now().to_rfc3339();
    let filename = Path::new(path)
        .file_name().unwrap_or_default()
        .to_string_lossy().to_string();
    let ext = Path::new(path)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default();

    conn.execute(r#"
        INSERT INTO files
            (path, filename, extension, size_bytes, file_hash, category, summary, tags, tagged_at, modified_at, error)
        VALUES (?,?,?,?,?,?,?,?,?,?,?)
        ON CONFLICT(path) DO UPDATE SET
            filename    = excluded.filename,
            extension   = excluded.extension,
            size_bytes  = excluded.size_bytes,
            file_hash   = excluded.file_hash,
            category    = excluded.category,
            summary     = excluded.summary,
            tags        = excluded.tags,
            tagged_at   = excluded.tagged_at,
            modified_at = excluded.modified_at,
            error       = excluded.error
    "#, params![
        path, filename, ext, size_bytes, hash,
        category, summary, tags_json, now, now, error
    ])?;

    let file_id: i64 = conn.query_row(
        "SELECT id FROM files WHERE path=?",
        params![path],
        |row| row.get(0),
    )?;

    conn.execute("DELETE FROM tag_index WHERE file_id=?", params![file_id])?;
    for tag in tags {
        conn.execute(
            "INSERT OR IGNORE INTO tag_index (tag, file_id) VALUES (?,?)",
            params![tag.to_lowercase(), file_id],
        )?;
    }
    Ok(file_id)
}

pub fn upsert_tags(pool: &DbPool, file_id: i64, tags: &[String]) -> Result<()> {
    let conn = pool.get()?;
    let tags_json = serde_json::to_string(tags)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE files SET tags=?, tagged_at=? WHERE id=?",
        params![tags_json, now, file_id],
    )?;
    conn.execute("DELETE FROM tag_index WHERE file_id=?", params![file_id])?;
    for tag in tags {
        conn.execute(
            "INSERT OR IGNORE INTO tag_index (tag, file_id) VALUES (?,?)",
            params![tag.to_lowercase(), file_id],
        )?;
    }
    Ok(())
}

pub fn remove_file(pool: &DbPool, path: &str) -> Result<()> {
    let conn = pool.get()?;
    conn.execute("DELETE FROM files WHERE path=?", params![path])?;
    Ok(())
}

pub fn search_files(pool: &DbPool, filter: &SearchFilter) -> Result<Vec<FileRecord>> {
    let conn = pool.get()?;

    // Build SQL manually to avoid dynamic param boxing issues
    let mut sql = String::new();
    let mut use_fts = false;

    if let Some(q) = &filter.query {
        if !q.is_empty() {
            use_fts = true;
            sql = format!(r#"
                SELECT f.id, f.path, f.filename, f.extension, f.category,
                       f.summary, f.tags, f.tagged_at, f.size_bytes, f.error
                FROM files_fts fts
                JOIN files f ON fts.rowid = f.id
                WHERE files_fts MATCH ?1
            "#);
        }
    }

    if !use_fts {
        sql = r#"
            SELECT id, path, filename, extension, category,
                   summary, tags, tagged_at, size_bytes, error
            FROM files WHERE 1=1
        "#.to_string();

        if !filter.tags.is_empty() {
            for _ in &filter.tags {
                sql.push_str(" AND id IN (SELECT file_id FROM tag_index WHERE tag=?)");
            }
        }
        if filter.category.is_some() {
            sql.push_str(" AND category=?");
        }
    } else {
        if !filter.tags.is_empty() {
            for _ in &filter.tags {
                sql.push_str(" AND f.id IN (SELECT file_id FROM tag_index WHERE tag=?)");
            }
        }
        if filter.category.is_some() {
            sql.push_str(" AND f.category=?");
        }
    }

    sql.push_str(&format!(" ORDER BY {} DESC LIMIT {} OFFSET {}",
        if use_fts { "f.tagged_at" } else { "tagged_at" },
        filter.limit, filter.offset
    ));

    // Build params vec as strings, then execute
    let mut param_strs: Vec<String> = Vec::new();
    if let Some(q) = &filter.query {
        if !q.is_empty() {
            param_strs.push(format!("{q}*"));
        }
    }
    for tag in &filter.tags {
        param_strs.push(tag.to_lowercase());
    }
    if let Some(cat) = &filter.category {
        param_strs.push(cat.clone());
    }

    let params_refs: Vec<&dyn rusqlite::ToSql> = param_strs.iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        rusqlite::params_from_iter(params_refs.iter()),
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, Option<String>>(9)?,
            ))
        }
    )?.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut records = Vec::new();
    for (id, path, filename, ext, category, summary, tags_json, tagged_at, size, error) in rows {
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let tagged_at = tagged_at
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc));
        records.push(FileRecord {
            id, path, filename, extension: ext, category,
            summary, tags, tagged_at, size_bytes: size, error,
            file_hash: None,
        });
    }
    Ok(records)
}

pub fn all_tags(pool: &DbPool) -> Result<Vec<(String, i64)>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT tag, COUNT(*) as count FROM tag_index GROUP BY tag ORDER BY count DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn get_stats(pool: &DbPool) -> Result<DbStats> {
    let conn = pool.get()?;
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
    let errors: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files WHERE error IS NOT NULL", [], |r| r.get(0)
    )?;
    let untagged: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files WHERE tagged_at IS NULL", [], |r| r.get(0)
    )?;
    let mut stmt = conn.prepare(
        "SELECT category, COUNT(*) FROM files GROUP BY category ORDER BY COUNT(*) DESC"
    )?;
    let by_category = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(DbStats { total, errors, untagged, by_category })
}

pub fn get_file_by_id(pool: &DbPool, id: i64) -> Result<Option<FileRecord>> {
    let conn = pool.get()?;
    let result = conn.query_row(
        r#"SELECT id, path, filename, extension, category, summary, tags,
                  tagged_at, size_bytes, error
           FROM files WHERE id=?"#,
        params![id],
        |row| Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, Option<String>>(9)?,
        )),
    ).ok();

    Ok(result.map(|(id, path, filename, ext, category, summary, tags_json, tagged_at, size, error)| {
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let tagged_at = tagged_at
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc));
        FileRecord {
            id, path, filename, extension: ext, category,
            summary, tags, tagged_at, size_bytes: size, error,
            file_hash: None,
        }
    }))
}

pub fn clear_errors(pool: &DbPool) -> Result<usize> {
    let conn = pool.get()?;
    let n = conn.execute(
        "UPDATE files SET error=NULL, file_hash=NULL, tagged_at=NULL WHERE error IS NOT NULL",
        [],
    )?;
    Ok(n)
}
