pub mod account_resolution;
pub mod client;
pub mod environment;
pub mod internal;
pub mod payment_submission;

use {
    crate::{
        error::Error,
        gen::kin::agora::common::v4 as model_pb_v4,
        gen::kin::agora::transaction::v4 as tx_pb,
        key::private::PrivateKey,
        model::memo::Memo,
        model::transaction_type::TransactionType,
        solana::memo::program::{MemoParams, MemoProgram},
    },
    solana_sdk::{
        hash::Hash, instruction::Instruction, pubkey::Pubkey as SolanaPublicKey,
        signer::keypair::Keypair as SolanaKeypair, transaction::Transaction as SolanaTransaction,
    },
};

fn get_signers_and_funder<'a>(
    key: &'a PrivateKey,
    subsidizer: Option<&'a PrivateKey>,
    config: &tx_pb::GetServiceConfigResponse,
) -> Result<(Vec<&'a PrivateKey>, SolanaPublicKey), Error> {
    let mut signers = vec![key];

    let funder;
    if let Some(s) = subsidizer {
        funder = s.public_key().to_solana_key();
        signers.push(s);
    } else {
        funder = get_subsidizer_from_config(config)?;
    }

    Ok((signers, funder))
}

fn get_subsidizer_from_config(
    config: &tx_pb::GetServiceConfigResponse,
) -> Result<SolanaPublicKey, Error> {
    match &config.subsidizer_account {
        Some(account) => Ok(SolanaPublicKey::new(&account.value)),
        None => Err(Error::NoSubsidizer),
    }
}

fn partial_sign(tx: &mut SolanaTransaction, signers: &[&PrivateKey], recent_blockhash: &[u8]) {
    let keypairs: Vec<SolanaKeypair> = signers
        .iter()
        .map(|s| SolanaKeypair::from_bytes(&s.secret_key()).unwrap())
        .collect();
    let keypair_refs: Vec<&SolanaKeypair> = keypairs.iter().collect();
    tx.partial_sign(&keypair_refs, Hash::new(recent_blockhash));
}

fn kin_memo_instruction(
    tx_type: TransactionType,
    app_index: u16,
    foreign_key: &[u8],
) -> Instruction {
    let kin_memo = Memo::new(1, tx_type, app_index, foreign_key);

    MemoProgram::memo(MemoParams {
        data: kin_memo.to_base64(),
    })
}

fn proto_tx(tx: &SolanaTransaction) -> model_pb_v4::Transaction {
    model_pb_v4::Transaction {
        value: bincode::serialize(tx).unwrap(),
    }
}
