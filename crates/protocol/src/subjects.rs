//! NATS subject constants.
//!
//! Subjects follow the pattern defined in CLAUDE.md and ADR-005.
//! Use [`agent_subject`] and [`endpoint_subject`] helpers to build
//! per-entity subjects from a base constant.

// ── Discovery ─────────────────────────────────────────────────────────────────

/// Published by the API Gateway to begin a discovery run.
pub const DISCOVERY_START: &str = "discovery.start";

/// Published by the Discovery Orchestrator when a run completes.
pub const DISCOVERY_COMPLETE: &str = "discovery.complete";

/// Published by the Discovery Orchestrator for each discovered endpoint.
pub const DISCOVERY_ENDPOINT_FOUND: &str = "discovery.endpoint.found";

// ── Identification ────────────────────────────────────────────────────────────

/// Published by the Endpoint Manager to trigger identification.
/// Append the endpoint ID: `endpoint.{id}.identify`
pub const ENDPOINT_IDENTIFY_BASE: &str = "endpoint";
pub const ENDPOINT_IDENTIFY_SUFFIX: &str = "identify";

/// Published by the Identifier when identification completes.
/// Append the endpoint ID: `endpoint.{id}.identified`
pub const ENDPOINT_IDENTIFIED_SUFFIX: &str = "identified";

// ── Agent commands ────────────────────────────────────────────────────────────

pub const AGENT_CMD_EXEC: &str    = "exec";
pub const AGENT_CMD_INSTALL: &str = "install";
pub const AGENT_CMD_CONFIG: &str  = "config";
pub const AGENT_CMD_RESTART: &str = "restart";
pub const AGENT_CMD_UPDATE: &str  = "update";

// ── Agent status ──────────────────────────────────────────────────────────────

pub const AGENT_STATUS_HEARTBEAT: &str = "heartbeat";
pub const AGENT_STATUS_RESULT: &str    = "result";
pub const AGENT_STATUS_METRICS: &str   = "metrics";

// ── Agent logs ────────────────────────────────────────────────────────────────

pub const AGENT_LOGS_STREAM: &str = "stream";

// ── Agent lifecycle ───────────────────────────────────────────────────────────

pub const AGENT_LIFECYCLE_CONNECTED:        &str = "connected";
pub const AGENT_LIFECYCLE_DISCONNECTED:     &str = "disconnected";
pub const AGENT_LIFECYCLE_UPDATE_AVAILABLE: &str = "update_available";

// ── Broadcast ─────────────────────────────────────────────────────────────────

pub const AGENT_BROADCAST_PING:   &str = "agent.broadcast.cmd.ping";
pub const AGENT_BROADCAST_UPDATE: &str = "agent.broadcast.cmd.update";

// ── NATS KV bucket names ──────────────────────────────────────────────────────

pub const KV_BUCKET_ENDPOINT_STATE: &str = "ENDPOINT_STATE";
pub const KV_STREAM_AGENT_CMDS: &str     = "AGENT_CMDS";

// ── Subject builder helpers ───────────────────────────────────────────────────

/// Build `agent.{endpoint_id}.cmd.{cmd_type}`
pub fn agent_cmd_subject(endpoint_id: &str, cmd_type: &str) -> String {
    format!("agent.{endpoint_id}.cmd.{cmd_type}")
}

/// Build `agent.{endpoint_id}.status.{status_type}`
pub fn agent_status_subject(endpoint_id: &str, status_type: &str) -> String {
    format!("agent.{endpoint_id}.status.{status_type}")
}

/// Build `agent.{endpoint_id}.logs.stream`
pub fn agent_logs_subject(endpoint_id: &str) -> String {
    format!("agent.{endpoint_id}.logs.stream")
}

/// Build `agent.{endpoint_id}.lifecycle.{event}`
pub fn agent_lifecycle_subject(endpoint_id: &str, event: &str) -> String {
    format!("agent.{endpoint_id}.lifecycle.{event}")
}

/// Build `endpoint.{endpoint_id}.identify`
pub fn endpoint_identify_subject(endpoint_id: &str) -> String {
    format!("endpoint.{endpoint_id}.identify")
}

/// Build `endpoint.{endpoint_id}.identified`
pub fn endpoint_identified_subject(endpoint_id: &str) -> String {
    format!("endpoint.{endpoint_id}.identified")
}

/// Build the NATS KV key for endpoint status: `{endpoint_id}.status`
pub fn kv_status_key(endpoint_id: &str) -> String {
    format!("{endpoint_id}.status")
}

/// Build the NATS KV key for endpoint heartbeat: `{endpoint_id}.heartbeat`
pub fn kv_heartbeat_key(endpoint_id: &str) -> String {
    format!("{endpoint_id}.heartbeat")
}

/// Build the NATS KV key for in-flight command: `{endpoint_id}.cmd.{cmd_id}`
pub fn kv_command_key(endpoint_id: &str, cmd_id: &str) -> String {
    format!("{endpoint_id}.cmd.{cmd_id}")
}
