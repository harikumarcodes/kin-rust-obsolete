use {
    solana_sdk::{
        instruction::{CompiledInstruction, Instruction},
        pubkey::Pubkey as SolanaPublicKey,
    },
    std::str::FromStr,
};

pub struct MemoParams {
    pub data: String,
}

impl MemoParams {
    /// Returns memo params from the given data.
    pub fn new(data: String) -> MemoParams {
        MemoParams { data }
    }
}

pub struct MemoInstruction;
impl MemoInstruction {
    /// Decode a memo instruction and retrieve the instruction params.
    pub fn decode_memo(instruction: &CompiledInstruction) -> MemoParams {
        MemoParams {
            data: String::from_utf8(instruction.data.clone()).unwrap(),
        }
    }

    /// Confirms that a given Solana public key is equivalent to the program ID.
    pub fn check_program_id(id: &SolanaPublicKey) {
        if !id.eq(&MemoProgram::id()) {
            panic!("Invalid instruction: programId did not match that of MemoProgram.")
        }
    }
}

pub struct MemoProgram;
impl MemoProgram {
    /// The address of the memo program that should be used.
    pub fn id() -> SolanaPublicKey {
        SolanaPublicKey::from_str("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo").unwrap()
    }

    pub fn memo(params: MemoParams) -> Instruction {
        Instruction {
            program_id: MemoProgram::id(),
            data: params.data.as_bytes().to_vec(),
            accounts: Vec::new(),
        }
    }
}
