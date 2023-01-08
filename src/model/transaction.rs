use {
    crate::{
        error::TransactionErrors,
        gen::kin::agora::transaction::v4 as tx_pb,
        key::public::PublicKey,
        model::{
            invoice::Invoice, memo::Memo, payment::ReadOnlyPayment,
            transaction_type::TransactionType,
        },
        solana::memo::program::{MemoInstruction, MemoProgram},
    },
    bincode,
    num_derive::FromPrimitive,
    num_traits::FromPrimitive,
    solana_sdk::transaction::Transaction as SolanaTransaction,
    substrate_stellar_sdk::{types::TransactionV0Envelope, Memo as StellarMemo, XdrCodec},
};

#[derive(Debug, PartialEq, Eq, FromPrimitive)]
pub enum TransactionState {
    Unknown = 0,
    Success = 1,
    Failed = 2,
    Pending = 3,
}

impl TransactionState {
    /// Converts i32 to TransactionState.
    pub fn from_i32(value: i32) -> TransactionState {
        if let Some(tx_state) = FromPrimitive::from_i32(value) {
            tx_state
        } else {
            TransactionState::Unknown
        }
    }

    pub fn from_proto(state: tx_pb::get_transaction_response::State) -> TransactionState {
        TransactionState::from_i32(state as i32)
    }
}

/// Contains both metadata and payment data related to a blockchain transaction.
#[derive(Debug)]
pub struct TransactionData {
    pub tx_id: Vec<u8>,
    pub tx_state: TransactionState,
    pub payments: Vec<ReadOnlyPayment>,
    pub errors: Option<TransactionErrors>,
}

impl TransactionData {
    /// Returns TransactionData object from provided id and state.
    pub fn new(tx_id: Vec<u8>, tx_state: TransactionState) -> TransactionData {
        TransactionData {
            tx_id,
            tx_state,
            payments: Vec::new(),
            errors: None,
        }
    }

    /// Returns TransactionData object from provided history item and state.
    pub fn from_proto(
        item: &tx_pb::HistoryItem,
        state: tx_pb::get_transaction_response::State,
    ) -> TransactionData {
        if let Some(invoice_list) = &item.invoice_list {
            if invoice_list.invoices.len() != item.payments.len() {
                panic!("Number of invoices does not match number of payments.");
            }
        }

        let mut tx_type = TransactionType::Unknown;
        let mut string_memo: Option<String> = None;
        let mut errors: Option<TransactionErrors> = None;

        if let Some(raw_tx) = &item.raw_transaction {
            match raw_tx {
                tx_pb::history_item::RawTransaction::SolanaTransaction(tx) => {
                    let solana_tx: SolanaTransaction = bincode::deserialize(&tx.value).unwrap();

                    // Memo.
                    let program_id = solana_tx.message().program_id(0);
                    if let Some(program_id) = program_id {
                        if program_id.eq(&MemoProgram::id()) {
                            let memo_params =
                                MemoInstruction::decode_memo(&solana_tx.message().instructions[0]);
                            let agora_memo = Memo::from_base64(&memo_params.data, false);
                            match agora_memo {
                                Ok(memo) => tx_type = memo.tx_type(),
                                _ => {
                                    // Not a valid agora memo.
                                    string_memo = Some(memo_params.data);
                                }
                            }
                        }
                    }

                    // Errors.
                    if let Some(tx_error) = &item.transaction_error {
                        errors = Some(TransactionErrors::from_solana_tx(
                            &solana_tx, tx_error, None,
                        ));
                    }
                }
                tx_pb::history_item::RawTransaction::StellarTransaction(tx) => {
                    let envelope =
                        TransactionV0Envelope::from_xdr(tx.envelope_xdr.clone()).unwrap();
                    let stellar_memo = &envelope.tx.memo;
                    let agora_memo = Memo::from_stellar(stellar_memo, true);

                    match agora_memo {
                        Some(agora_memo) => tx_type = agora_memo.tx_type(),
                        None => {
                            if let StellarMemo::MemoText(text) = stellar_memo {
                                let text_bytes = text.get_vec().to_vec();
                                string_memo = Some(String::from_utf8(text_bytes).unwrap());
                            }
                        }
                    }

                    if let Some(tx_error) = &item.transaction_error {
                        errors = Some(TransactionErrors::from_stellar_tx(&envelope, tx_error));
                    }
                }
            }
        }

        // Payments.
        let mut payments: Vec<ReadOnlyPayment> = Vec::new();
        for (i, payment) in item.payments.iter().enumerate() {
            let source_key = match &payment.source {
                Some(source) => PublicKey::new(&source.value),
                None => panic!("No sender."),
            };

            let destination_key = match &payment.destination {
                Some(destination) => PublicKey::new(&destination.value),
                None => panic!("No destination."),
            };

            let mut read_only_payment =
                ReadOnlyPayment::new(source_key, destination_key, tx_type, payment.amount);

            // Set Invoice or Memo, if any.
            if let Some(invoice_list) = &item.invoice_list {
                let invoice = Invoice::from_proto(invoice_list.invoices[i].clone());
                read_only_payment.invoice = Some(invoice);
            } else {
                read_only_payment.memo = string_memo.clone();
            }

            payments.push(read_only_payment);
        }

        let tx_id = item.transaction_id.as_ref().unwrap().value.clone();
        let tx_state = TransactionState::from_proto(state);

        TransactionData {
            tx_id,
            tx_state,
            payments,
            errors,
        }
    }
}
