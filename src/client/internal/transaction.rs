use {
    crate::{
        client::proto_tx,
        error::{Error, TransactionError::AlreadySubmitted, TransactionErrors},
        gen::kin::agora::{
            common::{v3 as model_pb_v3, v4 as model_pb_v4},
            transaction::v4 as tx_pb,
        },
        model::transaction::{TransactionData, TransactionState},
        solana::{commitment::Commitment, token::program::ACCOUNT_LEN},
    },
    solana_sdk::transaction::Transaction as SolanaTransaction,
    tonic::transport::{Channel, ClientTlsConfig, Error as TonicError},
};

#[derive(Debug)]
pub struct SignTransactionResult {
    pub transaction_id: Option<Vec<u8>>,
    pub invoice_errors: Option<Vec<model_pb_v3::InvoiceError>>,
}

#[derive(Debug)]
pub struct SubmitTransactionResult {
    pub tx_id: Option<Vec<u8>>,
    pub invoice_errors: Option<Vec<model_pb_v3::InvoiceError>>,
    pub errors: Option<TransactionErrors>,
}

pub struct InternalTransactionClient {
    client: tx_pb::transaction_client::TransactionClient<Channel>,
}

impl InternalTransactionClient {
    pub async fn new(endpoint: &'static str) -> Result<InternalTransactionClient, TonicError> {
        let tls = ClientTlsConfig::new();
        let channel = Channel::from_static(endpoint)
            .tls_config(tls)?
            .connect()
            .await?;

        let tx_client = InternalTransactionClient {
            client: tx_pb::transaction_client::TransactionClient::new(channel),
        };

        Ok(tx_client)
    }

    pub async fn get_service_config(&mut self) -> tx_pb::GetServiceConfigResponse {
        let req = tx_pb::GetServiceConfigRequest {};

        self.client
            .get_service_config(req)
            .await
            .unwrap()
            .into_inner()
    }

    pub async fn get_recent_blockhash(&mut self) -> Vec<u8> {
        let req = tx_pb::GetRecentBlockhashRequest {};

        let res = self
            .client
            .get_recent_blockhash(req)
            .await
            .unwrap()
            .into_inner();

        res.blockhash.unwrap().value
    }

    pub async fn get_minimum_kin_version(&mut self) -> tx_pb::GetMinimumKinVersionResponse {
        let req = tx_pb::GetMinimumKinVersionRequest {};

        self.client
            .get_minimum_kin_version(req)
            .await
            .unwrap()
            .into_inner()
    }

    pub async fn get_minimum_balance_for_rent_exemption(&mut self) -> u64 {
        let req = tx_pb::GetMinimumBalanceForRentExemptionRequest { size: ACCOUNT_LEN };

        let res = self
            .client
            .get_minimum_balance_for_rent_exemption(req)
            .await
            .unwrap()
            .into_inner();

        res.lamports
    }

    pub async fn get_transaction(
        &mut self,
        id: &[u8],
        commitment: Option<Commitment>,
    ) -> TransactionData {
        let tx_id = model_pb_v4::TransactionId { value: id.to_vec() };

        let req = tx_pb::GetTransactionRequest {
            transaction_id: Some(tx_id),
            commitment: commitment.unwrap_or_default() as i32,
        };

        let res = self.client.get_transaction(req).await.unwrap().into_inner();

        use tx_pb::get_transaction_response::State;
        let state = match State::from_i32(res.state) {
            Some(state) => state,
            None => panic!("Unexpected state from Agora."),
        };

        match res.item {
            Some(item) => TransactionData::from_proto(&item, state),
            None => TransactionData::new(id.to_vec(), TransactionState::from_proto(state)),
        }
    }

    pub async fn sign_transaction(
        &mut self,
        transaction: &SolanaTransaction,
        invoice_list: Option<&model_pb_v3::InvoiceList>,
    ) -> SignTransactionResult {
        let req = tx_pb::SignTransactionRequest {
            transaction: Some(proto_tx(transaction)),
            invoice_list: invoice_list.cloned(),
        };

        let res = self
            .client
            .sign_transaction(req)
            .await
            .unwrap()
            .into_inner();

        let transaction_id = match res.signature {
            Some(sig) => Some(sig.value),
            None => None,
        };

        let mut sign_result = SignTransactionResult {
            transaction_id,
            invoice_errors: None,
        };

        use tx_pb::sign_transaction_response::Result;
        match Result::from_i32(res.result) {
            Some(Result::Ok) => (),
            Some(Result::Rejected) => panic!("Rejected."),
            Some(Result::InvoiceError) => {
                sign_result.invoice_errors = Some(res.invoice_errors);
            }
            None => panic!("Unexpected result from Agora: {}.", res.result),
        }

        sign_result
    }

    pub async fn submit_transaction(
        &mut self,
        tx: &SolanaTransaction,
        invoice_list: Option<&model_pb_v3::InvoiceList>,
        commitment: Option<Commitment>,
        dedupe_id: Option<&Vec<u8>>,
    ) -> Result<SubmitTransactionResult, Error> {
        let commitment = match commitment {
            Some(commitment) => commitment as i32,
            None => Commitment::Single as i32,
        };

        let dedupe_id = match dedupe_id {
            Some(dedupe_id) => dedupe_id.clone(),
            None => Vec::new(),
        };

        let req = tx_pb::SubmitTransactionRequest {
            transaction: Some(proto_tx(tx)),
            invoice_list: invoice_list.cloned(),
            commitment,
            dedupe_id,
            send_simulation_event: false,
        };

        let res = self
            .client
            .submit_transaction(req)
            .await
            .unwrap()
            .into_inner();

        let tx_id = match res.signature {
            Some(sig) => Some(sig.value),
            None => None,
        };

        let mut submit_result = SubmitTransactionResult {
            tx_id: tx_id.clone(),
            invoice_errors: None,
            errors: None,
        };

        use tx_pb::submit_transaction_response::Result;
        match Result::from_i32(res.result) {
            Some(Result::Ok) => (),
            Some(Result::AlreadySubmitted) => {
                return Err(Error::TransactionFailed(AlreadySubmitted(tx_id)))
            }
            Some(Result::Rejected) => return Err(Error::TransactionRejected),
            Some(Result::PayerRequired) => return Err(Error::PayerRequired),
            Some(Result::InvoiceError) => {
                submit_result.invoice_errors = Some(res.invoice_errors);
            }
            Some(Result::Failed) => {
                submit_result.errors = Some(TransactionErrors::from_solana_tx(
                    tx,
                    &res.transaction_error.unwrap(),
                    tx_id,
                ));
            }
            None => panic!("Unexpected result from Agora: {}.", res.result),
        }

        Ok(submit_result)
    }
}
