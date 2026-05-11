//! Discovery capability provider.
//!
//! Receives NATS messages from the discovery-orchestrator actor and performs
//! network scanning to enumerate live hosts on a given CIDR subnet.
//!
//! Protocol (request-reply over NATS):
//!   Subject:  `discovery.scan`
//!   Request:  `{ "subnet": "192.168.1.0/24" }`
//!   Response: `{ "results": [ { "ip": "…", "mac": "…", "hostnames": […] } ] }`
//!
//! Scan strategy (MVP):
//!   1. Parse CIDR with `ipnetwork`
//!   2. Send ICMP echo (ping) to each address in the range (parallel, 50 ms timeout)
//!   3. For hosts that respond, attempt ARP table lookup for MAC address
//!   4. Attempt reverse DNS for hostnames
//!   5. Respond with list of `DiscoveryResult`

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,hostmgr_provider_discovery=debug".into()),
        )
        .init();

    info!("discovery provider starting");

    // TODO: connect to NATS using wasmcloud-provider-sdk host data
    // TODO: subscribe to "discovery.scan" subject
    // TODO: for each request, run scan and reply with results

    // Block forever (provider process; wasmcloud host manages lifecycle)
    tokio::signal::ctrl_c().await?;
    info!("discovery provider shutting down");
    Ok(())
}
