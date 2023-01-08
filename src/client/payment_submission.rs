use {
    crate::{
        client::{
            account_resolution::AccountResolution,
            client::Client,
            internal::transaction::SubmitTransactionResult,
            {get_signers_and_funder, get_subsidizer_from_config, kin_memo_instruction},
        },
        error::{Error, TransactionError},
        gen::kin::agora::{common::v3 as model_pb_v3, transaction::v4 as tx_pb},
        key::{private::PrivateKey, public::PublicKey},
        model::{invoice::InvoiceList, payment::Payment},
        solana::{
            commitment::Commitment,
            memo::program::{MemoParams, MemoProgram},
            token::{
                instruction::{set_close_authority, set_owner_authority},
                program::ACCOUNT_LEN,
            },
        },
    },
    solana_sdk::{
        instruction::Instruction, pubkey::Pubkey as SolanaPublicKey,
        system_instruction::create_account, transaction::Transaction as SolanaTransaction,
    },
    spl_token::instruction::initialize_account,
};

type Result<T> = std::result::Result<T, Error>;

pub async fn submit_payment(
    client: &mut Client,
    payment: &mut Payment,
    commitment: Option<Commitment>,
    sender_resolution: Option<AccountResolution>,
    destination_resolution: Option<AccountResolution>,
    create_destination_token_account: bool,
) -> Result<Option<Vec<u8>>> {
    if payment.invoice.is_some() && client.app_index == 0 {
        panic!("App index required to use invoices.");
    }

    let result = submit_payment_with_resolution(
        client,
        payment,
        commitment,
        sender_resolution.unwrap_or_default(),
        destination_resolution.unwrap_or_default(),
        create_destination_token_account,
    )
    .await?;

    handle_submit_payment_result(result)
}

fn handle_submit_payment_result(result: SubmitTransactionResult) -> Result<Option<Vec<u8>>> {
    if let Some(errors) = result.errors {
        if let Some(payment_errors) = errors.payment_errors {
            if payment_errors.len() != 1 {
                panic!("Invalid number of payment errors, expected 0 or 1. Found: {}. Payment errors: {:?}", payment_errors.len(), payment_errors);
            }
            if let Some(e) = &payment_errors[0] {
                return Err(e.clone().into());
            }
        }

        if let Some(e) = errors.tx_error {
            return Err(e.into());
        }
    }

    if let Some(invoice_errors) = result.invoice_errors {
        if !invoice_errors.is_empty() {
            if invoice_errors.len() != 1 {
                panic!("Invalid number of invoice errors, expected 0 or 1.");
            }
            return Err(Error::from_invoice_error(&invoice_errors[0]));
        }
    }

    Ok(result.tx_id)
}

async fn submit_payment_with_resolution(
    client: &mut Client,
    payment: &mut Payment,
    commitment: Option<Commitment>,
    sender_resolution: AccountResolution,
    destination_resolution: AccountResolution,
    create_destination_token_account: bool,
) -> Result<SubmitTransactionResult> {
    let config = client.internal.tx.get_service_config().await;
    let mint = SolanaPublicKey::new(&config.token.as_ref().unwrap().value);
    let funder = match payment.subsidizer {
        Some(s) => s.public_key().to_solana_key(),
        None => get_subsidizer_from_config(&config)?,
    };

    let mut result =
        submit_payment_tx(client, payment, &config, commitment, None, None, None).await?;

    if let Some(errors) = &result.errors {
        if let Some(TransactionError::AccountDoesNotExist(_)) = &errors.tx_error {
            let mut transfer_sender: Option<PublicKey> = None;
            let mut resubmit = false;
            let mut create_instructions: Option<Vec<Instruction>> = None;
            let mut create_signer: Option<PrivateKey> = None;

            if sender_resolution == AccountResolution::Preferred {
                let accounts = client
                    .resolve_token_accounts(&payment.sender.public_key())
                    .await;
                if !accounts.is_empty() {
                    transfer_sender = Some(accounts[0]);
                    resubmit = true;
                }
            }
            if destination_resolution == AccountResolution::Preferred {
                let accounts = client.resolve_token_accounts(&payment.destination).await;
                if !accounts.is_empty() {
                    payment.destination = accounts[0];
                    resubmit = true;
                } else if create_destination_token_account {
                    let temp_owner = PrivateKey::rand();
                    let new_dest = temp_owner.public_key().to_solana_key();
                    let original_dest = payment.destination.to_solana_key();
                    let new_owner = original_dest;

                    create_instructions = Some(
                        create_account_and_pass_ownership(
                            client, &new_dest, &funder, &mint, &new_owner,
                        )
                        .await,
                    );

                    let new_dest = temp_owner.public_key();
                    payment.destination = new_dest;
                    create_signer = Some(temp_owner);
                    resubmit = true;
                }
            }

            if resubmit {
                result = submit_payment_tx(
                    client,
                    payment,
                    &config,
                    commitment,
                    transfer_sender.as_ref(),
                    create_instructions.as_mut(),
                    create_signer.as_ref(),
                )
                .await?;
            }
        }
    }

    Ok(result)
}

async fn create_account_and_pass_ownership(
    client: &mut Client,
    account: &SolanaPublicKey,
    funder: &SolanaPublicKey,
    mint: &SolanaPublicKey,
    new_owner: &SolanaPublicKey,
) -> Vec<Instruction> {
    let lamports = client
        .internal
        .tx
        .get_minimum_balance_for_rent_exemption()
        .await;

    vec![
        create_account(funder, account, lamports, ACCOUNT_LEN, &spl_token::ID),
        initialize_account(&spl_token::ID, account, mint, account).unwrap(),
        set_close_authority(account, funder, account),
        set_owner_authority(account, new_owner, account),
    ]
}

async fn submit_payment_tx(
    client: &mut Client,
    payment: &Payment,
    config: &tx_pb::GetServiceConfigResponse,
    commitment: Option<Commitment>,
    transfer_sender: Option<&PublicKey>,
    create_instructions: Option<&mut Vec<Instruction>>,
    create_signer: Option<&PrivateKey>,
) -> Result<SubmitTransactionResult> {
    let (mut signers, funder) =
        get_signers_and_funder(&payment.sender, payment.subsidizer.as_ref(), config)?;

    if let Some(c) = create_signer {
        signers.push(c);
    }

    let mut instructions = Vec::new();
    let mut invoice_list_proto: Option<model_pb_v3::InvoiceList> = None;

    if let Some(string_memo) = &payment.memo {
        instructions.push(MemoProgram::memo(MemoParams::new(string_memo.to_string())))
    } else if client.app_index > 0 {
        let mut foreign_key: Vec<u8> = [0; 29].to_vec();

        if let Some(i) = &payment.invoice {
            let il = InvoiceList::new(&[i.clone()]);
            foreign_key = il.get_sha244_hash();
            invoice_list_proto = Some(il.to_proto());
        }

        instructions.push(kin_memo_instruction(
            payment.tx_type,
            client.app_index,
            &foreign_key,
        ));
    }

    if let Some(i) = create_instructions {
        instructions.append(i);
    }

    let sender = match transfer_sender {
        Some(t) => *t,
        None => payment.sender.public_key(),
    };

    instructions.push(transfer_instruction(&sender, payment));

    let mut tx = SolanaTransaction::new_with_payer(&instructions, Some(&funder));

    client
        .sign_and_submit_tx(
            &signers,
            &mut tx,
            commitment,
            invoice_list_proto.as_ref(),
            payment.dedupe_id.as_ref(),
        )
        .await
}

fn transfer_instruction(sender: &PublicKey, payment: &Payment) -> Instruction {
    spl_token::instruction::transfer(
        &spl_token::id(),
        &sender.to_solana_key(),
        &payment.destination.to_solana_key(),
        &payment.sender.public_key().to_solana_key(),
        &[],
        payment.quarks,
    )
    .unwrap()
}
