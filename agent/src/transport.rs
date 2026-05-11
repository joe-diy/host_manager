//! Transport negotiation — NATS WSS (primary) → HTTPS polling (fallback).
//!
//! Per ADR-005:
//!   1. Attempt NATS WebSocket over TLS 1.3 on port 443
//!   2. On failure (connect timeout 10s, 3 retries with exponential backoff),
//!      fall back to HTTPS polling mode
//!   3. Once connected, the same `Connection` type is returned regardless of mode

use anyhow::Result;
use async_nats::Client;
use tracing::{info, warn};

use crate::Args;

/// Opaque connection handle — hides which transport is active.
#[derive(Clone)]
pub struct Connection {
    pub nats: Option<Client>,
    pub mode: TransportMode,
    pub agent_id: String,
    pub control_plane_url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TransportMode {
    /// NATS WebSocket over TLS 1.3 on port 443.
    NatsWss,
    /// HTTPS polling fallback (anax-model).
    HttpsPolling,
}

/// Attempt NATS WSS connection. Returns the active `Connection`.
pub async fn connect(args: &Args, agent_id: &str) -> Result<Connection> {
    let nats_url = args
        .nats_url
        .clone()
        .unwrap_or_else(|| "wss://localhost:443".to_string());

    let control_plane_url = std::env::var("HOSTMGR_EXTERNAL_URL")
        .unwrap_or_else(|_| "https://localhost:443".to_string());

    info!(url = %nats_url, "attempting NATS WSS connection");

    match try_nats_connect(&nats_url, args).await {
        Ok(client) => {
            info!("NATS WSS connection established — primary transport active");
            Ok(Connection {
                nats: Some(client),
                mode: TransportMode::NatsWss,
                agent_id: agent_id.to_string(),
                control_plane_url,
            })
        }
        Err(e) => {
            warn!(error = %e, "NATS WSS failed; falling back to HTTPS polling");
            Ok(Connection {
                nats: None,
                mode: TransportMode::HttpsPolling,
                agent_id: agent_id.to_string(),
                control_plane_url,
            })
        }
    }
}

async fn try_nats_connect(url: &str, args: &Args) -> Result<Client> {
    // TODO: load NKey seed from file path in args.nkey_seed_ref
    //   let seed = std::fs::read_to_string(&seed_path)?;
    //   let kp   = nkeys::KeyPair::from_seed(&seed.trim())?;
    //   Build async_nats::ConnectOptions with .nkey(kp)

    // Retry up to 3 times with exponential backoff (1s, 2s, 4s)
    let mut backoff = tokio::time::Duration::from_secs(1);
    for attempt in 1..=3u8 {
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            async_nats::connect(url),
        )
        .await
        {
            Ok(Ok(client)) => return Ok(client),
            Ok(Err(e)) => {
                warn!(attempt, error = %e, backoff_ms = backoff.as_millis(), "NATS connect failed");
            }
            Err(_) => {
                warn!(attempt, "NATS connect timed out after 10s");
            }
        }
        if attempt < 3 {
            tokio::time::sleep(backoff).await;
            backoff *= 2;
        }
    }

    let _ = args; // suppress unused warning until NKey wiring is complete
    anyhow::bail!("NATS WSS connection failed after 3 attempts")
}
