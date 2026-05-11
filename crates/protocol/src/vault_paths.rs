//! OpenBao / Vault path constants.
//!
//! All paths follow the schema defined in ADR-008.
//! Use the helper functions to build per-endpoint paths.

// ── Endpoint sub-paths ────────────────────────────────────────────────────────

pub const VAULT_ENDPOINTS_PREFIX: &str = "secret/endpoints";

pub const ENDPOINT_SUB_CORE:        &str = "core";
pub const ENDPOINT_SUB_NETWORK:     &str = "network";
pub const ENDPOINT_SUB_IDENTITY:    &str = "identity";
pub const ENDPOINT_SUB_AGENT:       &str = "agent";
pub const ENDPOINT_SUB_CREDENTIALS: &str = "credentials";

// ── Credential paths ──────────────────────────────────────────────────────────

pub const VAULT_CREDENTIALS_PREFIX: &str = "secret/credentials";

// ── Agent NKey paths ──────────────────────────────────────────────────────────

pub const VAULT_AGENTS_PREFIX: &str = "secret/agents";
pub const AGENT_NKEY_SUFFIX:   &str = "nkey";

// ── Control plane config ──────────────────────────────────────────────────────

pub const VAULT_CONFIG_OAUTH_GITHUB_CLIENT_SECRET: &str =
    "secret/config/oauth/github_client_secret";
pub const VAULT_CONFIG_OAUTH_GOOGLE_CLIENT_SECRET: &str =
    "secret/config/oauth/google_client_secret";
pub const VAULT_CONFIG_JWT_SIGNING_KEY:    &str = "secret/config/jwt_signing_key";
pub const VAULT_CONFIG_API_KEYS_PREFIX:    &str = "secret/config/api_keys";
pub const VAULT_CONFIG_MCP_PREFIX:         &str = "secret/config/mcp";

// ── Path builder helpers ──────────────────────────────────────────────────────

/// `secret/endpoints/{id}/core`
pub fn endpoint_core_path(endpoint_id: &str) -> String {
    format!("{VAULT_ENDPOINTS_PREFIX}/{endpoint_id}/{ENDPOINT_SUB_CORE}")
}

/// `secret/endpoints/{id}/network`
pub fn endpoint_network_path(endpoint_id: &str) -> String {
    format!("{VAULT_ENDPOINTS_PREFIX}/{endpoint_id}/{ENDPOINT_SUB_NETWORK}")
}

/// `secret/endpoints/{id}/identity`
pub fn endpoint_identity_path(endpoint_id: &str) -> String {
    format!("{VAULT_ENDPOINTS_PREFIX}/{endpoint_id}/{ENDPOINT_SUB_IDENTITY}")
}

/// `secret/endpoints/{id}/agent`
pub fn endpoint_agent_path(endpoint_id: &str) -> String {
    format!("{VAULT_ENDPOINTS_PREFIX}/{endpoint_id}/{ENDPOINT_SUB_AGENT}")
}

/// `secret/endpoints/{id}/credentials`
pub fn endpoint_credentials_path(endpoint_id: &str) -> String {
    format!("{VAULT_ENDPOINTS_PREFIX}/{endpoint_id}/{ENDPOINT_SUB_CREDENTIALS}")
}

/// `secret/credentials/{id}/ssh`
pub fn credential_ssh_path(endpoint_id: &str) -> String {
    format!("{VAULT_CREDENTIALS_PREFIX}/{endpoint_id}/ssh")
}

/// `secret/agents/{id}/nkey`
pub fn agent_nkey_path(endpoint_id: &str) -> String {
    format!("{VAULT_AGENTS_PREFIX}/{endpoint_id}/{AGENT_NKEY_SUFFIX}")
}

/// `secret/config/api_keys/{key_id}`
pub fn api_key_path(key_id: &str) -> String {
    format!("{VAULT_CONFIG_API_KEYS_PREFIX}/{key_id}")
}
