use anyhow::{anyhow, Context, Result};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName};
use std::str::FromStr;
use serde::{Deserialize, Serialize};

use crate::db::DbPool;

/// Generic reqwest client with Bearer token + X-Api-Key fallback.
pub fn new_client(api_key: &str) -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    if !api_key.is_empty() {
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", api_key).parse().context("Parsing auth header")?,
        );
        headers.insert(
            HeaderName::from_str("X-Api-Key").unwrap(),
            api_key.parse().context("Parsing X-Api-Key header")?,
        );
    }
    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("Building HTTP client")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: i64,
    pub filename: String,
    pub path: String,
    pub hash: String,
    pub size: i64,
    pub mtime: i64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagRecord {
    pub id: i64,
    pub tag: String,
    pub file_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub hash: String,
    pub mtime: i64,
    pub size: i64,
}

/// Build local manifest from DB.
pub async fn build_local_manifest(pool: &DbPool) -> Result<Vec<ManifestEntry>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT path, file_hash, modified_at, size_bytes FROM files WHERE file_hash IS NOT NULL"
    )?;
    let rows = stmt.query_map([], |row| {
        let path: String = row.get(0)?;
        let hash: String = row.get(1)?;
        let mtime_str: Option<String> = row.get(2)?;
        let size: i64 = row.get(3)?;
        let mtime = mtime_str
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.timestamp())
            .unwrap_or(0);
        Ok(ManifestEntry { path, hash, mtime, size })
    })?;
    let mut entries = Vec::new();
    for r in rows {
        entries.push(r?);
    }
    Ok(entries)
}

/// List all files from server.
pub async fn list_all_files(
    client: &reqwest::Client,
    base: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<FileRecord>> {
    let url = format!("{}/api/files?limit={}&offset={}", base, limit, offset);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("List files failed: HTTP {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await?;
    // Handle both plain array and { files: [...] } shapes
    let files: Vec<FileRecord> = if let Some(arr) = json.get("files") {
        serde_json::from_value(arr.clone())?
    } else {
        serde_json::from_value(json)?
    };
    Ok(files)
}

/// Get tags for a file by server file ID.
pub async fn get_file_tags(
    client: &reqwest::Client,
    base: &str,
    file_id: i64,
) -> Result<Vec<String>> {
    let url = format!("{}/api/file/{}/tags", base, file_id);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("Get file tags failed: HTTP {}", resp.status()));
    }
    let tags: Vec<TagRecord> = resp.json().await?;
    Ok(tags.into_iter().map(|t| t.tag).collect())
}

/// PUT /api/file-tags/:file_id/:tag
pub async fn add_file_tag(
    client: &reqwest::Client,
    base: &str,
    file_id: i64,
    tag: &str,
) -> Result<()> {
    let url = format!("{}/api/file-tags/{}/{}", base, file_id, tag);
    let resp = client.put(&url).send().await?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(anyhow!("Add file tag failed: HTTP {}", resp.status()))
    }
}

/// Upload a single file by path.
pub async fn upload_file(
    client: &reqwest::Client,
    base: &str,
    remote_path: &str,
    local_path: &str,
) -> Result<()> {
    let url = format!("{}/api/sync/files/{}", base, remote_path);
    let body = tokio::fs::read(local_path).await?;
    let resp = client
        .post(&url)
        .body(body)
        .send()
        .await?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!("Upload failed: {}", body))
    }
}

/// Download a single file by path.
pub async fn download_file(
    client: &reqwest::Client,
    base: &str,
    remote_path: &str,
    local_path: &str,
) -> Result<()> {
    let url = format!("{}/api/sync/files/{}", base, remote_path);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("Download failed: HTTP {}", resp.status()));
    }
    let bytes = resp.bytes().await?;
    tokio::fs::write(local_path, &bytes).await?;
    Ok(())
}
