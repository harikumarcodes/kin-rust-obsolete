use {
    crate::{
        client::{get_signers_and_funder, kin_memo_instruction, partial_sign, proto_tx},
        error::{Error, TransactionError},
        gen::kin::agora::{
            account::v4 as account_pb, common::v4 as model_pb_v4, transaction::v4 as tx_pb,
        },
        key::{private::PrivateKey, public::PublicKey},
        model::transaction_type::TransactionType,
        solana::{
            commitment::Commitment, token::instruction::create_assoc_account_and_set_close_auth,
        },
    },
    solana_sdk::instruction::Instruction,
    solana_sdk::{
        pubkey::Pubkey as SolanaPublicKey, transaction::Transaction as SolanaTransaction,
    },
    spl_associated_token_account::get_associated_token_address,
    tonic::transport::{Channel, ClientTlsConfig, Error as TonicError},
};

pub struct InternalAccountClient {
    client: account_pb::account_client::AccountClient<Channel>,
}

impl InternalAccountClient {
    pub async fn new(endpoint: &'static str) -> Result<InternalAccountClient, TonicError> {
        let tls = ClientTlsConfig::new();
        let channel = Channel::from_static(endpoint)
            .tls_config(tls)?
            .connect()
            .await?;

        let account_client = InternalAccountClient {
            client: account_pb::account_client::AccountClient::new(channel),
        };

        Ok(account_client)
    }

    pub async fn create_account(
        &mut self,
        key: &PrivateKey,
        commitment: Commitment,
        app_index: u16,
        subsidizer: Option<&PrivateKey>,
        config: &tx_pb::GetServiceConfigResponse,
        recent_blockhash: &[u8],
    ) -> Result<(), Error> {
        use account_pb::create_account_response::Result;

        let (signers, funder) = get_signers_and_funder(key, subsidizer, config)?;
        let instructions = Self::get_create_account_instructions(key, app_index, &funder, config);

        let mut tx = SolanaTransaction::new_with_payer(&instructions, Some(&funder));
        partial_sign(&mut tx, &signers, recent_blockhash);

        let req = account_pb::CreateAccountRequest {
            transaction: Some(proto_tx(&tx)),
            commitment: commitment as i32,
        };

        let res = self.client.create_account(req).await.unwrap().into_inner();
        match Result::from_i32(res.result) {
            Some(Result::Ok) => Ok(()),
            Some(Result::Exists) => Err(Error::AccountExists),
            Some(Result::PayerRequired) => Err(Error::PayerRequired),
            Some(Result::BadNonce) => Err(TransactionError::BadNonce(None).into()),
            None => panic!("Unexpected result from Agora: {}.", res.result),
        }
    }

    #[allow(deprecated)]
    pub async fn resolve_token_accounts(
        &mut self,
        public_key: &PublicKey,
        include_account_info: bool,
    ) -> Vec<account_pb::AccountInfo> {
        let id = model_pb_v4::SolanaAccountId {
            value: public_key.to_bytes().to_vec(),
        };

        let req = account_pb::ResolveTokenAccountsRequest {
            account_id: Some(id),
            include_account_info,
        };

        let res = self
            .client
            .resolve_token_accounts(req)
            .await
            .unwrap()
            .into_inner();

        let token_accounts = res.token_accounts;
        let infos = res.token_account_infos;

        // This is currently in place for backward compat with the server - `token_accounts` is deprecated.
        if !token_accounts.is_empty() && infos.len() != token_accounts.len() {
            // If we aren't requesting account info, we can interpolate the results ourselves.
            if !include_account_info {
                return infos.iter().map(Self::to_info_with_id_only).collect();
            } else {
                panic!("Server does not support resolving with account info.")
            }
        }

        infos
    }

    pub async fn get_account_info(
        &mut self,
        public_key: &PublicKey,
        commitment: Commitment,
    ) -> Result<account_pb::AccountInfo, TransactionError> {
        use account_pb::get_account_info_response::Result;

        let account_id = model_pb_v4::SolanaAccountId {
            value: public_key.to_bytes().to_vec(),
        };

        let req = account_pb::GetAccountInfoRequest {
            account_id: Some(account_id),
            commitment: commitment as i32,
        };

        let res = self
            .client
            .get_account_info(req)
            .await
            .unwrap()
            .into_inner();

        match Result::from_i32(res.result) {
            Some(Result::Ok) => Ok(res.account_info.unwrap()),
            Some(Result::NotFound) => Err(TransactionError::AccountDoesNotExist(None)),
            None => panic!("Unexpected result from Agora: {}.", res.result),
        }
    }

    fn to_info_with_id_only(info: &account_pb::AccountInfo) -> account_pb::AccountInfo {
        account_pb::AccountInfo {
            account_id: info.account_id.clone(),
            balance: 0,
            owner: None,
            close_authority: None,
        }
    }

    fn get_create_account_instructions(
        key: &PrivateKey,
        app_index: u16,
        funder: &SolanaPublicKey,
        config: &tx_pb::GetServiceConfigResponse,
    ) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        if app_index > 0 {
            instructions.push(kin_memo_instruction(
                TransactionType::None,
                app_index,
                &[0; 29],
            ));
        }

        let owner = key.public_key().to_solana_key();
        let mint = SolanaPublicKey::new(&config.token.as_ref().unwrap().value);
        let assoc = get_associated_token_address(&owner, &mint);
        instructions.append(&mut create_assoc_account_and_set_close_auth(
            funder, &owner, &mint, &assoc,
        ));

        instructions
    }
}
