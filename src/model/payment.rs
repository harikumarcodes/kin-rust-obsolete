use crate::{
    key::{private::PrivateKey, public::PublicKey},
    model::{invoice::Invoice, transaction_type::TransactionType},
};

/// Represents a payment retrieved from history.
#[derive(Debug)]
pub struct ReadOnlyPayment {
    pub sender: PublicKey,
    pub destination: PublicKey,
    pub tx_type: TransactionType,
    pub quarks: i64,
    pub memo: Option<String>,
    pub invoice: Option<Invoice>,
}

impl ReadOnlyPayment {
    pub fn new(
        sender: PublicKey,
        destination: PublicKey,
        tx_type: TransactionType,
        quarks: i64,
    ) -> ReadOnlyPayment {
        ReadOnlyPayment {
            sender,
            destination,
            tx_type,
            quarks,
            memo: None,
            invoice: None,
        }
    }
}

/// Represents a payment to be submitted.
#[derive(Clone)]
pub struct Payment {
    pub sender: PrivateKey,
    pub destination: PublicKey,
    pub tx_type: TransactionType,
    pub quarks: u64,
    pub subsidizer: Option<PrivateKey>,
    pub memo: Option<String>,
    pub invoice: Option<Invoice>,
    pub dedupe_id: Option<Vec<u8>>,
}

impl Payment {
    pub fn new(
        sender: PrivateKey,
        destination: PublicKey,
        tx_type: TransactionType,
        quarks: u64,
    ) -> Payment {
        Payment {
            sender,
            destination,
            tx_type,
            quarks,
            subsidizer: None,
            memo: None,
            invoice: None,
            dedupe_id: None,
        }
    }

    pub fn set_subsidizer(&mut self, subsidizer: PrivateKey) {
        self.subsidizer = Some(subsidizer);
    }

    pub fn set_memo(&mut self, memo: &str) {
        self.memo = Some(memo.to_string());
    }

    pub fn set_invoice(&mut self, invoice: Invoice) {
        self.invoice = Some(invoice);
    }

    pub fn set_dedupe_id(&mut self, dedupe_id: Vec<u8>) {
        self.dedupe_id = Some(dedupe_id);
    }
}
