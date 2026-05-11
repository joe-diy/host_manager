//! Agent bootstrap — exchange a single-use token for permanent NKey credentials.
//!
//! Called on first start when HOSTMGR_AGENT_ID is not set.
//!
//! Flow (ADR-006):
//!   1. POST /api/v1/agents/bootstrap  { token, hostname, os_info }
//!   2. Control plane validates token (5-min TTL, single-use, stored in OpenBao)
//!   3. Response: { agent_id, nkey_seed }
//!   4. Write nkey_seed to ~/.config/hostmgr/nkey.seed (mode 0600)
//!   5. Write agent_id to ~/.config/hostmgr/agent.conf
//!   6. Return agent_id to caller

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize)]
struct BootstrapRequest {
    token: String,
    hostname: String,
}

#[derive(Deserialize)]
struct BootstrapResponse {
    agent_id: String,
    nkey_seed: String,
}

pub async fn run(token: &str) -> Result<String> {
    let control_plane = std::env::var("HOSTMGR_EXTERNAL_URL")
        .unwrap_or_else(|_| "https://localhost:443".to_string());

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    info!(%hostname, "bootstrapping agent with control plane");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let resp: BootstrapResponse = client
        .post(format!("{control_plane}/api/v1/agents/bootstrap"))
        .json(&BootstrapRequest {
            token: token.to_string(),
            hostname,
        })
        .send()
        .await
        .context("bootstrap request failed")?
        .error_for_status()
        .context("bootstrap rejected by control plane")?
        .json()
        .await
        .context("invalid bootstrap response")?;

    // Persist credentials
    let config_dir = dirs_next::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/etc/hostmgr"))
        .join("hostmgr");

    std::fs::create_dir_all(&config_dir)?;

    let seed_path = config_dir.join("nkey.seed");
    write_secret_file(&seed_path, resp.nkey_seed.as_bytes())?;

    let conf_path = config_dir.join("agent.conf");
    let conf = format!("HOSTMGR_AGENT_ID={}\nHOSTMGR_NATS_NKEY_SEED_REF={}\n",
        resp.agent_id,
        seed_path.display());
    std::fs::write(&conf_path, conf)?;

    info!(agent_id = %resp.agent_id, config = %conf_path.display(), "bootstrap complete");
    Ok(resp.agent_id)
}

/// Write `data` to `path` with mode 0600 (owner read/write only).
fn write_secret_file(path: &std::path::Path, data: &[u8]) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    std::io::Write::write_all(&mut f, data)?;
    Ok(())
}
