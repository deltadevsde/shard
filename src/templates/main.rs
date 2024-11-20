use anyhow::{Context, Result};
use celestia_types::nmt::Namespace;
use clap::{Parser, Subcommand};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tx::Transaction;

mod node;
mod state;
mod tx;
mod webserver;
use node::{Config, Node};

#[macro_use]
extern crate log;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// The namespace used by this rollup (hex encoded)
    #[arg(long, default_value = "2a2a2a2a")]
    namespace: String,

    /// The height from which to start syncing
    #[arg(long, default_value_t = 1)]
    start_height: u64,

    /// The URL of the Celestia node to connect to
    #[arg(long, default_value = "ws://0.0.0.0:26658")]
    celestia_url: String,

    /// The address to listen on for the node's webserver
    #[arg(long, default_value = "0.0.0.0:3000")]
    listen_addr: String,

    /// The auth token to use when connecting to Celestia
    #[arg(long)]
    auth_token: Option<String>,

    /// The interval at which to post batches of transactions (in seconds)
    #[arg(long, default_value_t = 3)]
    batch_interval: u64,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the node
    Serve,
    /// Submit a transaction
    SubmitTx {
        #[command(subcommand)]
        tx: Transaction,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let args = Args::parse();

    let namespace =
        Namespace::new_v0(&hex::decode(&args.namespace).context("Invalid namespace hex")?)
            .context("Failed to create namespace")?;

    let config = Config {
        namespace,
        start_height: args.start_height,
        celestia_url: args.celestia_url,
        listen_addr: args.listen_addr,
        auth_token: args.auth_token,
        batch_interval: Duration::from_secs(args.batch_interval),
    };

    // Run the async main in the smol runtime
    match args.command {
        Command::Serve => start_node(config).await,
        Command::SubmitTx { tx } => submit_tx(config, tx).await,
    }
}

async fn start_node(config: Config) -> Result<()> {
    let node = Arc::new(Node::new(config).await?);

    node.start().await?;

    Ok(())
}

async fn submit_tx(config: Config, tx: Transaction) -> Result<()> {
    let url = format!("http://{}/submit_tx", config.listen_addr);

    let client = reqwest::Client::new();
    let response = client.post(url).json(&tx).send().await?;

    if response.status().is_success() {
        println!("Transaction submitted successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to submit transaction: {}",
            response.text().await?
        ))
    }
}
