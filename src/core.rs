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
enum Action {
    /// A transaction to process.
    RawTx(RawTransaction),
    /// Close the transaction processor, returns the state of the accounts.
    Close(OneshotSender<Accounts>),
}

/// Transaction processor task.
#[derive(Debug)]
pub struct Task {
    sender: UnboundedSender<Action>,
}

impl Task {
    /// Spawn a new [`Task`] that will handle all the transactions.
    pub fn new() -> Task {
        let (sender, receiver) = unbounded_channel::<Action>();
        // Spawn our transaction processor.
        tokio::task::spawn(async move { task(receiver).await });

        Task { sender }
    }

    /// Send a transaction to the [`Task`] to be processed.
    pub fn send_tx(&self, tx: RawTransaction) -> Result<()> {
        Ok(self
            .sender
            .send(Action::RawTx(tx))
            .context("Transaction processor task stopped")?)
    }

    /// Close the [`Task`] and return the result of the operation in the accounts.
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

/// Transaction processing task.
///
/// Here the transactions are received through an unbounded receiver
/// which can be changed later or be made into a configuration to
/// convert it to a bounded receiver where we know approx. how much
/// data we will have to sent to the thread. However for demonstration
/// purposes an unbounded one works just fine.
async fn task(mut actions: UnboundedReceiver<Action>) {
    // Keeo track of accounts and transactions. Accounts are created on deposits.
    // Only deposit transactions are stored for now as they are only used here
    // For the transaction types related to the dispues.
    //
    // Ideally the account data is saved to a database, but here we have to print it
    // to the console (in CSV) format, so it made sense to add a "Close" action that
    // will stop the task from receiving any more transactions to process and will
    // return the accounts information to be printed.
    //
    // A hash map is used to reduce the lookup time of old transactions and the same
    // is done with accounts.
    let mut accounts = Accounts::new();
    let mut transactions = HashMap::<TransactionId, Transaction>::new();

    // Process each action, they may come from anywhere here, the CSV is just
    // only one source :-D (intended to scale for multiple TCP streams sending
    // data like there's no tomorrow)
    while let Some(action) = actions.recv().await {
        match action {
            Action::RawTx(raw_tx) => {
                // For other transactions you need an account. To avoid transactions from
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

                // Process the transaction depending on it's type and apply the
                // corresponding operation.
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
