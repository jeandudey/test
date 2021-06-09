//! Core program types.

use std::collections::HashMap;

use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot::{channel as oneshot_channel, error::TryRecvError, Sender as OneshotSender},
};

use anyhow::{Context, Result};

mod account;
mod amount;
mod transaction;

pub use account::{Account, Accounts, ClientId, RawAccount};
pub use amount::{deserialize_amount, serialize_amount, Amount};
pub use transaction::{RawTransaction, Transaction, TransactionId, TransactionType};

#[derive(Debug)]
pub enum Action {
    /// A transaction to process.
    RawTx(RawTransaction),
    /// Close the transaction processor, returns the state of the accounts.
    Close(OneshotSender<Accounts>),
}

#[derive(Debug)]
pub struct Task {
    sender: UnboundedSender<Action>,
}

impl Task {
    pub fn new() -> Task {
        let (sender, receiver) = unbounded_channel::<Action>();
        // Spawn our transaction processor.
        tokio::task::spawn(async move { task(receiver).await });

        Task { sender }
    }

    pub fn send_tx(&self, tx: RawTransaction) -> Result<()> {
        Ok(self
            .sender
            .send(Action::RawTx(tx))
            .context("Transaction processor task stopped")?)
    }

    pub fn close(self) -> Result<Accounts> {
        let (results_tx, mut results_rx) = oneshot_channel::<Accounts>();
        self.sender
            .send(Action::Close(results_tx))
            .context("Transaction processor task stopped")?;

        loop {
            match results_rx.try_recv() {
                Err(e) if e == TryRecvError::Empty => {}
                Err(_) => anyhow::bail!("Could not retrieve acocunts information"),
                Ok(accounts) => return Ok(accounts),
            }
        }
    }
}

async fn task(mut actions: UnboundedReceiver<Action>) {
    let mut accounts = Accounts::new();
    let mut transactions = HashMap::<TransactionId, Transaction>::new();

    while let Some(action) = actions.recv().await {
        match action {
            Action::RawTx(raw_tx) => {
                // For other transactions you need an account.
                if raw_tx.tx_type != TransactionType::Deposit && !accounts.exists(raw_tx.client) {
                    continue;
                }

                let account = accounts.get_or_create(raw_tx.client);
                let tx: Transaction = raw_tx.into();

                if tx.raw.tx_type == TransactionType::Deposit {
                    // Only deposits and withdrawals contain IDs, however,
                    // as this is only used to store transactions for posterior
                    // disputes, reolutions and chargebacks there is no need to store
                    // withdrawals as they are not disputed.
                    if transactions.get(&tx.raw.id).is_none() {
                        transactions.insert(tx.raw.id, tx.clone());
                    }
                }

                match tx.raw.tx_type {
                    TransactionType::Deposit => account.deposit(tx.raw.amount),
                    TransactionType::Withdrawal => account.withdraw(tx.raw.amount),
                    TransactionType::Dispute => {
                        // Find our disputed TX.
                        if let Some(disputed_tx) = transactions.get_mut(&tx.raw.id) {
                            account.hold(disputed_tx.raw.amount);
                            disputed_tx.disputed = true;
                        }
                    }
                    TransactionType::Resolve => {
                        if let Some(disputed_tx) = transactions.get_mut(&tx.raw.id) {
                            // If not disputed, just ignore it.
                            if !disputed_tx.disputed {
                                continue;
                            }

                            account.release(disputed_tx.raw.amount);
                            disputed_tx.disputed = false;
                        }
                    }
                    TransactionType::Chargeback => {
                        if let Some(disputed_tx) = transactions.get_mut(&tx.raw.id) {
                            // If not disputed, just ignore it.
                            if !disputed_tx.disputed {
                                continue;
                            }

                            account.chargeback(disputed_tx.raw.amount);
                        }
                    }
                }
            }
            Action::Close(tx) => {
                tx.send(accounts).ok();
                break;
            }
        }
    }
}
