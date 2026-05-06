use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Graveyard entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct GraveyardEntry {
    pub id: i64,
    pub original_path: String,
    pub object_hash: String,
    pub original_hash: String,
    pub size_bytes: i64,
    pub compressed_bytes: i64,
    pub replaced_by: String,
    pub replaced_at: DateTime<Utc>,
    pub peer_node_id: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub tags: Option<String>,
    pub summary: Option<String>,
}

fn graveyard_db_path() -> PathBuf {
    let path = std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    path.join(".graveyard").join("graveyard.db")
}

fn objects_dir() -> PathBuf {
    let path = std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    path.join(".graveyard").join("objects")
}

fn init_graveyard_db() -> Result<()> {
    let db_path = graveyard_db_path();
    std::fs::create_dir_all(db_path.parent().unwrap())?;

    let conn = rusqlite::Connection::open(&db_path)?;
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS graveyard (
            id               INTEGER PRIMARY KEY,
            original_path    TEXT NOT NULL,
            object_hash      TEXT NOT NULL,
            original_hash    TEXT NOT NULL,
            size_bytes       INTEGER NOT NULL,
            compressed_bytes INTEGER NOT NULL,
            replaced_by      TEXT NOT NULL,
            replaced_at      TEXT NOT NULL,
            peer_node_id     TEXT,
            expires_at       TEXT NOT NULL,
            tags             TEXT,
            summary          TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_graveyard_path    ON graveyard(original_path);
        CREATE INDEX IF NOT EXISTS idx_graveyard_expires ON graveyard(expires_at);
    "#)?;
    Ok(())
}

/// Compress and archive a file into the graveyard.
pub fn entomb(
    original_path: &Path,
    original_hash: &str,
    replaced_by: &str,
    peer_node_id: Option<&str>,
    tags: Option<&[String]>,
    summary: Option<&str>,
    ttl_days: u32,
) -> Result<()> {
    init_graveyard_db()?;

    let content = fs::read(original_path)
        .with_context(|| format!("Reading file for graveyard: {}", original_path.display()))?;
    let size_bytes = content.len() as i64;

    // SHA-256 of original content (object hash)
    let object_hash = sha256_string(&content);
    let compressed = zstd::encode_all(&content[..],
        3 // compression level
    ).with_context(|| "zstd compression failed")?;
    let compressed_bytes = compressed.len() as i64;

    // Write object to content-addressed store
    let obj_dir = objects_dir();
    std::fs::create_dir_all(&obj_dir)?;
    let prefix = &object_hash[..2];
    let suffix = &object_hash[2..];
    let obj_path = obj_dir.join(prefix).join(format!("{}.zst", suffix));
    std::fs::create_dir_all(obj_path.parent().unwrap())?;
    fs::write(&obj_path, compressed)?;

    let replaced_at = Utc::now();
    let expires_at = replaced_at + chrono::Duration::days(ttl_days as i64);

    let db_path = graveyard_db_path();
    let conn = rusqlite::Connection::open(&db_path)?;
    conn.execute(r#"
        INSERT INTO graveyard
            (original_path, object_hash, original_hash, size_bytes, compressed_bytes,
             replaced_by, replaced_at, peer_node_id, expires_at, tags, summary)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
    "#, params![
        original_path.to_string_lossy().to_string(),
        &object_hash,
        original_hash,
        size_bytes,
        compressed_bytes,
        replaced_by,
        replaced_at.to_rfc3339(),
        peer_node_id,
        expires_at.to_rfc3339(),
        tags.map(|t| serde_json::to_string(t).unwrap_or_default()),
        summary,
    ])?;

    Ok(())
}

/// Restore a file from the graveyard. Returns the content bytes.
#[allow(dead_code)]
pub fn unearth(original_path: &Path) -> Result<Option<Vec<u8>>> {
    let db_path = graveyard_db_path();
    if !db_path.exists() {
        return Ok(None);
    }
    let conn = rusqlite::Connection::open(&db_path)?;
    let mut stmt = conn.prepare(
        "SELECT object_hash FROM graveyard WHERE original_path = ? ORDER BY replaced_at DESC LIMIT 1"
    )?;
    let rows = stmt.query_map(params![original_path.to_string_lossy().to_string()], |row| {
        row.get::<_, String>(0)
    })?;
    let mut tags = Vec::new();
    for row in rows {
        tags.push(row?);
    }

    if let Some(hash) = tags.into_iter().next() {
        let prefix = &hash[..2];
        let suffix = &hash[2..];
        let obj_path = objects_dir().join(prefix).join(format!("{}.zst", suffix));
        if obj_path.exists() {
            let compressed = fs::read(&obj_path)?;
            let content = zstd::decode_all(&compressed[..])
                .with_context(|| "zstd decompression failed")?;
            return Ok(Some(content));
        }
    }
    Ok(None)
}

/// Run the reaper: delete expired entries and orphaned objects.
pub fn reap() -> Result<()> {
    let db_path = graveyard_db_path();
    if !db_path.exists() {
        return Ok(());
    }

    let conn = rusqlite::Connection::open(&db_path)?;
    let now = Utc::now().to_rfc3339();

    // Find expired rows
    let mut stmt = conn.prepare(
        "SELECT id, object_hash FROM graveyard WHERE expires_at < ?"
    )?;
    let mut expired = Vec::new();
    for row in stmt.query_map(params![&now], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })? {
        expired.push(row?);
    }

    for (id, hash) in expired {
        conn.execute("DELETE FROM graveyard WHERE id = ?", params![id])?;

        // Check if any other rows still reference this object hash
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM graveyard WHERE object_hash = ?",
            params![&hash],
            |row| row.get(0),
        )?;

        if count == 0 {
            let prefix = &hash[..2];
            let suffix = &hash[2..];
            let obj_path = objects_dir().join(prefix).join(format!("{}.zst", suffix));
            let _ = fs::remove_file(&obj_path);
            // Clean up empty prefix dirs
            let prefix_dir = objects_dir().join(prefix);
            if prefix_dir.exists() {
                let _ = fs::remove_dir(&prefix_dir);
            }
        }
    }

    Ok(())
}

/// Enforce max_size_mb by deleting oldest-first when cap is exceeded.
pub fn enforce_size_cap(max_size_mb: u32) -> Result<()> {
    let db_path = graveyard_db_path();
    if !db_path.exists() {
        return Ok(());
    }
    let conn = rusqlite::Connection::open(&db_path)?;

    let total_compressed: i64 = conn.query_row(
        "SELECT COALESCE(SUM(compressed_bytes), 0) FROM graveyard",
        [],
        |row| row.get(0),
    )?;

    let max_bytes = (max_size_mb as i64) * 1024 * 1024;
    if total_compressed <= max_bytes {
        return Ok(());
    }

    // Delete oldest entries until under cap
    let mut stmt = conn.prepare(
        "SELECT id, object_hash, compressed_bytes FROM graveyard ORDER BY replaced_at ASC"
    )?;
    let mut entries: Vec<(i64, String, i64)> = Vec::new();
    for row in stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })? {
        entries.push(row?);
    }

    let mut current = total_compressed;
    for (id, hash, bytes) in entries {
        if current <= max_bytes {
            break;
        }
        conn.execute("DELETE FROM graveyard WHERE id = ?", params![id])?;
        current -= bytes;

        // Check if any other rows still reference this object hash
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM graveyard WHERE object_hash = ?",
            params![&hash],
            |row| row.get(0),
        )?;

        if count == 0 {
            let prefix = &hash[..2];
            let suffix = &hash[2..];
            let obj_path = objects_dir().join(prefix).join(format!("{}.zst", suffix));
            let _ = fs::remove_file(&obj_path);
        }
    }

    Ok(())
}

fn sha256_string(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
