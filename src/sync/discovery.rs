use anyhow::Result;
use std::net::IpAddr;
use tracing::{debug, error, info};

/// Attempt to discover AKCloud on the local network via mDNS.
pub async fn discover_lan_server() -> Option<String> {
    // Use mdns-sd to browse for _akcloud._tcp.local
    let mdns = mdns_sd::ServiceDaemon::new().ok()?;
    let receiver = mdns.browse("_akcloud._tcp").ok()?;

    // Wait up to 3 seconds for a response
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 3 {
        if let Ok(event) = receiver.recv_timeout(std::time::Duration::from_millis(300)) {
            if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
                if let Some(addr) = info.get_addresses().iter().next() {
                    let port = info.get_port();
                    let url = format!("http://{}:{}", addr, port);
                    info!("[discovery] Found AKCloud on LAN: {}", url);
                    return Some(url);
                }
            }
        }
    }
    debug!("[discovery] No AKCloud found on LAN via mDNS");
    None
}

/// Send a heartbeat to the configured AKCloud server to advertise local presence.
pub async fn send_heartbeat(base_url: &str, node_id: &str) -> Result<()> {
    let url = format!("{}/api/heartbeat", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "node_id": node_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
        .send()
        .await?;
    if resp.status().is_success() {
        debug!("[discovery] Heartbeat sent to {}", url);
        Ok(())
    } else {
        Err(anyhow::anyhow!("Heartbeat failed: HTTP {}", resp.status()))
    }
}
