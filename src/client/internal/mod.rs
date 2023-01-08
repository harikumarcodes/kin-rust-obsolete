pub mod account;
pub mod airdrop;
pub mod transaction;

use account::InternalAccountClient;
use airdrop::InternalAirdropClient;
use transaction::InternalTransactionClient;

pub struct InternalClient {
    pub account: InternalAccountClient,
    pub tx: InternalTransactionClient,
    pub airdrop: InternalAirdropClient,
}
