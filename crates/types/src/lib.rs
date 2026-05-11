//! Shared domain types for Host Manager.
//!
//! This crate is `no_std`-compatible (except for `std` feature) and compiles on both
//! native targets and `wasm32-wasip2` (used by WasmCloud actor crates).

pub use endpoint::{Endpoint, EndpointId, EndpointStatus};
pub use discovery::{DiscoveryMethod, DiscoveryResult, NetworkInfo};
pub use identification::{EndpointType, IdentificationResult};
pub use agent::{AgentInfo, AgentStatus, ServiceMode};
pub use credentials::CredentialRef;

// ── Endpoint ─────────────────────────────────────────────────────────────────

pub mod endpoint {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use chrono::{DateTime, Utc};

    /// Stable, immutable identifier assigned at first discovery.
    /// Format: `ep-{uuid_v4}`
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct EndpointId(pub String);

    impl EndpointId {
        pub fn new() -> Self {
            Self(format!("ep-{}", uuid::Uuid::new_v4()))
        }

        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    impl Default for EndpointId {
        fn default() -> Self {
            Self::new()
        }
    }

    impl std::fmt::Display for EndpointId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    /// Lifecycle state of an endpoint. Transitions are owned by specific actors
    /// (see ADR-008 for the full state machine).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum EndpointStatus {
        #[default]
        Discovered,
        Identified,
        AgentDeploying,
        Managed,
        Offline,
        Degraded,
        Decommissioned,
    }

    impl std::fmt::Display for EndpointStatus {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let s = match self {
                Self::Discovered     => "DISCOVERED",
                Self::Identified     => "IDENTIFIED",
                Self::AgentDeploying => "AGENT_DEPLOYING",
                Self::Managed        => "MANAGED",
                Self::Offline        => "OFFLINE",
                Self::Degraded       => "DEGRADED",
                Self::Decommissioned => "DECOMMISSIONED",
            };
            write!(f, "{s}")
        }
    }

    /// Core endpoint record — stored in OpenBao at `secret/endpoints/{id}/core`.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Endpoint {
        pub schema_version: u32,
        pub id:             EndpointId,
        pub display_name:   String,
        pub status:         EndpointStatus,
        pub created_at:     DateTime<Utc>,
        pub updated_at:     DateTime<Utc>,
        pub discovered_at:  DateTime<Utc>,
        pub identified_at:  Option<DateTime<Utc>>,
        pub agent_deployed_at: Option<DateTime<Utc>>,
        pub decommissioned_at: Option<DateTime<Utc>>,
        pub tags:           HashMap<String, String>,
        pub notes:          Option<String>,
    }

    impl Endpoint {
        pub fn new(display_name: impl Into<String>) -> Self {
            let now = Utc::now();
            Self {
                schema_version: 1,
                id:             EndpointId::new(),
                display_name:   display_name.into(),
                status:         EndpointStatus::Discovered,
                created_at:     now,
                updated_at:     now,
                discovered_at:  now,
                identified_at:  None,
                agent_deployed_at: None,
                decommissioned_at: None,
                tags:           HashMap::new(),
                notes:          None,
            }
        }
    }
}

// ── Discovery ─────────────────────────────────────────────────────────────────

pub mod discovery {
    use serde::{Deserialize, Serialize};
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum DiscoveryMethod {
        Arp,
        Mdns,
        Manual,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IpEntry {
        pub address:   String,
        #[serde(rename = "type")]
        pub ip_type:   String, // "ipv4" | "ipv6"
        pub interface: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MacEntry {
        pub address:   String,
        pub interface: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HostnameEntry {
        pub name:   String,
        pub source: String, // "mdns" | "dns" | "manual"
    }

    /// Network information stored in OpenBao at `secret/endpoints/{id}/network`.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NetworkInfo {
        pub schema_version:   u32,
        pub primary_ip:       String,
        pub ip_addresses:     Vec<IpEntry>,
        pub mac_addresses:    Vec<MacEntry>,
        pub hostnames:        Vec<HostnameEntry>,
        pub discovery_method: DiscoveryMethod,
        pub last_seen_ip:     String,
        pub last_seen_at:     DateTime<Utc>,
        pub network_segment:  Option<String>,
    }

    /// Result produced by the discovery capability provider.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DiscoveryResult {
        pub ip:           String,
        pub mac:          Option<String>,
        pub hostnames:    Vec<String>,
        pub method:       DiscoveryMethod,
        pub discovered_at: DateTime<Utc>,
    }
}

// ── Identification ────────────────────────────────────────────────────────────

pub mod identification {
    use serde::{Deserialize, Serialize};
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum EndpointType {
        LinuxServer,
        RaspberryPi,
        Vm,
        KubernetesNode,
        Macos,
        Unknown,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum IdentificationConfidence {
        High,
        Medium,
        Low,
    }

    /// Identification data stored in OpenBao at `secret/endpoints/{id}/identity`.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IdentificationResult {
        pub schema_version:          u32,
        pub endpoint_type:           EndpointType,
        pub subtype:                 Option<String>,
        pub os:                      String,
        pub os_version:              String,
        pub os_codename:             Option<String>,
        pub kernel:                  Option<String>,
        pub architecture:            String,
        pub cpu_model:               Option<String>,
        pub cpu_cores:               Option<u32>,
        pub memory_mb:               Option<u64>,
        pub hostname:                String,
        pub ssh_host_fingerprint:    Option<String>,
        pub identification_method:   String,
        pub identified_at:           DateTime<Utc>,
        pub identification_confidence: IdentificationConfidence,
    }
}

// ── Agent ─────────────────────────────────────────────────────────────────────

pub mod agent {
    use serde::{Deserialize, Serialize};
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum AgentStatus {
        NotDeployed,
        Deployed,
        Connected,
        Offline,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ServiceMode {
        Systemd,
        Docker,
        Podman,
        Foreground,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Transport {
        Wss,
        HttpsPoll,
    }

    /// Agent metrics reported in each heartbeat.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AgentMetrics {
        pub cpu_percent:       f32,
        pub memory_used_mb:    u64,
        pub memory_total_mb:   u64,
        pub disk_used_percent: f32,
        pub load_1m:           f32,
        pub load_5m:           f32,
        pub load_15m:          f32,
    }

    /// Agent metadata stored in OpenBao at `secret/endpoints/{id}/agent`.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AgentInfo {
        pub schema_version:      u32,
        pub status:              AgentStatus,
        pub version:             String,
        pub deployed_at:         Option<DateTime<Utc>>,
        pub deployment_method:   String,
        pub service_mode:        ServiceMode,
        pub binary_checksum:     Option<String>,
        pub control_plane_url:   String,
        pub transport:           Transport,
        pub nkey_public:         Option<String>,
        pub last_heartbeat_at:   Option<DateTime<Utc>>,
        pub agent_uptime_seconds: Option<u64>,
        pub update_channel:      String,
    }

    /// Heartbeat payload published by the agent.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Heartbeat {
        pub endpoint_id:      String,
        pub reported_at:      DateTime<Utc>,
        pub agent_version:    String,
        pub transport:        Transport,
        pub uptime_seconds:   u64,
        pub metrics:          AgentMetrics,
    }
}

// ── Credentials ───────────────────────────────────────────────────────────────

pub mod credentials {
    use serde::{Deserialize, Serialize};
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum CredentialType {
        SshKey,
        Password,
        ApiToken,
        Kubeconfig,
        Sudo,
    }

    /// A reference to a credential stored in OpenBao. Never contains the value.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CredentialRef {
        pub vault_path: String,
        pub cred_type:  CredentialType,
        pub username:   Option<String>,
        pub added_at:   DateTime<Utc>,
        pub added_by:   String, // JWT sub claim, e.g. "github:joewxboy"
    }
}
