use anyhow::{Context, Result};
use tracing::{error, info, warn};

use crate::config::CloudConfig;
use crate::db::DbPool;

pub mod client;
pub mod discovery;
pub mod identity;

/// Run a full sync cycle against AKCloud.
pub async fn run_sync(config: &CloudConfig, pool: &DbPool, identity: &crate::sync::identity::Identity) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }
    let base = config.url.trim_end_matches('/');
    let api_key = &config.api_key;

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
    let local_manifest = client::build_local_manifest(pool).await?;

    // 4. DIFF
    let mut uploads: Vec<client::ManifestEntry> = vec![];
    let mut downloads: Vec<client::ManifestEntry> = vec![];
    let mut conflicts: Vec<(client::ManifestEntry, client::ManifestEntry)> = vec![];

    for entry in &server_manifest {
        match local_manifest.iter().find(|e| e.path == entry.path) {
            None => downloads.push(entry.clone()),
            Some(local) if local.hash == entry.hash => {}
            Some(local) => conflicts.push((local.clone(), entry.clone())),
        }
    }
    for entry in &local_manifest {
        if !server_manifest.iter().any(|e| e.path == entry.path) {
            uploads.push(entry.clone());
        }
    }

    info!(
        "[sync] diff result: {} uploads, {} downloads, {} conflicts",
        uploads.len(),
        downloads.len(),
        conflicts.len()
    );

    // 5. TRANSFER — UPLOADS
    for entry in uploads {
        let local_path = shellexpand::tilde(&entry.path).to_string();
        // Strip to just filename for server upload path
        let path = std::path::Path::new(&entry.path);
        let remote_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&entry.path);
        match client::upload_file(&http, base, remote_name, &local_path).await {
            Ok(()) => {
                info!("[sync] uploaded {}", entry.path);
            }
            Err(e) => {
                error!("[sync] upload failed for {}: {}", entry.path, e);
            }
        }
    }

    // 5. TRANSFER — DOWNLOADS
    for entry in downloads {
        // Use server's path as-is (server stores relative paths in manifest)
        let remote_path = entry.path.as_str();
        // Map to local watch directory for download destination
        let local_path = shellexpand::tilde(&entry.path).to_string();
        match client::download_file(&http, base, remote_path, &local_path).await {
            Ok(()) => {
                info!("[sync] downloaded {}", entry.path);
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
            match client::upload_file(&http, base, &local.path, &local_disk).await {
                Ok(()) => {
                    info!("[sync] uploaded {} (local newer)", path);
                }
                Err(e) => {
                    error!("[sync] upload failed for {}: {}", path, e);
                }
            }
        } else {
            // Server wins → download (before overwriting local file, entomb local copy)
            let local_disk = shellexpand::tilde(&local.path).to_string();
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
            match client::download_file(&http, base, &server.path, &local_disk).await {
                Ok(()) => {
                    info!("[sync] downloaded {} (server newer)", path);
                }
                Err(e) => {
                    error!("[sync] download failed for {}: {}", path, e);
                }
            }
        }
    }

    // 6. TAG SYNC
    info!("[sync] syncing tags ...");
    let all_files = client::list_all_files(&http, base, 10000, 0
    ).await?;
    for file in all_files {
        let local_id = {
            let pool = pool.clone();
            tokio::task::block_in_place(|| {
                let conn = pool.get().ok()?;
                let id: Option<i64> = conn
                    .query_row("SELECT id FROM files WHERE path = ?", [file.path.clone()], |r| r.get(0))
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
