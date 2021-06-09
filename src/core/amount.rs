use std::{fmt, str::FromStr, string::ToString};

/// Financial amount of the transactions. Stored as a fixed point integer to
/// avoid loss of precision using fp numbers (`f32`, `f64`).
///
/// 64 bits for the integer and 64 bits for the fractional part, even though we
/// only use 4 digits past the decimal.
pub type Amount = fixed::types::I64F64;

/// Custom deserializer function for an [`Amount`]
pub fn deserialize_amount<'de, D>(deserializer: D) -> Result<Amount, D::Error>
where
    D: serde::Deserializer<'de>
{
    use serde::de;

    pub struct AmountVisitor;

    impl<'de> de::Visitor<'de> for AmountVisitor {
        type Value = Amount;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a valid amount")
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Amount::from_str(s).map_err(|e| de::Error::custom(e))
        }
    }

    deserializer.deserialize_str(AmountVisitor)
}

/// Custom deserializer function for an [`Amount`]
pub fn serialize_amount<S>(amount: &Amount, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&amount.to_string())
}