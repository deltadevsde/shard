use crate::tx::{Transaction, TransactionType};
use anyhow::Result;

#[derive(Default)]
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
        match tx.tx_type {
            TransactionType::Noop => Ok(()),
        }
    }

    /// Processes a transaction by validating it and updating the state.
    pub(crate) fn process_tx(&mut self, tx: Transaction) -> Result<()> {
        self.validate_tx(tx.clone())?;
        match tx.tx_type {
            TransactionType::Noop => Ok(()),
        }
    }
}
