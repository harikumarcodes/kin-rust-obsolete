use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// Commitment is used to indicate to Solana nodes which bank state to query.
///
/// See: https://docs.solana.com/apps/jsonrpc-api#configuring-state-commitment
#[derive(Debug, PartialEq, Eq, FromPrimitive, Copy, Clone)]
pub enum Commitment {
    /// The node will query its most recent block.
    Recent = 0,

    /// The node will query the most recent block that has been voted on by supermajority of the cluster.
    Single = 1,

    /// The node will query the most recent block having reached maximum lockout on this node.
    Root = 2,

    /// The node will query the most recent block confirmed by supermajority of the cluster as having reached maximum lockout.
    Max = 3,
}

impl Commitment {
    /// Converts i32 to Commitment. Defaults to Commitment::Single.
    pub fn from_i32(value: i32) -> Commitment {
        if let Some(commitment) = FromPrimitive::from_i32(value) {
            commitment
        } else {
            Commitment::Single
        }
    }
}

impl Default for Commitment {
    fn default() -> Self {
        Commitment::Single
    }
}
