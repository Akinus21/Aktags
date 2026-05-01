use anyhow::{Context, Result};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub private_key: String,
    pub public_key_hex: String,
    pub node_id: String,
}

fn identity_key_path() -> PathBuf {
    crate::config::config_dir().join("identity.key")
}

/// Load existing identity or generate a new Ed25519 keypair.
/// Returns the node_id (public key hex).
pub fn load_or_generate_node_id() -> Result<String> {
    let path = identity_key_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Reading identity key from {}", path.display()))?;
        let identity: Identity = serde_json::from_str(&content)
            .context("Parsing identity.key")?;
        return Ok(identity.node_id);
    }

    // Generate new keypair
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_bytes();
    let public_key_hex = hex::encode(&public_key_bytes);

    let identity = Identity {
        private_key: hex::encode(signing_key.to_bytes()),
        public_key_hex: public_key_hex.clone(),
        node_id: public_key_hex.clone(),
    };

    let json = serde_json::to_string_pretty(&identity)
        .context("Serializing identity")?;
    std::fs::write(&path, json)
        .with_context(|| format!("Writing identity.key to {}", path.display()))?;

    // Set permissions to 600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }

    Ok(public_key_hex)
}

/// Get the identity struct (loads from disk).
pub fn load_identity() -> Result<Identity> {
    let path = identity_key_path();
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Reading identity key from {}", path.display()))?;
    let identity: Identity = serde_json::from_str(&content)
        .context("Parsing identity.key")?;
    Ok(identity)
}
