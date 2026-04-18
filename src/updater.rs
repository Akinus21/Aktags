use serde::Deserialize;

const AKTAGS_REPO: &str = "Akinus21/Aktags";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    pub body: Option<String>,
    #[serde(rename = "draft")]
    pub is_draft: bool,
    #[serde(rename = "prerelease")]
    pub is_prerelease: bool,
}

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    UpToDate,
    Available { version: String, html_url: String, body: String },
    Downloading { version: String, progress: f32 },
    Ready { version: String, path: String },
    Error(String),
}

pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

pub async fn check_for_update_async() -> UpdateStatus {
    use reqwest::Client;
    use tracing::warn;

    let url = format!("https://api.github.com/repos/{}/releases/latest", AKTAGS_REPO);
    let client = Client::new();

    match client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "aktags")
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                return UpdateStatus::Error(format!("HTTP {}", resp.status()));
            }
            match resp.json::<Release>().await {
                Ok(release) => {
                    if release.is_draft || release.is_prerelease {
                        return UpdateStatus::UpToDate;
                    }
                    let latest = release.tag_name.trim_start_matches('v');
                    if latest != CURRENT_VERSION {
                        return UpdateStatus::Available {
                            version: latest.to_string(),
                            html_url: release.html_url,
                            body: release.body.unwrap_or_default(),
                        };
                    }
                    UpdateStatus::UpToDate
                }
                Err(e) => UpdateStatus::Error(e.to_string()),
            }
        }
        Err(e) => {
            warn!("Update check failed: {}", e);
            UpdateStatus::Error(e.to_string())
        }
    }
}

pub fn parse_version(version: &str) -> Option<(u8, u8, u8)> {
    let v = version.trim_start_matches('v');
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}