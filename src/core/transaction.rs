use std::fmt;

use serde::Deserialize;

use crate::core::{Amount, ClientId};

/// Transaction ID.
pub type TransactionId = u16;

/// RawTransaction types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionType {
    /// Deposit.
    Deposit,
    /// Withdrawal.
    Withdrawal,
    /// Dispute.
    Dispute,
    /// Resolve.
    Resolve,
    /// Chargeback.
    Chargeback,
}

impl<'de> Deserialize<'de> for TransactionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        pub struct TypeVisitor;

        impl<'de> de::Visitor<'de> for TypeVisitor {
            type Value = TransactionType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid transaction type string")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(match s {
                    "deposit" => TransactionType::Deposit,
                    "withdrawal" => TransactionType::Withdrawal,
                    "dispute" => TransactionType::Dispute,
                    "resolve" => TransactionType::Resolve,
                    "chargeback" => TransactionType::Chargeback,
                    _ => return Err(de::Error::invalid_value(de::Unexpected::Str(s), &self)),
                })
            }
        }

        deserializer.deserialize_str(TypeVisitor)
    }
}

/// A raw transaction
///
/// Represents a single entry from the provided CSV containing
/// the transactions.
#[derive(Debug, Clone, Deserialize)]
pub struct RawTransaction {
    /// RawTransaction type.
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    /// Client.
    pub client: ClientId,
    /// RawTransaction ID.
    #[serde(rename = "tx")]
    pub id: TransactionId,
    /// Amount of the transaction.
    #[serde(deserialize_with = "crate::core::deserialize_amount")]
    pub amount: Amount,
}

/// A transaction with information about it's state.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub raw: RawTransaction,
    pub disputed: bool,
}

impl From<RawTransaction> for Transaction {
    fn from(raw: RawTransaction) -> Self {
        Transaction {
            raw,
            disputed: false,
        }
    }
}
