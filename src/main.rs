use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const CARGO_TEMPLATE: &str = r#"[package]
name = "{{project_name}}"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.6.0"
bincode = "1.3.3"
celestia-rpc = "0.4.0"
celestia-types = "0.4.0"
serde = "1.0.210"
tokio = { version = "1.40.0", features = ["full"] }
anyhow = "1.0.89"
reqwest = { version = "0.12.7", features = ["json"] }
serde_json = "1.0.128"
hex = "0.4.3"
ed25519-dalek = "2.1.1"
keystore-rs = "0.1.0"
smol = "1.3.0"
futures-lite = "1.13.0"
async-lock = "2.8.0"
async-channel = "1.9.0"
log = "0.4.22"
pretty_env_logger = "0.5.0""#;

const LIB_RS: &str = r#"pub mod node;
pub mod state;
pub mod tx;

#[macro_use]
extern crate log;"#;

const MAIN_RS: &str = r#"fn main() {
    println!("Hello, world!");
}"#;

const NODE_RS: &str = r#"use anyhow::{Context, Result};
use async_channel::{bounded, Receiver, Sender};
use async_lock::Mutex;
use celestia_rpc::{BlobClient, HeaderClient};
use celestia_types::{nmt::Namespace, Blob, TxConfig};
use futures_lite::future;
use smol::Timer;
use std::sync::Arc;
use std::time::Duration;

use crate::tx::Batch;
use crate::{state::State, tx::Transaction};

const DEFAULT_BATCH_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub struct Config {
    /// The namespace used by this rollup.
    namespace: Namespace,

    /// The height from which to start syncing.
    start_height: u64,

    /// The URL of the Celestia node to connect to.
    celestia_url: String,
    /// The auth token to use when connecting to Celestia.
    auth_token: Option<&'static str>,

    /// The interval at which to post batches of transactions.
    batch_interval: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            namespace: Namespace::new_v0(&[42, 42, 42, 42]).unwrap(),
            start_height: 0,
            celestia_url: "http://localhost:8080".to_string(),
            auth_token: None,
            batch_interval: DEFAULT_BATCH_INTERVAL,
        }
    }
}

pub struct Node {
    da_client: celestia_rpc::Client,
    cfg: Config,

    /// The state of the rollup that is mutated by incoming transactions
    state: Arc<Mutex<State>>,

    /// Transactions that have been queued for batch posting to Celestia
    pending_transactions: Arc<Mutex<Vec<Transaction>>>,

    /// Used to notify the syncer that genesis sync has completed, and queued
    /// stored blocks from incoming sync can be processed
    genesis_sync_completed: (Sender<()>, Receiver<()>),
}

impl Node {
    pub async fn new(cfg: Config) -> Result<Self> {
        let da_client = celestia_rpc::Client::new(&cfg.celestia_url, cfg.auth_token)
            .await
            .context("Couldn't start Celestia client")?;

        Ok(Node {
            cfg,
            da_client,
            genesis_sync_completed: bounded(1),
            pending_transactions: Arc::new(Mutex::new(Vec::new())),
            state: Arc::new(Mutex::new(State::new())),
        })
    }

    pub async fn queue_transaction(&self, tx: Transaction) -> Result<()> {
        self.pending_transactions.lock().await.push(tx);
        Ok(())
    }

    async fn post_pending_batch(&self) -> Result<Batch> {
        let mut pending_txs = self.pending_transactions.lock().await;
        if pending_txs.is_empty() {
            return Ok(Batch::new(Vec::new()));
        }

        let batch = Batch::new(pending_txs.drain(..).collect());
        let encoded_batch = bincode::serialize(&batch)?;
        let blob = Blob::new(self.cfg.namespace, encoded_batch)?;
        BlobClient::blob_submit(&self.da_client, &[blob], TxConfig::default()).await?;

        Ok(batch)
    }

    async fn process_l1_block(&self, blobs: Vec<Blob>) {
        let txs: Vec<Transaction> = blobs
            .into_iter()
            .flat_map(|blob| {
                Batch::try_from(&blob)
                    .map(|b| b.get_transactions())
                    .unwrap_or_default()
            })
            .collect();

        let mut state = self.state.lock().await;
        for tx in txs {
            if let Err(e) = state.process_tx(tx) {
                error!("processing tx: {}", e);
            }
        }
    }

    async fn sync_historical(&self) -> Result<()> {
        let network_head = HeaderClient::header_network_head(&self.da_client).await?;
        let network_height = network_head.height();
        info!(
            "syncing historical blocks from {}-{}",
            self.cfg.start_height,
            network_height.value()
        );

        for height in self.cfg.start_height..network_height.value() {
            if let Some(blobs) =
                BlobClient::blob_get_all(&self.da_client, height, &[self.cfg.namespace]).await?
            {
                self.process_l1_block(blobs).await;
            }
        }

        let _ = self.genesis_sync_completed.0.send(()).await;
        info!("historical sync completed");

        Ok(())
    }

    async fn start_batch_posting(&self) {
        loop {
            Timer::after(self.cfg.batch_interval).await;
            match self.post_pending_batch().await {
                Ok(batch) => {
                    let tx_count = batch.get_transactions().len();
                    if tx_count > 0 {
                        info!("batch posted with {} transactions", tx_count);
                    }
                }
                Err(e) => error!("posting batch: {}", e),
            }
        }
    }

    async fn sync_incoming_blocks(&self) -> Result<()> {
        let mut blobsub = BlobClient::blob_subscribe(&self.da_client, self.cfg.namespace)
            .await
            .context("Failed to subscribe to app namespace")?;

        self.genesis_sync_completed.1.recv().await?;

        while let Some(result) = blobsub.next().await {
            match result {
                Ok(blob_response) => {
                    if let Some(blobs) = blob_response.blobs {
                        self.process_l1_block(blobs).await;
                    }
                }
                Err(e) => error!("retrieving blobs from DA layer: {}", e),
            }
        }
        Ok(())
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        let genesis_sync = {
            let node = self.clone();
            smol::spawn(async move { node.sync_historical().await })
        };

        let incoming_sync = {
            let node = self.clone();
            smol::spawn(async move { node.sync_incoming_blocks().await })
        };

        let batch_posting = {
            let node = self.clone();
            smol::spawn(async move {
                node.start_batch_posting().await;
                Ok(()) as Result<()>
            })
        };

        future::race(future::race(genesis_sync, incoming_sync), batch_posting).await?;

        Ok(())
    }
}"#;

const STATE_RS: &str = r#"use crate::tx::Transaction;
use anyhow::Result;

pub struct State {}

impl State {
    pub fn new() -> Self {
        State {}
    }

    /// Validates a transaction against the current chain state.
    /// Called during [`process_tx`], but can also be used independently, for
    /// example when queuing transactions to be batched.
    pub(crate) fn validate_tx(&self, tx: Transaction) -> Result<()> {
        tx.verify()?;
        Ok(())
    }

    /// Processes a transaction by validating it and updating the state.
    pub(crate) fn process_tx(&mut self, tx: Transaction) -> Result<()> {
        self.validate_tx(tx)?;
        Ok(())
    }
}"#;

const TX_RS: &str = r#"use anyhow::{Context, Result};
use celestia_types::Blob;
use serde::{Deserialize, Serialize};

/// Represents the full set of transaction types supported by the system.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Transaction {
    Noop,
}

impl Transaction {
    pub fn verify(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct Batch(Vec<Transaction>);

impl Batch {
    pub fn new(txs: Vec<Transaction>) -> Self {
        Batch(txs.clone())
    }

    pub fn get_transactions(&self) -> Vec<Transaction> {
        self.0.clone()
    }
}

impl TryFrom<&Blob> for Batch {
    type Error = anyhow::Error;

    fn try_from(value: &Blob) -> Result<Self, Self::Error> {
        match bincode::deserialize(&value.data) {
            Ok(batch) => Ok(batch),
            Err(_) => {
                let transaction: Transaction = bincode::deserialize(&value.data)
                    .context(format!("Failed to decode blob into Transaction: {value:?}"))?;

                Ok(Batch(vec![transaction]))
            }
        }
    }
}"#;

fn create_project(project_name: &str) -> Result<()> {
    Command::new("cargo")
        .args(["new", project_name])
        .output()
        .context("Failed to create new cargo project")?;

    let project_path = Path::new(project_name).join("src");
    let src_path = project_path.join("src");

    fs::write(src_path.join("lib.rs"), LIB_RS).context("Failed to create lib.rs")?;
    fs::write(src_path.join("main.rs"), MAIN_RS).context("Failed to update main.rs")?;
    fs::write(src_path.join("node.rs"), NODE_RS).context("Failed to create node.rs")?;
    fs::write(src_path.join("state.rs"), STATE_RS).context("Failed to create state.rs")?;
    fs::write(src_path.join("tx.rs"), TX_RS).context("Failed to create tx.rs")?;

    let cargo_content = CARGO_TEMPLATE.replace("{{project_name}}", project_name);
    fs::write(project_path.join("Cargo.toml"), cargo_content)
        .context("Failed to update Cargo.toml")?;

    println!("✨ Created new rollup project: {}", project_name);
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("init") => {
            let project_name = args.get(2).map(|s| s.as_str()).unwrap_or("my-rollup");
            create_project(project_name)?;
        }
        _ => {
            println!("Usage: shard init [project-name]");
            println!(
                "Creates a new rollup project with the specified name (defaults to my-rollup)"
            );
        }
    }

    Ok(())
}
