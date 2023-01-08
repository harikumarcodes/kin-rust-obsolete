use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// The type of a Kin transaction.
#[derive(Debug, PartialEq, Eq, FromPrimitive, Copy, Clone)]
pub enum TransactionType {
    Unknown = -1,
    None = 0,
    Earn = 1,
    Spend = 2,
    P2P = 3,
}

impl TransactionType {
    /// Converts i32 to TrasactionType.
    pub fn from_i32(value: i32) -> TransactionType {
        if let Some(transaction_type) = FromPrimitive::from_i32(value) {
            transaction_type
        } else {
            TransactionType::Unknown
        }
    }
}
