//! Heartbeat task — publishes `agent.{id}.heartbeat` every 30 seconds.
//!
//! Per ADR-005: the agent-coordinator actor monitors NATS KV heartbeat keys
//! with a 120s TTL. If no heartbeat arrives within that window the endpoint
//! is marked Offline (offline detection threshold = 90s; ADR-008).

use anyhow::Result;
use hostmgr_protocol::subjects;
use hostmgr_types::agent::{AgentMetrics, Heartbeat};
use serde_json;
use std::time::Duration;
use tracing::{debug, warn};

use crate::transport::Connection;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

pub async fn run(conn: Connection, agent_id: String) -> Result<()> {
    let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    // Skip the first immediate tick so we don't fire before connection is ready.
    interval.tick().await;

    loop {
        interval.tick().await;

        let hb = Heartbeat {
            endpoint_id:    agent_id.clone(),
            reported_at:    chrono::Utc::now(),
            agent_version:  env!("CARGO_PKG_VERSION").to_string(),
            transport:      hostmgr_types::agent::Transport::Wss,
            uptime_seconds: 0, // TODO: read from /proc/uptime
            metrics:        collect_metrics(),
        };

        let subject = subjects::agent_status_subject(&agent_id, "heartbeat");
        let payload = serde_json::to_vec(&hb)?;

        if let Some(nats) = &conn.nats {
            match nats.publish(subject.clone(), payload.into()).await {
                Ok(_) => debug!(%agent_id, "heartbeat sent"),
                Err(e) => warn!(%agent_id, error = %e, "heartbeat publish failed"),
            }
        } else {
            // TODO: HTTPS polling fallback — POST heartbeat to REST endpoint
            debug!(%agent_id, "heartbeat skipped (HTTPS polling mode not yet implemented)");
        }
    }
}

fn collect_metrics() -> AgentMetrics {
    // TODO: read /proc/loadavg, /proc/meminfo, /proc/diskstats
    AgentMetrics {
        cpu_percent:       0.0,
        memory_used_mb:    0,
        memory_total_mb:   0,
        disk_used_percent: 0.0,
        load_1m:           0.0,
        load_5m:           0.0,
        load_15m:          0.0,
    }
}
