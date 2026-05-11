//! Agent self-update lifecycle (ADR-010).
//!
//! State machine:
//!   IDLE → CHECKING → DOWNLOADING → VERIFYING → STAGING → RESTARTING
//!        → RECONNECTION_CHECK → IDLE (success) or ROLLING_BACK (failure)
//!
//! Trigger modes (HOSTMGR_UPDATE_TRIGGER env var):
//!   "manual"    — update only when an Update command arrives (default)
//!   "automatic" — poll for updates on a configurable interval

use anyhow::Result;
use tracing::{info, warn};

use crate::transport::Connection;

pub async fn run(conn: Connection, agent_id: String, trigger: String) -> Result<()> {
    match trigger.as_str() {
        "automatic" => run_automatic(conn, agent_id).await,
        _ => {
            // In manual mode this task is a no-op — updates are driven by the
            // command executor receiving a CommandPayload::Update message.
            info!(%agent_id, "update trigger = manual; waiting for operator command");
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        }
    }
}

async fn run_automatic(conn: Connection, agent_id: String) -> Result<()> {
    let check_interval = std::time::Duration::from_secs(
        std::env::var("HOSTMGR_UPDATE_CHECK_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600u64),
    );

    let mut interval = tokio::time::interval(check_interval);
    loop {
        interval.tick().await;
        if let Err(e) = check_and_update(&conn, &agent_id).await {
            warn!(%agent_id, error = %e, "update check failed");
        }
    }
}

async fn check_and_update(_conn: &Connection, agent_id: &str) -> Result<()> {
    // TODO (ADR-010 full implementation):
    //
    // CHECKING:
    //   POST /api/v1/agents/{id}/update/check
    //   Response: { available: bool, version: String, download_url: String, sha256: String }
    //
    // DOWNLOADING:
    //   GET download_url → stream to agent.new (temp path)
    //
    // VERIFYING:
    //   sha256(agent.new) == expected_sha256
    //
    // STAGING:
    //   cp agent (current binary) → agent.prev (rollback)
    //   chmod +x agent.new
    //
    // RESTARTING:
    //   systemd: systemctl restart hostmgr-agent
    //   docker:  POST http+unix:///run/hostmgr/update.sock /restart
    //   foreground: exec(agent.new, args)
    //
    // RECONNECTION_CHECK:
    //   Wait up to 90s for NATS reconnect
    //   On success: rm agent.prev, publish agent.{id}.updated
    //   On failure: cp agent.prev → agent, restart → ROLLING_BACK

    info!(%agent_id, "update check (stub — not yet implemented)");
    Ok(())
}
