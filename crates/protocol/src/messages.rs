//! Message envelope types published over NATS.
//!
//! Every message over the bus is wrapped in an envelope that carries a
//! `message_id` for idempotency and `issued_at` for ordering/debugging.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use hostmgr_types::{DiscoveryResult, IdentificationResult};
use hostmgr_types::agent::Heartbeat;

// ── Generic envelope ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub message_id: String,
    pub issued_at:  DateTime<Utc>,
    pub payload:    T,
}

impl<T> Envelope<T> {
    pub fn new(payload: T) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            issued_at:  Utc::now(),
            payload,
        }
    }
}

// ── Discovery messages ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryStartPayload {
    /// Optional subnet override, e.g. `"192.168.1.0/24"`.
    /// If `None`, all configured subnets are scanned.
    pub subnet:     Option<String>,
    pub requested_by: String, // JWT sub of requesting operator
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryCompletePayload {
    pub run_id:            String,
    pub endpoints_found:   u32,
    pub endpoints_new:     u32,
    pub duration_ms:       u64,
}

pub type DiscoveryStartMessage    = Envelope<DiscoveryStartPayload>;
pub type DiscoveryCompleteMessage = Envelope<DiscoveryCompletePayload>;
pub type EndpointFoundMessage     = Envelope<DiscoveryResult>;

// ── Identification messages ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifyPayload {
    pub endpoint_id: String,
    pub primary_ip:  String,
    pub requested_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedPayload {
    pub endpoint_id: String,
    pub result:      IdentificationResult,
}

pub type IdentifyMessage    = Envelope<IdentifyPayload>;
pub type IdentifiedMessage  = Envelope<IdentifiedPayload>;

// ── Agent command messages ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandPayload {
    Exec {
        command:           String,
        timeout_seconds:   u32,
        working_directory: Option<String>,
    },
    Install {
        package: String,
        version: Option<String>,
    },
    Config {
        path:    String,
        content: String,
    },
    Restart {
        service: String,
    },
    Update {
        version: String,
    },
    CheckUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandMessage {
    pub message_id:  String,
    pub command_id:  String,
    pub issued_by:   String,
    pub issued_at:   DateTime<Utc>,
    pub expires_at:  DateTime<Utc>,
    pub payload:     CommandPayload,
}

// ── Agent result messages ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus { Success, Failure, Timeout }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResultPayload {
    pub command_id:   String,
    pub endpoint_id:  String,
    pub reported_at:  DateTime<Utc>,
    pub status:       CommandStatus,
    pub exit_code:    Option<i32>,
    pub stdout:       Option<String>,
    pub stderr:       Option<String>,
    pub duration_ms:  u64,
}

pub type CommandResultMessage = Envelope<CommandResultPayload>;
pub type HeartbeatMessage     = Envelope<Heartbeat>;
