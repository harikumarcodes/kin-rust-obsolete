use {solana_sdk::message::Message, spl_token};

/// Reference: https://docs.rs/spl-token/3.2.0/src/spl_token/state.rs.html#124.
pub const ACCOUNT_LEN: u64 = 165;

mod instructions {
    pub mod transfer {
        pub const COMMAND: u8 = 3;
        pub const ACCOUNTS_LEN: usize = 3;
        pub const DATA_LEN: usize = 9;
    }
}

/// Returns true if the instruction at the given index
/// is a tranfer instruction, and false otherwise.
pub fn is_transfer(msg: &Message, instruction_index: usize) -> bool {
    use instructions::transfer;

    let instruction = &msg.instructions[instruction_index];
    let program_id = msg.program_id(instruction_index).unwrap();

    program_id.eq(&spl_token::id())
        && instruction.accounts.len() == transfer::ACCOUNTS_LEN
        && instruction.data.len() == transfer::DATA_LEN
        && instruction.data[0] == transfer::COMMAND
}
