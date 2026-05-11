use anyhow::Result;
use serde_json::Value;
use tabled::{Table, Tabled};

use crate::client::ApiClient;
use super::{EndpointsArgs, EndpointsCommand};

pub async fn run(client: ApiClient, args: EndpointsArgs) -> Result<()> {
    match args.command {
        EndpointsCommand::List { status } => list(&client, status).await,
        EndpointsCommand::Get { id } => get(&client, &id).await,
    }
}

async fn list(client: &ApiClient, status_filter: Option<String>) -> Result<()> {
    let mut path = "/api/v1/endpoints".to_string();
    if let Some(s) = status_filter {
        path = format!("{path}?status={s}");
    }

    let endpoints: Vec<Value> = client.get(&path).await?;

    if client.output_format == "json" {
        println!("{}", serde_json::to_string_pretty(&endpoints)?);
        return Ok(());
    }

    // Table output
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "ID")]
        id: String,
        #[tabled(rename = "Hostname")]
        hostname: String,
        #[tabled(rename = "IP")]
        ip: String,
        #[tabled(rename = "Status")]
        status: String,
        #[tabled(rename = "OS")]
        os: String,
    }

    let rows: Vec<Row> = endpoints
        .iter()
        .map(|ep| Row {
            id: ep["id"].as_str().unwrap_or("-").to_string(),
            hostname: ep["network"]["primary_hostname"].as_str().unwrap_or("-").to_string(),
            ip: ep["network"]["primary_ip"].as_str().unwrap_or("-").to_string(),
            status: ep["status"].as_str().unwrap_or("-").to_string(),
            os: ep["identity"]["distro"].as_str().unwrap_or("-").to_string(),
        })
        .collect();

    println!("{}", Table::new(rows));
    Ok(())
}

async fn get(client: &ApiClient, id: &str) -> Result<()> {
    let endpoint: Value = client.get(&format!("/api/v1/endpoints/{id}")).await?;

    if client.output_format == "json" {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        // Human-readable key-value output
        println!("ID:       {}", endpoint["id"].as_str().unwrap_or("-"));
        println!("Status:   {}", endpoint["status"].as_str().unwrap_or("-"));
        println!("Hostname: {}", endpoint["network"]["primary_hostname"].as_str().unwrap_or("-"));
        println!("IP:       {}", endpoint["network"]["primary_ip"].as_str().unwrap_or("-"));
        println!("OS:       {}", endpoint["identity"]["distro"].as_str().unwrap_or("-"));
        println!("Arch:     {}", endpoint["identity"]["arch"].as_str().unwrap_or("-"));
    }
    Ok(())
}
