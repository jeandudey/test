use std::collections::HashMap;

use serde::Serialize;

use crate::core::Amount;

/// Client ID.
pub type ClientId = u16;

/// The account containing the finances of each client.
///
/// Final state of each client that is written to the output
/// CSV file.
#[derive(Debug, Clone)]
pub struct Account {
    pub available: Amount,
    pub held: Amount,
    pub locked: bool,
}

impl Account {
    /// Create a new [``Account`]
    pub fn new() -> Account {
        Account {
            available: Amount::ZERO,
            held: Amount::ZERO,
            locked: false,
        }
    }
}

/// A raw account (e.g.: that's serialized)
#[derive(Debug, Serialize)]
pub struct RawAccount {
    #[serde(serialize_with = "crate::core::serialize_amount")]
    pub available: Amount,
    #[serde(serialize_with = "crate::core::serialize_amount")]
    pub held: Amount,
    #[serde(serialize_with = "crate::core::serialize_amount")]
    pub total: Amount,
    pub locked: bool,
}

impl From<Account> for RawAccount {
    fn from(a: Account) -> Self {
        RawAccount {
            available: a.available,
            held: a.held,
            total: a.available + a.held,
            locked: a.locked,
        }
    }
}

/// Data structure holding accounts.
#[derive(Debug)]
pub struct Accounts {
    pub data: HashMap<ClientId, Account>,
}

impl Accounts {
    /// Create a new [`Accounts`].
    pub fn new() -> Accounts {
        Accounts {
            data: HashMap::new(),
        }
    }

    /// Get an existent account or create it.
    pub fn get_or_create(&mut self, id: ClientId) -> &mut Account {
        self.data.entry(id).or_insert(Account::new());
        self.data.get_mut(&id).unwrap()
    }

    /// Check if an account exists.
    pub fn exists(&self, id: ClientId) -> bool {
        self.data.get(&id).is_some()
    }
}