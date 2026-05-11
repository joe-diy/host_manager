//! Host Manager Agent
//!
//! Runs on managed endpoints (Linux servers, Raspberry Pis, VMs).
//! Responsibilities:
//!   - Negotiate transport: NATS WSS on port 443 → HTTPS polling fallback (ADR-005)
//!   - Send heartbeats every 30s to `agent.{id}.heartbeat`
//!   - Subscribe to `agent.cmd.{id}.>` on JetStream for inbound commands
//!   - Execute commands and publish results to `agent.{id}.cmd_result.{cmd_id}`
//!   - Report status transitions to `agent.{id}.status`
//!   - Manage own update lifecycle (ADR-010)
//!
//! Configuration (all via env vars per ADR-009):
//!   HOSTMGR_AGENT_ID           — assigned endpoint ID (ep-{uuid})
//!   HOSTMGR_EXTERNAL_URL       — control plane base URL (WSS or HTTPS)
//!   HOSTMGR_NATS_URL           — NATS WSS URL (wss://host:443)
//!   HOSTMGR_NATS_NKEY_SEED_REF — path to NKey seed file (mode 0600)
//!   HOSTMGR_BOOTSTRAP_TOKEN    — single-use bootstrap token (cleared after use)
//!   HOSTMGR_UPDATE_TRIGGER     — "manual" (default) | "automatic"
//!   RUST_LOG                   — log level filter

use anyhow::Result;
use clap::Parser;
use tracing::info;

mod bootstrap;
mod commands;
mod heartbeat;
mod transport;
mod update;

/// Host Manager Agent daemon.
#[derive(Parser, Debug)]
#[command(name = "hostmgr-agent", version, about)]
struct Args {
    /// Override NATS WebSocket URL (wss://host:443).
    #[arg(long, env = "HOSTMGR_NATS_URL")]
    nats_url: Option<String>,

    /// Agent endpoint ID (assigned during bootstrap).
    #[arg(long, env = "HOSTMGR_AGENT_ID")]
    agent_id: Option<String>,

    /// Path to NKey seed file.
    #[arg(long, env = "HOSTMGR_NATS_NKEY_SEED_REF")]
    nkey_seed_ref: Option<String>,

    /// Bootstrap token (single-use, 5-min TTL).
    #[arg(long, env = "HOSTMGR_BOOTSTRAP_TOKEN")]
    bootstrap_token: Option<String>,

    /// Update trigger mode.
    #[arg(long, env = "HOSTMGR_UPDATE_TRIGGER", default_value = "manual")]
    update_trigger: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,hostmgr_agent=debug".into()),
        )
        .init();

    let args = Args::parse();
    info!(
        version = env!("CARGO_PKG_VERSION"),
        "hostmgr-agent starting"
    );

    // --- Bootstrap phase ---------------------------------------------------
    // If no agent_id is set, we have not yet registered with the control plane.
    // Use the bootstrap token to obtain NKey credentials and an agent_id.
    let agent_id = if let Some(id) = args.agent_id.clone() {
        id
    } else {
        let token = args.bootstrap_token.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "no HOSTMGR_AGENT_ID set and no HOSTMGR_BOOTSTRAP_TOKEN provided; \
                 cannot bootstrap. Set one of these env vars."
            )
        })?;
        bootstrap::run(token).await?
    };

    info!(%agent_id, "agent identity confirmed");

    // --- Transport negotiation (ADR-005) -----------------------------------
    // Attempt NATS WSS on port 443; fall back to HTTPS polling if that fails.
    let conn = transport::connect(&args, &agent_id).await?;

    // --- Main event loop ---------------------------------------------------
    // Spawn tasks:
    //   1. Heartbeat sender (30s interval → agent.{id}.heartbeat)
    //   2. Command subscriber (JetStream AGENT_CMDS → agent.cmd.{id}.>)
    //   3. Update watcher (if update_trigger == "automatic")
    tokio::try_join!(
        heartbeat::run(conn.clone(), agent_id.clone()),
        commands::run(conn.clone(), agent_id.clone()),
        update::run(conn.clone(), agent_id.clone(), args.update_trigger.clone()),
    )?;

    Ok(())
}
