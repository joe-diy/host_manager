use anyhow::Result;
use serde_json::{json, Value};

use super::{CommandsArgs, CommandsCommand};
use crate::client::ApiClient;

pub async fn run(client: ApiClient, args: CommandsArgs) -> Result<()> {
    match args.command {
        CommandsCommand::Exec {
            target,
            command,
            timeout,
        } => exec(&client, &target, command, timeout).await,
    }
}

async fn exec(client: &ApiClient, target: &str, command: Vec<String>, timeout: u64) -> Result<()> {
    let cmd_str = command.join(" ");
    println!("Dispatching to {target}: {cmd_str}");

    // Resolve target → list of endpoint IDs
    let endpoint_ids: Vec<String> = if target == "all" {
        let eps: Vec<Value> = client.get("/api/v1/endpoints").await?;
        eps.iter()
            .filter_map(|ep| ep["id"].as_str().map(String::from))
            .collect()
    } else {
        target.split(',').map(str::trim).map(String::from).collect()
    };

    for id in &endpoint_ids {
        let body = json!({
            "type": "exec",
            "command": cmd_str,
            "timeout_seconds": timeout,
        });
        match client
            .post::<_, Value>(&format!("/api/v1/endpoints/{id}/commands"), &body)
            .await
        {
            Ok(resp) => {
                println!(
                    "[{id}] dispatched — command_id: {}",
                    resp["command_id"].as_str().unwrap_or("-")
                );
            }
            Err(e) => {
                eprintln!("[{id}] error: {e}");
            }
        }
    }

    Ok(())
}
