//! Identification capability provider.
//!
//! Receives SSH probe requests from the identifier actor and returns
//! structured identification results (OS family, distro, version, arch).
//!
//! Protocol (request-reply over NATS):
//!   Subject:  `identification.probe`
//!   Request:  `{ "ip": "192.168.1.42", "port": 22, "credential_ref": "secret/endpoints/…" }`
//!   Response: IdentificationResult JSON
//!
//! SSH probe strategy:
//!   1. Open SSH connection using russh
//!   2. Run: `uname -a`, `cat /etc/os-release`, `hostnamectl` (if available)
//!   3. Parse output into IdentificationResult
//!   4. Close connection
//!   5. Respond; set confidence = High if all commands succeeded, Medium if partial

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,hostmgr_provider_identification=debug".into()),
        )
        .init();

    info!("identification provider starting");

    // TODO: connect to NATS using wasmcloud-provider-sdk host data
    // TODO: subscribe to "identification.probe" subject
    // TODO: for each request, establish SSH connection and run probe commands

    tokio::signal::ctrl_c().await?;
    info!("identification provider shutting down");
    Ok(())
}
