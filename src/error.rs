use {
    crate::{
        gen::kin::agora::common::v3 as model_pb_v3, gen::kin::agora::common::v4 as model_pb_v4,
        solana::token::program::is_transfer,
    },
    solana_sdk::transaction::Transaction as SolanaTransaction,
    substrate_stellar_sdk::types::{OperationBody, TransactionV0Envelope},
    thiserror::Error as ThisError,
};

/// Base Error for Agora SDK errors.
#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Transaction Failed: {0}")]
    TransactionFailed(TransactionError),

    #[error("Account already exists.")]
    AccountExists,

    #[error("Transaction data could not be found.")]
    TransactionNotFound,

    #[error("Malformed")]
    Malformed,

    #[error("Insufficient fee.")]
    InsufficientFee,

    #[error("Sender does not exist.")]
    SenderDoesNotExist,

    #[error("Destination does not exist.")]
    DestinationDoesNotExist,

    #[error("Invoice has already been paid.")]
    AlreadyPaid,

    #[error("Transaction rejected by app webhook for having wrong destination.")]
    WrongDestination,

    #[error("Invoice contains a SKU that could not be found.")]
    SkuNotFound,

    #[error("Transaction rejected by a configured webhook.")]
    TransactionRejected,

    #[error("Transaction missing signature from funder.")]
    PayerRequired,

    #[error("No subsidizer provided for transaction.")]
    NoSubsidizer,

    #[error("No token accounts resolved for requested account ID.")]
    NoTokenAccounts,
}

impl Error {
    /// Returns error from invoice error.
    pub fn from_invoice_error(invoice_error: &model_pb_v3::InvoiceError) -> Error {
        use model_pb_v3::invoice_error::Reason;

        let reason = invoice_error.reason;
        match Reason::from_i32(reason) {
            Some(Reason::AlreadyPaid) => Error::AlreadyPaid,
            Some(Reason::WrongDestination) => Error::WrongDestination,
            Some(Reason::SkuNotFound) => Error::SkuNotFound,
            _ => panic!("Unknown invoice error reason: {}", reason),
        }
    }
}

// TransactionError to Error conversion.
impl From<TransactionError> for Error {
    fn from(tx_error: TransactionError) -> Error {
        Error::TransactionFailed(tx_error)
    }
}

/// Reasons a transaction might be rejected.
#[derive(ThisError, Debug, Clone, Eq, PartialEq)]
pub enum TransactionError {
    #[error("Account does not exist.")]
    AccountDoesNotExist(Option<Vec<u8>>),

    #[error("Bad nonce.")]
    BadNonce(Option<Vec<u8>>),

    #[error("Insufficient balance.")]
    InsufficientBalance(Option<Vec<u8>>),

    #[error("Invalid signature.")]
    InvalidSignature(Option<Vec<u8>>),

    #[error("Already submitted.")]
    AlreadySubmitted(Option<Vec<u8>>),
}

impl TransactionError {
    fn from_proto(
        proto_error: &model_pb_v4::TransactionError,
        tx_id: Option<Vec<u8>>,
    ) -> Option<TransactionError> {
        use model_pb_v4::transaction_error::Reason;

        match Reason::from_i32(proto_error.reason) {
            Some(Reason::None) => None,
            Some(Reason::Unauthorized) => Some(TransactionError::InvalidSignature(tx_id)),
            Some(Reason::BadNonce) => Some(TransactionError::BadNonce(tx_id)),
            Some(Reason::InsufficientFunds) => Some(TransactionError::InsufficientBalance(tx_id)),
            Some(Reason::InvalidAccount) => Some(TransactionError::AccountDoesNotExist(tx_id)),
            _ => panic!("Unknown transaction error reason: {}.", proto_error.reason),
        }
    }
}

type TransactionErrorArray = Vec<Option<TransactionError>>;

/// Contains the details of a failed transaction.
#[derive(Debug, Clone)]
pub struct TransactionErrors {
    /// If present, the transaction failed.
    pub tx_error: Option<TransactionError>,

    /// May be set if tx_error is set.
    ///
    /// If set, length equals number of instructions in the transaction.
    pub op_errors: Option<TransactionErrorArray>,

    /// May be set if tx_error is set.
    ///
    /// If set, length equals number of payments in the transaction.
    pub payment_errors: Option<TransactionErrorArray>,
}

impl TransactionErrors {
    pub fn new() -> TransactionErrors {
        TransactionErrors {
            tx_error: None,
            op_errors: None,
            payment_errors: None,
        }
    }

    pub fn from_solana_tx(
        tx: &SolanaTransaction,
        proto_tx_error: &model_pb_v4::TransactionError,
        tx_id: Option<Vec<u8>>,
    ) -> TransactionErrors {
        let mut tx_errors = TransactionErrors::new();

        let tx_error = TransactionError::from_proto(proto_tx_error, tx_id);
        if tx_error.is_none() {
            return tx_errors;
        }

        tx_errors.tx_error = tx_error.clone();

        let index = proto_tx_error.instruction_index;
        if index >= 0 {
            tx_errors.op_errors =
                tx_error_array(tx.message.instructions.len(), &tx_error, index as usize);
            tx_errors.payment_errors = payment_errors_from_solana_tx(tx, &tx_error, index as usize);
        }

        tx_errors
    }

    pub fn from_stellar_tx(
        envelope: &TransactionV0Envelope,
        proto_tx_error: &model_pb_v4::TransactionError,
    ) -> TransactionErrors {
        let mut tx_errors = TransactionErrors::new();

        let tx_error = TransactionError::from_proto(proto_tx_error, None);
        if tx_error.is_none() {
            return tx_errors;
        }

        tx_errors.tx_error = tx_error.clone();

        let index = proto_tx_error.instruction_index;
        if index >= 0 {
            tx_errors.op_errors =
                tx_error_array(envelope.tx.operations.len(), &tx_error, index as usize);
            tx_errors.payment_errors =
                payment_errors_from_stellar_tx(envelope, &tx_error, index as usize);
        }

        tx_errors
    }
}

/// Returns an empty TransactionErrorArray of size `len`, with `tx_error` inserted at `index`.
fn tx_error_array(
    len: usize,
    tx_error: &Option<TransactionError>,
    index: usize,
) -> Option<TransactionErrorArray> {
    let mut tx_errors = vec![None; len];
    tx_errors[index] = tx_error.clone();

    Some(tx_errors)
}

fn payment_errors_from_solana_tx(
    tx: &SolanaTransaction,
    tx_error: &Option<TransactionError>,
    failed_index: usize,
) -> Option<TransactionErrorArray> {
    let mut payment_index = 0;
    let mut payment_count = 0;
    let mut payment_failed = true;

    let instructions = &tx.message.instructions;
    for i in 0..instructions.len() {
        let at_failed = i == failed_index;

        if is_transfer(&tx.message, i) {
            if at_failed {
                payment_index = payment_count;
            }
            payment_count += 1;
        } else if at_failed {
            payment_failed = false;
        }
    }

    if payment_failed {
        return tx_error_array(payment_count, tx_error, payment_index);
    }

    None
}

fn payment_errors_from_stellar_tx(
    envelope: &TransactionV0Envelope,
    tx_error: &Option<TransactionError>,
    failed_index: usize,
) -> Option<TransactionErrorArray> {
    let mut payment_index = 0;
    let mut payment_count = 0;
    let mut payment_failed = true;

    let ops = &envelope.tx.operations.get_vec();
    for i in 0..ops.len() {
        let at_failed = i == failed_index;

        let op = &ops[i];
        if is_payment(&op.body) {
            if at_failed {
                payment_index = payment_count;
            }
            payment_count += 1;
        } else if at_failed {
            payment_failed = false;
        }
    }

    if payment_failed {
        return tx_error_array(payment_count, tx_error, payment_index);
    }

    None
}

/// Returns true if the variant of the operation body is `Payment`.
fn is_payment(op_body: &OperationBody) -> bool {
    matches!(op_body, OperationBody::Payment(_))
}
