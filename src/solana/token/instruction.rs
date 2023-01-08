use {
    solana_sdk::{instruction::Instruction, pubkey::Pubkey as SolanaPublicKey},
    spl_associated_token_account::create_associated_token_account,
    spl_token::instruction::{set_authority, AuthorityType},
};

pub fn create_assoc_account_and_set_close_auth(
    funder: &SolanaPublicKey,
    owner: &SolanaPublicKey,
    mint: &SolanaPublicKey,
    assoc: &SolanaPublicKey,
) -> Vec<Instruction> {
    let mut instructions = vec![create_associated_token_account(funder, owner, mint)];

    instructions.push(set_close_authority(assoc, funder, owner));

    instructions
}

pub fn set_close_authority(
    owned: &SolanaPublicKey,
    new_auth: &SolanaPublicKey,
    owner: &SolanaPublicKey,
) -> Instruction {
    set_authority(
        &spl_token::id(),
        owned,
        Some(new_auth),
        AuthorityType::CloseAccount,
        owner,
        &[],
    )
    .unwrap()
}

pub fn set_owner_authority(
    owned: &SolanaPublicKey,
    new_auth: &SolanaPublicKey,
    owner: &SolanaPublicKey,
) -> Instruction {
    set_authority(
        &spl_token::ID,
        owned,
        Some(new_auth),
        AuthorityType::AccountOwner,
        owner,
        &[],
    )
    .unwrap()
}
