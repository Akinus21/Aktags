use anyhow::Result;
use std::path::PathBuf;

const DESKTOP_FILE: &str = r#"[Desktop Entry]
Version=1.0
Name=AkTags
Comment=AI-powered tag-based file browser
Exec={binary} --gui %U
Icon=folder
Terminal=false
Type=Application
Categories=Utility;FileManager;
MimeType=inode/directory;
StartupNotify=true
StartupWMClass=aktags
"#;

fn desktop_file_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("applications/aktags.desktop")
}

fn mimeapps_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("applications/mimeapps.list")
}

fn get_binary_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "aktags".to_string())
}

pub fn set_as_default_file_manager() -> Result<()> {
    let binary = get_binary_path();
    let content = DESKTOP_FILE.replace("{binary}", &binary);

    let desktop_path = desktop_file_path();
    if let Some(parent) = desktop_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&desktop_path, content)?;

    let apps_path = mimeapps_path();
    let mut entries: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    entries.insert(
        "inode/directory".to_string(),
        vec!["aktags.desktop".to_string()],
    );

    if apps_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&apps_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('[') && line.ends_with(']') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    if key == "inode/directory" {
                        let existing: Vec<String> = value
                            .split(';')
                            .filter(|s| !s.is_empty())
                            .map(String::from)
                            .collect();
                        let mut combined = existing;
                        if !combined.contains(&"aktags.desktop".to_string()) {
                            combined.insert(0, "aktags.desktop".to_string());
                        }
                        entries.insert(key.to_string(), combined);
                    }
                }
            }
        }
    }

    let mut output = String::new();
    output.push_str("[Default Applications]\n");
    for (mime, handlers) in &entries {
        output.push_str(&format!("{}={}\n", mime, handlers.join(";")));
    }

    std::fs::write(&apps_path, output)?;
    Ok(())
}

pub fn unset_as_default_file_manager() -> Result<()> {
    let desktop_path = desktop_file_path();
    if desktop_path.exists() {
        std::fs::remove_file(&desktop_path)?;
    }

    let apps_path = mimeapps_path();
    if apps_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&apps_path) {
            let mut lines: Vec<String> = content.lines().map(String::from).collect();
            lines.retain(|line| {
                let line = line.trim();
                !line.starts_with("inode/directory=")
                    || !line.contains("aktags.desktop")
            });
            std::fs::write(&apps_path, lines.join("\n"))?;
        }
    }
    Ok(())
}

pub fn is_default_file_manager() -> bool {
    desktop_file_path().exists() && mimeapps_path().exists()
}
