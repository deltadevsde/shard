use anyhow::{Context, Result};
use celestia_types::nmt::Namespace;
use clap::{Parser, Subcommand};
use keystore_rs::KeyStore;
use prism_common::keys::{Signature, VerifyingKey};
use std::sync::Arc;
use std::time::Duration;
use tx::{Transaction, TransactionType, SIGNATURE_VERIFICATION_ENABLED};

mod node;
mod state;
mod tx;
mod webserver;
use node::{Config, Node};

#[macro_use]
extern crate log;

#[derive(Parser, Debug)]
struct CommonArgs {
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
    Serve(CommonArgs),
    /// Submit a transaction
    SubmitTx(SubmitTxArgs),
    /// Create a signer
    CreateSigner(CreateSignerArgs),
}

#[derive(Parser, Debug)]
struct SubmitTxArgs {
    #[command(subcommand)]
    tx: TransactionType,

    #[arg(long, default_value = "default")]
    key_name: String,

    #[arg(long, default_value = "0")]
    nonce: u64,

    #[command(flatten)]
    common: CommonArgs,
}

#[derive(Parser, Debug)]
struct CreateSignerArgs {
    /// The name of the key to create (used for signing transactions)
    key_name: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let args = Args::parse();

    match args.command {
        Command::Serve(common_args) => {
            let config = config_from_args(common_args)?;
            start_node(config).await
        }
        Command::SubmitTx(SubmitTxArgs {
            common,
            key_name,
            nonce,
            tx,
        }) => {
            let config = config_from_args(common)?;
            submit_tx(config, key_name, nonce, tx).await
        }
        Command::CreateSigner(CreateSignerArgs { key_name }) => create_signer(key_name),
    }
}

fn create_signer(key_name: String) -> Result<()> {
    let signer = keystore_rs::create_signing_key();
    keystore_rs::KeyChain
        .add_signing_key(key_name.as_str(), &signer)
        .map_err(|e| anyhow::anyhow!("Failed to create signer: {}", e))?;
    info!("Signer '{}' created successfully", key_name);
    Ok(())
}

fn config_from_args(args: CommonArgs) -> Result<Config> {
    let namespace =
        Namespace::new_v0(&hex::decode(&args.namespace).context("Invalid namespace hex")?)
            .context("Failed to create namespace")?;

    Ok(Config {
        namespace,
        start_height: args.start_height,
        celestia_url: args.celestia_url,
        listen_addr: args.listen_addr,
        auth_token: args.auth_token,
        batch_interval: Duration::from_secs(args.batch_interval),
    })
}

async fn start_node(config: Config) -> Result<()> {
    let node = Arc::new(Node::new(config).await?);

    node.start().await?;

    Ok(())
}

async fn submit_tx(
    config: Config,
    key_name: String,
    nonce: u64,
    tx_variant: TransactionType,
) -> Result<()> {
    let url = format!("http://{}/submit_tx", config.listen_addr);

    let tx = if SIGNATURE_VERIFICATION_ENABLED {
        let signer = keystore_rs::KeyChain
            .get_signing_key(key_name.as_str())
            .unwrap();
        let vk: VerifyingKey = signer.clone().into();
        let mut tx = Transaction {
            signature: Signature::default(),
            nonce,
            vk,
            tx_type: tx_variant,
        };

        // TODO: ugly api
        tx.sign(&prism_common::keys::SigningKey::Ed25519(Box::new(signer)))?;
        tx
    } else {
        Transaction {
            signature: Signature::default(),
            nonce: 0,
            vk: VerifyingKey::Ed25519(keystore_rs::create_signing_key().verification_key()),
            tx_type: tx_variant,
        }
    };

    let client = reqwest::Client::new();
    let response = client.post(url).json(&tx).send().await?;

    if response.status().is_success() {
        info!("Transaction submitted successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to submit transaction: {}",
            response.text().await?
        ))
    }
}
