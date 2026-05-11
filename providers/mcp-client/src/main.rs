//! MCP Client capability provider — optional enrichment from external MCP servers.
//!
//! Consumes external Model Context Protocol servers (NetBox, asset databases,
//! monitoring systems) and makes their data available to actors as NATS
//! request-reply calls. This provider is always optional: actors degrade
//! gracefully when the provider is absent or a server times out.
//!
//! Protocol (request-reply over NATS):
//!   Subject:  `mcp.query`
//!   Request:  `{ "server": "netbox", "tool": "get_device", "args": { "ip": "…" } }`
//!   Response: `{ "result": { … } }` or `{ "error": "timeout" }`
//!
//! Config (env vars; all optional — provider is no-op if none set):
//!   HOSTMGR_MCP_NETBOX_URL     — NetBox MCP server URL
//!   HOSTMGR_MCP_NETBOX_API_KEY — NetBox API key
//!   HOSTMGR_MCP_TIMEOUT_MS     — per-call timeout (default: 5000)
//!
//! Security note: MCP servers are consumed with a 5-second hard timeout
//! (ADR-011). They are never on the critical path for endpoint state changes.

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,hostmgr_provider_mcp_client=debug".into()),
        )
        .init();

    let timeout_ms: u64 = std::env::var("HOSTMGR_MCP_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);

    info!(timeout_ms, "mcp-client provider starting");

    // TODO: build reqwest client with timeout = timeout_ms
    // TODO: build server registry from env vars (HOSTMGR_MCP_*_URL)
    // TODO: subscribe to "mcp.query"
    // TODO: dispatch to registered MCP server, apply timeout, reply

    tokio::signal::ctrl_c().await?;
    info!("mcp-client provider shutting down");
    Ok(())
}
