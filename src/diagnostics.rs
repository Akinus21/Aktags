use anyhow::Result;
use serde::Serialize;
use tracing::warn;

const USER_AGENT: &str = "aktags/diagnostics";

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticsReport {
    pub hostname: String,
    pub version: String,
    pub entries: Vec<LogEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

pub async fn send_report(webhook_url: &str, entries: Vec<LogEntry>) -> Result<()> {
    let report = DiagnosticsReport {
        hostname: std::env::var("HOSTNAME")
            .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
            .unwrap_or_else(|_| "unknown".into()),
        version: crate::updater::current_version().to_string(),
        entries,
    };

    let client = reqwest::Client::new();
    let body = serde_json::to_string(&report)?;

    let resp = client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .header("User-Agent", USER_AGENT)
        .header("X-GitHub-Event", "diagnostics")
        .body(body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("webhook POST failed: HTTP {} - {}", status, text);
    }

    Ok(())
}

pub fn read_recent_log_errors() -> Vec<LogEntry> {
    let log_path = crate::config::config_dir().join("aktags.log");

    if !log_path.exists() {
        return vec![];
    }

    let content = match std::fs::read_to_string(&log_path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read log file: {}", e);
            return vec![];
        }
    };

    let mut entries = Vec::new();

    for line in content.lines().rev().take(50) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let level = if line.contains("ERROR") {
            "ERROR"
        } else if line.contains("WARN") {
            "WARN"
        } else {
            continue;
        };

        entries.push(LogEntry {
            timestamp: extract_timestamp(line).to_string(),
            level: level.to_string(),
            message: line.to_string(),
        });
    }

    entries.reverse();
    entries
}

fn extract_timestamp(line: &str) -> &str {
    let start = line.find('[').unwrap_or(0);
    let end = line.find(']').unwrap_or(0);
    if start < end && start > 0 {
        &line[start + 1..end]
    } else {
        "unknown"
    }
}