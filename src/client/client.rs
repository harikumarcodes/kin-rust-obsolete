use {
    crate::{
        client::{
            account_resolution::AccountResolution,
            environment::Environment,
            internal::{
                account::InternalAccountClient,
                airdrop::InternalAirdropClient,
                transaction::{InternalTransactionClient, SubmitTransactionResult},
                InternalClient,
            },
            {get_signers_and_funder, partial_sign},
        },
        error::{Error, TransactionError},
        gen::kin::agora::account::v4 as account_pb,
        gen::kin::agora::common::v3 as model_pb_v3,
        key::{private::PrivateKey, public::PublicKey},
        model::{payment::Payment, transaction::TransactionData},
        solana::{
            commitment::Commitment, token::instruction::create_assoc_account_and_set_close_auth,
        },
    },
    solana_sdk::{
        instruction::Instruction,
        pubkey::Pubkey as SolanaPublicKey,
        signature::{Signature, SIGNATURE_BYTES},
        transaction::Transaction as SolanaTransaction,
    },
    spl_associated_token_account::get_associated_token_address,
    std::convert::TryInto,
};

type Result<T> = std::result::Result<T, Error>;

pub mod endpoint {
    pub const PRODUCTION: &str = "https://api.agorainfra.net:443";
    pub const TEST: &str = "https://api.agorainfra.dev:443";
}

/// An interface for accessing Agora features.
pub struct Client {
    pub internal: InternalClient,
    pub app_index: u16,
    pub env: Environment,
}

impl Client {
    pub async fn new(env: Environment, app_index: Option<u16>) -> Client {
        let app_index = app_index.unwrap_or_default();

        let endpoint = match env {
            Environment::Production => endpoint::PRODUCTION,
            Environment::Test => endpoint::TEST,
        };

        let account = InternalAccountClient::new(endpoint).await.unwrap();
        let tx = InternalTransactionClient::new(endpoint).await.unwrap();
        let airdrop = InternalAirdropClient::new(endpoint).await.unwrap();

        Client {
            internal: InternalClient {
                account,
                tx,
                airdrop,
            },
            app_index,
            env,
        }
    }

    /// Creates a new Kin account.
    pub async fn create_account(
        &mut self,
        key: &PrivateKey,
        commitment: Option<Commitment>,
        subsidizer: Option<&PrivateKey>,
    ) -> Result<()> {
        let config = self.internal.tx.get_service_config().await;
        let hash = self.internal.tx.get_recent_blockhash().await;

        self.internal
            .account
            .create_account(
                key,
                commitment.unwrap_or_default(),
                self.app_index,
                subsidizer,
                &config,
                &hash,
            )
            .await?;

        Ok(())
    }

    /// Resolves the token accounts owned by the specified account on kin 4.
    pub async fn resolve_token_accounts(&mut self, account: &PublicKey) -> Vec<PublicKey> {
        let account_infos = self
            .internal
            .account
            .resolve_token_accounts(account, false)
            .await;

        let mut accounts = Vec::new();
        for info in account_infos {
            if let Some(id) = info.account_id {
                accounts.push(PublicKey::new(&id.value));
            }
        }

        accounts
    }

    /// Merges all of an account's token accounts into one.
    pub async fn merge_token_accounts(
        &mut self,
        key: &PrivateKey,
        create_associated_account: bool,
        commitment: Option<Commitment>,
        subsidizer: Option<&PrivateKey>,
    ) -> Result<Option<Vec<u8>>> {
        let accounts = self
            .internal
            .account
            .resolve_token_accounts(&key.public_key(), true)
            .await;

        if !Self::enough_accounts_for_merge(accounts.len(), create_associated_account) {
            return Ok(None);
        }

        let config = self.internal.tx.get_service_config().await;
        let (signers, funder) = get_signers_and_funder(key, subsidizer, &config)?;

        let mut dest = SolanaPublicKey::new(&accounts[0].account_id.as_ref().unwrap().value);
        let owner = key.public_key().to_solana_key();
        let mint = SolanaPublicKey::new(&config.token.as_ref().unwrap().value);

        let mut instructions = Vec::new();
        if create_associated_account {
            let assoc = get_associated_token_address(&owner, &mint);

            if dest.ne(&assoc) {
                instructions.append(&mut create_assoc_account_and_set_close_auth(
                    &funder, &owner, &mint, &assoc,
                ));

                dest = assoc;
            } else if accounts.len() == 1 {
                return Ok(None);
            }
        }

        instructions.append(&mut Self::get_merge_instructions(
            &accounts, &dest, &owner, &funder,
        ));

        let tx = &mut SolanaTransaction::new_with_payer(&instructions, Some(&funder));

        let result = self
            .sign_and_submit_tx(&signers, tx, commitment, None, None)
            .await?;

        Ok(result.tx_id)
    }

    /// Retrieves the balance for an account.
    pub async fn get_balance(
        &mut self,
        account: &PublicKey,
        commitment: Option<Commitment>,
        account_resolution: Option<AccountResolution>,
    ) -> Result<i64> {
        let account_info = self
            .internal
            .account
            .get_account_info(account, commitment.unwrap_or_default())
            .await;
        let resolution = account_resolution.unwrap_or_default();

        match account_info {
            Ok(info) => Ok(info.balance),
            Err(err) => match err {
                TransactionError::AccountDoesNotExist(_) => {
                    if resolution == AccountResolution::Preferred {
                        let account_infos = self
                            .internal
                            .account
                            .resolve_token_accounts(account, true)
                            .await;
                        if !account_infos.is_empty() {
                            return Ok(account_infos[0].balance);
                        }
                    }

                    Err(err.into())
                }
                _ => Err(err.into()),
            },
        }
    }

    /// Retrieves the TransactionData for a transaction id.
    pub async fn get_transaction(
        &mut self,
        tx_id: &[u8],
        commitment: Option<Commitment>,
    ) -> TransactionData {
        self.internal.tx.get_transaction(tx_id, commitment).await
    }

    /// Submits a payment.
    ///
    /// If the payment has an invoice, an app index _must_ be set.
    /// If the payment has a memo, an invoice cannot also be provided.
    pub async fn submit_payment(
        &mut self,
        mut payment: Payment,
        commitment: Option<Commitment>,
        sender_resolution: Option<AccountResolution>,
        destination_resolution: Option<AccountResolution>,
        sender_create: Option<bool>,
    ) -> Result<Option<Vec<u8>>> {
        use crate::client::payment_submission::submit_payment;

        submit_payment(
            self,
            &mut payment,
            commitment,
            sender_resolution,
            destination_resolution,
            sender_create.unwrap_or_default(),
        )
        .await
    }

    /// Requests an airdrop of Kin to a Kin account.
    /// Only available on Kin 4 on the test environment.
    pub async fn request_airdrop(
        &mut self,
        public_key: &PublicKey,
        quarks: u64,
        commitment: Option<Commitment>,
    ) -> Result<Vec<u8>> {
        self.internal
            .airdrop
            .request_airdrop(public_key, quarks, commitment.unwrap_or_default())
            .await
    }

    pub async fn sign_and_submit_tx(
        &mut self,
        signers: &[&PrivateKey],
        tx: &mut SolanaTransaction,
        commitment: Option<Commitment>,
        invoice_list: Option<&model_pb_v3::InvoiceList>,
        dedupe_id: Option<&Vec<u8>>,
    ) -> Result<SubmitTransactionResult> {
        let hash = self.internal.tx.get_recent_blockhash().await;
        partial_sign(tx, signers, &hash);

        let mut remote_signed = false;
        if Self::needs_subsidizer_signature(tx) {
            let result = self.internal.tx.sign_transaction(tx, invoice_list).await;

            // Error
            if result.invoice_errors.is_some() {
                return Ok(SubmitTransactionResult {
                    tx_id: None,
                    invoice_errors: result.invoice_errors,
                    errors: None,
                });
            }
            if result.transaction_id.is_none() {
                return Err(Error::PayerRequired);
            }

            // Success
            if let Some(id) = result.transaction_id {
                remote_signed = true;
                tx.signatures[0] = Signature::new(&id);
            }
        }

        let result = self
            .internal
            .tx
            .submit_transaction(tx, invoice_list, commitment, dedupe_id)
            .await?;

        if let Some(errors) = &result.errors {
            if let Some(TransactionError::BadNonce(_)) = &errors.tx_error {
                if remote_signed {
                    tx.signatures[0] = Signature::new(&[0; SIGNATURE_BYTES]);
                }

                return Err(TransactionError::BadNonce(None).into());
            }
        }

        Ok(result)
    }

    fn needs_subsidizer_signature(tx: &SolanaTransaction) -> bool {
        tx.signatures[0].as_ref() == [0; SIGNATURE_BYTES]
    }

    /// Returns true if there are, or there will be, enough token accounts for a merge.
    fn enough_accounts_for_merge(
        token_accounts_len: usize,
        create_associated_account: bool,
    ) -> bool {
        let enough_accounts = token_accounts_len > 1;
        let will_have_enough_accounts = token_accounts_len == 1 && create_associated_account;

        enough_accounts || will_have_enough_accounts
    }

    fn get_merge_instructions(
        account_infos: &[account_pb::AccountInfo],
        dest: &SolanaPublicKey,
        owner: &SolanaPublicKey,
        funder: &SolanaPublicKey,
    ) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        for info in account_infos {
            let account = SolanaPublicKey::new(&info.account_id.as_ref().unwrap().value);

            if account.eq(dest) {
                continue;
            }

            instructions.push(
                spl_token::instruction::transfer(
                    &spl_token::id(),
                    &account,
                    dest,
                    owner,
                    &[],
                    info.balance.try_into().unwrap(),
                )
                .unwrap(),
            );

            // If no close authority is set, it likely means we
            // do not know it and can't make any assumptions.
            if info.close_authority.is_none() {
                continue;
            }

            let mut should_close = false;
            let close_auth = SolanaPublicKey::new(&info.close_authority.as_ref().unwrap().value);
            for auth in [owner, funder] {
                if auth.eq(&close_auth) {
                    should_close = true;
                    break;
                }
            }

            if should_close {
                instructions.push(
                    spl_token::instruction::close_account(
                        &spl_token::id(),
                        &account,
                        &close_auth,
                        &close_auth,
                        &[],
                    )
                    .unwrap(),
                );
            }
        }

        instructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::kin_to_quarks;
    use crate::{
        client::{account_resolution::AccountResolution, environment::Environment},
        key::private::PrivateKey,
        model::{invoice::Invoice, transaction_type::TransactionType},
    };

    #[tokio::test]
    async fn create_account() {
        let key = PrivateKey::rand();
        let mut client = Client::new(Environment::Test, Some(2)).await;

        client.create_account(&key, None, None).await.unwrap();
    }

    #[tokio::test]
    async fn get_balance_with_resolution() {
        let key = PrivateKey::rand();
        let mut client = Client::new(Environment::Test, Some(2)).await;

        client.create_account(&key, None, None).await.unwrap();

        let balance = client
            .get_balance(&key.public_key(), None, Some(AccountResolution::Preferred))
            .await
            .unwrap();
        assert_eq!(balance, 0);
    }

    #[tokio::test]
    async fn airdrop() {
        let key = PrivateKey::rand();
        let mut client = Client::new(Environment::Test, Some(2)).await;

        client.create_account(&key, None, None).await.unwrap();
        let accounts = client.resolve_token_accounts(&key.public_key()).await;

        let kin = "1234.555";
        client
            .request_airdrop(&accounts[0], kin_to_quarks(kin), None)
            .await
            .unwrap();

        let balance = client
            .get_balance(&key.public_key(), None, None)
            .await
            .unwrap();
        assert_eq!(balance, kin_to_quarks(kin) as i64);
    }

    #[tokio::test]
    async fn submit_payment() {
        let mut client = Client::new(Environment::Test, Some(100)).await;
        let sender = PrivateKey::rand();
        let dest = PrivateKey::rand();

        client.create_account(&sender, None, None).await.unwrap();
        client.create_account(&dest, None, None).await.unwrap();

        let sender_accounts = client.resolve_token_accounts(&sender.public_key()).await;
        let airdrop_kin = "10";
        client
            .request_airdrop(&sender_accounts[0], kin_to_quarks(airdrop_kin), None)
            .await
            .unwrap();

        let mut payment = Payment::new(
            sender,
            dest.public_key(),
            TransactionType::Spend,
            kin_to_quarks("4"),
        );
        payment.set_invoice(Invoice::new(
            "TestPayment",
            payment.quarks.try_into().unwrap(),
            None,
            None,
        ));
        let _tx_id = client
            .submit_payment(payment, None, None, None, None)
            .await
            .unwrap()
            .unwrap_or_default();

        let sender_balance = client
            .get_balance(&sender.public_key(), None, None)
            .await
            .unwrap();
        let dest_balance = client
            .get_balance(&dest.public_key(), None, None)
            .await
            .unwrap();
        assert_eq!(sender_balance, kin_to_quarks("6") as i64);
        assert_eq!(dest_balance, kin_to_quarks("4") as i64);
    }
}
