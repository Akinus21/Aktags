use tracing::{info, warn};

pub async fn check_brew_outdated() -> bool {
    let output = tokio::process::Command::new("brew")
        .arg("outdated")
        .arg("aktags")
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let is_outdated = !stdout.trim().is_empty();
            if is_outdated {
                info!("brew outdated reports update available: {}", stdout.trim());
            } else {
                info!("aktags is up to date via brew");
            }
            is_outdated
        }
        Err(e) => {
            warn!("brew outdated check failed: {}", e);
            false
        }
    }
}

pub async fn brew_upgrade() -> Result<(), String> {
    let output = tokio::process::Command::new("brew")
        .arg("upgrade")
        .arg("aktags")
        .output()
        .await
        .map_err(|e| format!("brew upgrade failed to execute: {}", e))?;

    if output.status.success() {
        info!("brew upgrade successful");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("brew upgrade failed: {}", stderr.trim()))
    }
}

pub fn restart_self() {
    info!("Restarting aktags after brew upgrade...");
    let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("aktags"));
    let err = std::process::Command::new(exe)
        .args(std::env::args().skip(1))
        .spawn();
    match err {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            warn!("Failed to restart: {}. Exiting anyway.", e);
            std::process::exit(1);
        }
    }
}