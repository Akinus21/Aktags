use anyhow::{Context, Result};
use tracing::{error, info, warn};
use std::path::PathBuf;

use crate::config::CloudConfig;
use crate::db::DbPool;

pub mod client;
pub mod discovery;
pub mod identity;

/// Run a full sync cycle against AKCloud.
pub async fn run_sync(config: &CloudConfig, pool: &DbPool, identity: &crate::sync::identity::Identity, watch_dirs: &[PathBuf]) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }
    let base = config.url.trim_end_matches('/');
    let api_key = &config.api_key;

    // Use first watch directory as sync root
    let sync_root = watch_dirs.first()
        .cloned()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Documents"));

    let http = client::new_client(api_key)?;

    // 1. HEALTH CHECK
    info!("[sync] health check {} ...", base);
    match http.get(format!("{}/health", base)).send().await {
        Ok(resp) if resp.status().is_success() => {
            info!("[sync] AKCloud reachable");
        }
        Ok(resp) => {
            let status = resp.status();
            warn!("[sync] AKCloud health check failed: HTTP {}", status);
            return Err(anyhow::anyhow!("AKCloud health check failed: HTTP {}", status));
        }
        Err(e) => {
            warn!("[sync] AKCloud unreachable: {}", e);
            return Err(anyhow::anyhow!("AKCloud unreachable: {}", e));
        }
    }

    // 2. FETCH SERVER MANIFEST
    info!("[sync] fetching server manifest ...");
    let manifest_resp = http
        .get(format!("{}/api/sync/manifest", base))
        .send()
        .await
        .context("Fetching server manifest")?;
    if !manifest_resp.status().is_success() {
        let status = manifest_resp.status();
        let body = manifest_resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Server manifest endpoint returned HTTP {}: {}", status, body
        ));
    }
    let server_manifest: Vec<client::ManifestEntry> = manifest_resp
        .json()
        .await
        .context("Parsing server manifest")?;

    // 3. BUILD LOCAL MANIFEST
    info!("[sync] building local manifest ...");
    let local_manifest = client::build_local_manifest(pool, &sync_root).await?;

    // 4. DIFF — match by filename (strip directory prefixes from server paths)
    let mut uploads: Vec<client::ManifestEntry> = vec![];
    let mut downloads: Vec<client::ManifestEntry> = vec![];
    let mut conflicts: Vec<(client::ManifestEntry, client::ManifestEntry)> = vec![];
    let mut delete_paths: Vec<String> = vec![];

    fn file_name_of(entry: &client::ManifestEntry) -> String {
        std::path::Path::new(&entry.path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| entry.path.clone())
    }

    for entry in &server_manifest {
        let s_name = file_name_of(entry);
        match local_manifest.iter().find(|e| file_name_of(e) == s_name) {
            None => downloads.push(entry.clone()),
            Some(local) if local.hash == entry.hash => {}
            Some(local) => conflicts.push((local.clone(), entry.clone())),
        }
    }
    for entry in &local_manifest {
        let l_name = file_name_of(entry);
        match server_manifest.iter().find(|e| file_name_of(e) == l_name) {
            Some(_) => {}
            None => uploads.push(entry.clone()),
        }
    }

    // Check for locally deleted files (soft-deleted records)
    {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT path FROM files WHERE deleted_at IS NOT NULL"
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for r in rows {
            if let Ok(path) = r {
                // Skip if already scheduled for upload (upload wins)
                let relative = std::path::Path::new(&path)
                    .strip_prefix(&sync_root)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| path.clone());
                if !uploads.iter().any(|e| e.path == relative) {
                    delete_paths.push(path);
                }
            }
        }
    }

    info!(
        "[sync] diff result: {} uploads, {} downloads, {} conflicts, {} deletes",
        uploads.len(),
        downloads.len(),
        conflicts.len(),
        delete_paths.len()
    );

    // 5. TRANSFER — UPLOADS
    for entry in uploads {
        // Compute relative path for server, keep absolute path for local DB operations
        let entry_path = std::path::Path::new(&entry.path);
        let relative_path = entry_path
            .strip_prefix(&sync_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| entry.path.clone());
        let local_disk = sync_root.join(&entry.path).to_string_lossy().to_string();
        let absolute_path = entry.path.clone(); // Keep absolute for mark_synced
        match client::upload_file(&http, base, &relative_path, &local_disk).await {
            Ok(()) => {
                info!("[sync] uploaded {}", entry.path);
                let pool = pool.clone();
                tokio::task::block_in_place(|| {
                    let _ = crate::db::mark_synced(&pool, &absolute_path, &entry.hash);
                });
            }
            Err(e) => {
                error!("[sync] upload failed for {}: {}", entry.path, e);
            }
        }
    }

    // 5. TRANSFER — DOWNLOADS
    for entry in downloads {
        // Strip directory prefix from server path for both download URL and local destination
        let file_name = std::path::Path::new(&entry.path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| entry.path.clone());
        let local_disk = sync_root.join(&file_name).to_string_lossy().to_string();
        match client::download_file(&http, base, &file_name, &local_disk).await {
            Ok(()) => {
                info!("[sync] downloaded {}", entry.path);
                let pool = pool.clone();
                tokio::task::block_in_place(|| {
                    let _ = crate::db::mark_synced(&pool, &local_disk, &entry.hash);
                });
            }
            Err(e) => {
                error!("[sync] download failed for {}: {}", entry.path, e);
            }
        }
    }

    // 5. TRANSFER — CONFLICTS (newer mtime wins)
    for (local, server) in conflicts {
        let mtime_local = local.mtime;
        let mtime_server = server.mtime;
        let path = &local.path;

        if mtime_local > mtime_server {
            // Local wins → upload
            let local_disk = shellexpand::tilde(&local.path).to_string();
            let pool = pool.clone();
            match client::upload_file(&http, base, &local.path, &local_disk).await {
                Ok(()) => {
                    info!("[sync] uploaded {} (local newer)", path);
                    let hash = local.hash.clone();
                    let path = local.path.clone();
                    tokio::task::block_in_place(|| {
                        let _ = crate::db::mark_synced(&pool, &path, &hash);
                    });
                }
                Err(e) => {
                    error!("[sync] upload failed for {}: {}", path, e);
                }
            }
        } else {
            // Server wins → download (before overwriting local file, entomb local copy)
            // Strip directory prefix from server path for server URL (may differ from local storage)
            let file_name = std::path::Path::new(&server.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| server.path.clone());
            let local_disk = sync_root.join(&local.path).to_string_lossy().to_string();
            let pool = pool.clone();
            // Entomb local losing copy
            let _ = crate::graveyard::entomb(
                std::path::Path::new(&local_disk),
                &local.hash,
                &server.hash,
                Some(&identity.public_key_hex),
                None,
                None,
                crate::config::GraveyardConfig::default().ttl_days,
            );
            match client::download_file(&http, base, &file_name, &local_disk).await {
                Ok(()) => {
                    info!("[sync] downloaded {} (server newer)", path);
                    let hash = server.hash.clone();
                    let path = local.path.clone();
                    tokio::task::block_in_place(|| {
                        let _ = crate::db::mark_synced(&pool, &path, &hash);
                    });
                }
                Err(e) => {
                    error!("[sync] download failed for {}: {}", path, e);
                }
            }
        }
    }

    // 6. PROPAGATE DELETES — delete files that were removed locally
    for abs_path in delete_paths {
        // Try stripping each watch_dir to get relative path for server
        let rel_path = watch_dirs.iter()
            .find_map(|wd| std::path::Path::new(&abs_path).strip_prefix(wd).ok())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| abs_path.clone());

        info!("[sync] deleting file on server: {}", rel_path);
        match client::delete_file(&http, base, &rel_path).await {
            Ok(()) => {
                info!("[sync] deleted {}", rel_path);
                // Hard-delete local record so it doesn't re-delete next cycle
                let pool = pool.clone();
                let abs_path = abs_path.clone();
                tokio::task::block_in_place(|| {
                    let conn = pool.get().ok();
                    if let Some(conn) = conn {
                        let _ = conn.execute("DELETE FROM files WHERE path = ?", [&abs_path]);
                    }
                });
            }
            Err(e) => {
                // 404 = file already gone on server, treat as success
                if e.to_string().contains("404") {
                    info!("[sync] file already deleted on server: {}", rel_path);
                    let pool = pool.clone();
                    let abs_path = abs_path.clone();
                    tokio::task::block_in_place(|| {
                        let conn = pool.get().ok();
                        if let Some(conn) = conn {
                            let _ = conn.execute("DELETE FROM files WHERE path = ?", [&abs_path]);
                        }
                    });
                } else {
                    error!("[sync] delete failed for {}: {}", abs_path, e);
                }
            }
        }
    }

    // 7. TAG SYNC
    info!("[sync] syncing tags ...");
    let all_files = client::list_all_files(&http, base, 10000, 0
    ).await?;
    for file in all_files {
        let local_id = {
            let pool = pool.clone();
            let abs_path = sync_root.join(&file.path).to_string_lossy().to_string();
            tokio::task::block_in_place(|| {
                let conn = pool.get().ok()?;
                let id: Option<i64> = conn
                    .query_row("SELECT id FROM files WHERE path = ?", [abs_path], |r| r.get(0))
                    .ok();
                id
            })
        };
        if let Some(id) = local_id {
            let local_tags = {
                let pool = pool.clone();
                tokio::task::block_in_place(|| {
                    crate::db::get_tags_for_file(&pool, id).unwrap_or_default()
                })
            };

            // Get server tags
            let server_tags = client::get_file_tags(&http, base, file.id
            ).await.unwrap_or_default();

            // Merge (union) both directions
            // Add tags present on server but missing locally
            for tag in &server_tags {
                if !local_tags.contains(tag) {
                    let pool = pool.clone();
                    let tag = tag.clone();
                    tokio::task::block_in_place(|| {
                        let _ = crate::db::tag_file(&pool, id, &tag);
                    });
                    info!("[sync] added local tag '{}' for {}", tag, file.path);
                }
            }

            // Add tags present locally but missing on server
            for tag in &local_tags {
                if !server_tags.contains(tag) {
                    match client::add_file_tag(&http, base, file.id, tag
                    ).await {
                        Ok(()) => {
                            info!("[sync] added server tag '{}' for {}", tag, file.path);
                        }
                        Err(e) => {
                            warn!("[sync] failed to add server tag '{}': {}", tag, e);
                        }
                    }
                }
            }
        }
    }

    // 7. STATS REFRESH (handled by caller UI)
    info!("[sync] cycle complete");
    Ok(())
}
