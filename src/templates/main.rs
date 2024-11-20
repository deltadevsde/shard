use anyhow::{Context, Result};
use celestia_types::nmt::Namespace;
use clap::Parser;
use std::sync::Arc;
use std::time::Duration;

mod node;
mod state;
mod tx;
use node::{Config, Node};

#[macro_use]
extern crate log;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The namespace used by this rollup (hex encoded)
    #[arg(long, default_value = "2a2a2a2a")]
    namespace: String,

    /// The height from which to start syncing
    #[arg(long, default_value_t = 1)]
    start_height: u64,

    /// The URL of the Celestia node to connect to
    #[arg(long, default_value = "ws://localhost:26658")]
    celestia_url: String,

    /// The auth token to use when connecting to Celestia
    #[arg(long)]
    auth_token: Option<String>,

    /// The interval at which to post batches of transactions (in seconds)
    #[arg(long, default_value_t = 3)]
    batch_interval: u64,
}

fn main() -> Result<()> {
    // Initialize logging
    pretty_env_logger::init();

    // Parse command-line arguments
    let args = Args::parse();

    // Create configuration
    let namespace =
        Namespace::new_v0(&hex::decode(&args.namespace).context("Invalid namespace hex")?)
            .context("Failed to create namespace")?;

    let config = Config {
        namespace,
        start_height: args.start_height,
        celestia_url: args.celestia_url,
        auth_token: args.auth_token,
        batch_interval: Duration::from_secs(args.batch_interval),
    };

    // Run the async main in the smol runtime
    smol::block_on(async_main(config))
}

async fn async_main(config: Config) -> Result<()> {
    // Create the node
    let node = Arc::new(Node::new(config).await?);

    // Start the node
    node.start().await?;

    Ok(())
}
