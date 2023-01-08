use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// Indicates which type of account resolution should be used if a transaction on Kin 4 fails due to
/// an account being unavailable.
#[derive(Debug, PartialEq, Eq, FromPrimitive, Copy, Clone)]
pub enum AccountResolution {
    /// No account resolution will be used.
    Exact = 0,

    /// When used for a sender key, in a payment or earn request, if Agora is able to resolve the original sender public key to
    /// a set of token accounts, the original sender will be used as the owner in the Solana transfer instruction and the first
    /// resolved token account will be used as the sender.
    ///
    /// When used for a destination key in a payment or earn request, if Agora is able to resolve the destination key to a set
    /// of token accounts, the first resolved token account will be used as the destination in the Solana transfer instruction.
    Preferred = 1,
}

impl AccountResolution {
    /// Converts i32 to AccountResolution. Defaults to AccountResolution::Exact.
    pub fn from_i32(value: i32) -> AccountResolution {
        if let Some(account_resolution) = FromPrimitive::from_i32(value) {
            account_resolution
        } else {
            AccountResolution::Exact
        }
    }
}

impl Default for AccountResolution {
    fn default() -> Self {
        AccountResolution::Preferred
    }
}
