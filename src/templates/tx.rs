use anyhow::{Context, Result};
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
}
