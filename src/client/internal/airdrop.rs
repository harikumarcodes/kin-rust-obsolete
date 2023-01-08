use {
    crate::{
        error::{Error, TransactionError},
        gen::kin::agora::{airdrop::v4 as airdrop_pb, common::v4 as model_pb_v4},
        key::public::PublicKey,
        solana::commitment::Commitment,
    },
    tonic::transport::{Channel, ClientTlsConfig, Error as TonicError},
};

pub struct InternalAirdropClient {
    client: airdrop_pb::airdrop_client::AirdropClient<Channel>,
}

impl InternalAirdropClient {
    pub async fn new(endpoint: &'static str) -> Result<InternalAirdropClient, TonicError> {
        let tls = ClientTlsConfig::new();
        let channel = Channel::from_static(endpoint)
            .tls_config(tls)?
            .connect()
            .await?;

        let airdrop_client = InternalAirdropClient {
            client: airdrop_pb::airdrop_client::AirdropClient::new(channel),
        };

        Ok(airdrop_client)
    }

    pub async fn request_airdrop(
        &mut self,
        public_key: &PublicKey,
        quarks: u64,
        commitment: Commitment,
    ) -> Result<Vec<u8>, Error> {
        let account_id = model_pb_v4::SolanaAccountId {
            value: public_key.to_bytes().to_vec(),
        };

        let req = airdrop_pb::RequestAirdropRequest {
            account_id: Some(account_id),
            quarks,
            commitment: commitment as i32,
        };

        let res = self.client.request_airdrop(req).await.unwrap().into_inner();

        use airdrop_pb::request_airdrop_response::Result;
        match Result::from_i32(res.result) {
            Some(Result::Ok) => match res.signature {
                Some(sig) => Ok(sig.value),
                None => panic!("No signature received from Agora."),
            },
            Some(Result::NotFound) => Err(TransactionError::AccountDoesNotExist(None).into()),
            Some(Result::InsufficientKin) => {
                Err(TransactionError::InsufficientBalance(None).into())
            }
            None => panic!("Unexpected result from Agora: {}.", res.result),
        }
    }
}
