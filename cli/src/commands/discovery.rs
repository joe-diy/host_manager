use anyhow::Result;
use serde_json::{json, Value};

use super::{DiscoveryArgs, DiscoveryCommand};
use crate::client::ApiClient;

pub async fn run(client: ApiClient, args: DiscoveryArgs) -> Result<()> {
    match args.command {
        DiscoveryCommand::Start { subnet } => start(&client, subnet).await,
        DiscoveryCommand::Status => status(&client).await,
    }
}

async fn start(client: &ApiClient, subnet: Option<String>) -> Result<()> {
    let body = json!({ "subnet": subnet });
    let resp: Value = client.post("/api/v1/discovery/start", &body).await?;
    println!(
        "Discovery started: {}",
        resp["status"].as_str().unwrap_or("ok")
    );
    Ok(())
}

async fn status(client: &ApiClient) -> Result<()> {
    let resp: Value = client.get("/api/v1/discovery/status").await?;
    if client.output_format == "json" {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        println!(
            "Last run:  {}",
            resp["completed_at"].as_str().unwrap_or("-")
        );
        println!("Subnet:    {}", resp["subnet"].as_str().unwrap_or("-"));
        println!("Found:     {}", resp["count"].as_u64().unwrap_or(0));
    }
    Ok(())
}
