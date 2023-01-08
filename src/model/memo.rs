use {
    crate::model::transaction_type::TransactionType,
    base64,
    std::convert::{TryFrom, TryInto},
    substrate_stellar_sdk::Memo as StellarMemo,
};

/// Implements the Agora memo specification as defined in github.com/kinecosystem/agora-api.
#[derive(Debug)]
pub struct Memo {
    pub bytes: [u8; Memo::LEN],
}

impl Memo {
    /// Byte length of a memo.
    pub const LEN: usize = 32;

    /// The highest Agora memo version supported by this implementation.
    pub const MAX_VERSION: u8 = 1;

    const MAGIC_BYTE: u8 = 0x1;

    /// Returns a memo containing the provided properties.
    pub fn new(version: u8, tx_type: TransactionType, app_index: u16, foreign_key: &[u8]) -> Memo {
        if version > 7 {
            panic!("Invalid version.");
        }
        if tx_type == TransactionType::Unknown {
            panic!("Invalid transaction type.");
        }
        if foreign_key.len() > 29 {
            panic!("Invalid foreign key length.");
        }

        let mut b = [0; 32];

        // Encode magic byte.
        b[0] = Memo::MAGIC_BYTE;

        // Encode version.
        b[0] |= version << 2;

        // Encode transaction type.
        let t = tx_type as u8;
        b[0] |= (t & 0x7) << 5;
        b[1] = (t & 0x18) >> 3;

        // Encode AppIndex.
        b[1] |= ((app_index & 0x3f) << 2) as u8;
        b[2] = ((app_index & 0x3fc0) >> 6) as u8;
        b[3] = ((app_index & 0xc000) >> 14) as u8;

        // Encode FK.
        if !foreign_key.is_empty() {
            b[3] |= (foreign_key[0] & 0x3f) << 2;

            // Insert the rest of the FK. Since each loop references FK[n] and
            // FK[n+1], the upper bound is offset by 3 instead of 4.
            for i in 4..(3 + foreign_key.len()) {
                // Apply last 2-bits of current byte.
                b[i] = (foreign_key[i - 4] >> 6) & 0x3;

                // Apply first 6-bits of next byte.
                b[i] |= (foreign_key[i - 3] & 0x3f) << 2;
            }

            // If the FK is less than 29 bytes, the last 2 bits
            // of the FK can be included in the memo
            if foreign_key.len() < 29 {
                b[foreign_key.len() + 3] = (foreign_key[foreign_key.len() - 1] >> 6) & 0x3;
            }
        }

        Memo::from_slice(&b)
    }

    /// Returns a memo from a slice.
    pub fn from_slice(slice: &[u8]) -> Memo {
        Memo::try_from(slice).unwrap()
    }

    /// Returns a memo from a base64 encoded string.
    ///
    /// # Arguments
    /// * `s` - A string slice that holds the base64 encoded memo.
    /// * `strict` - Whether or not to run strict validation on the memo.
    pub fn from_base64(s: &str, strict: bool) -> Result<Memo, base64::DecodeError> {
        let raw = base64::decode(s).unwrap();

        let memo = Memo::from_slice(&raw);
        if !memo.is_valid(strict) {
            panic!("Invalid memo.");
        }

        Ok(memo)
    }

    /// Returns an Agora memo from a Stellar hash memo.
    ///
    /// # Arguments
    /// * `stellar_memo` - The Stellar hash memo to convert into an Agora memo.
    /// * `strict` - Whether or not to run strict validation on the memo.
    pub fn from_stellar(stellar_memo: &StellarMemo, strict: bool) -> Option<Memo> {
        match stellar_memo {
            StellarMemo::MemoHash(hash) => {
                let memo = Memo::from_slice(hash);
                if !memo.is_valid(strict) {
                    panic!("Not a valid Agora memo.")
                }

                Some(memo)
            }
            _ => None,
        }
    }

    pub fn to_base64(&self) -> String {
        base64::encode(self.bytes)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.bytes
    }

    /// Returns whether or not the memo is valid.
    ///
    /// It should be noted that there are no guarantees if the memo is valid, only if the memo is invalid. That is, this
    /// function may return false positives.
    pub fn is_valid(&self, strict: bool) -> bool {
        let mut valid = self.bytes[0] & 0x3 == Memo::MAGIC_BYTE;

        if strict {
            valid &=
                self.version() <= Memo::MAX_VERSION && self.tx_type() != TransactionType::Unknown;
        }

        valid
    }

    /// Returns the TransactionType of this memo.
    pub fn tx_type(&self) -> TransactionType {
        TransactionType::from_i32(self.tx_type_raw() as i32)
    }

    /// Returns the raw transaction type of this memo, even if it's unsupported by this implementation.
    ///
    /// This method should only be used as a fallback for when Memo.tx_type yields Transactiontype::Unknown.
    pub fn tx_type_raw(&self) -> u8 {
        (self.bytes[0] >> 5) | (self.bytes[1] & 0x3) << 3
    }

    /// Returns the memo encoding version of this memo.
    pub fn version(&self) -> u8 {
        (self.bytes[0] & 0x1c) >> 2
    }

    /// Returns the app index of the memo.
    pub fn app_index(&self) -> u16 {
        let b1 = self.bytes[1] as u16;
        let b2 = self.bytes[2] as u16;
        let b3 = self.bytes[3] as u16;

        let x = b1 >> 2;
        let y = b2 << 6;
        let z = (b3 & 0x3) << 14;

        x | y | z
    }

    /// Returns the foreign key of the memo.
    pub fn foreign_key(&self) -> [u8; 29] {
        let mut fk = [0u8; 29];

        for (index, byte) in fk.iter_mut().enumerate().take(28) {
            *byte |= self.bytes[index + 3] >> 2;
            *byte |= (self.bytes[index + 4] & 0x3) << 6;
        }

        fk[28] = self.bytes[31] >> 2;

        fk
    }
}

impl TryFrom<&[u8]> for Memo {
    type Error = ();

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() <= Memo::LEN {
            let bytes: [u8; 32] = slice.try_into().unwrap();

            Ok(Memo { bytes })
        } else {
            panic!(
                "Expected a memo of size {} bytes or smaller. Received {} bytes.",
                Memo::LEN,
                slice.len()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Memo, TransactionType};
    use crate::model::invoice::{Invoice, InvoiceList};
    use substrate_stellar_sdk::compound_types::LimitedString;
    use substrate_stellar_sdk::Memo as StellarMemo;

    /// An empty Agora memo foreign key.
    const EMPTY_FK: [u8; 29] = [0; 29];

    #[test]
    fn new_with_all_versions() {
        for version in 0..8 {
            let m = Memo::new(version, TransactionType::Earn, 1, &[]);
            assert_memo_params_eq(&m, version, TransactionType::Earn, 1, &EMPTY_FK);
        }
    }

    #[test]
    fn new_with_all_transaction_types() {
        use TransactionType::{Earn, None, Spend, P2P};

        for tx_type in [None, Earn, Spend, P2P] {
            let m = Memo::new(1, tx_type, 1, &[]);
            assert_memo_params_eq(&m, 1, tx_type, 1, &EMPTY_FK);
        }
    }

    #[test]
    fn new_with_all_app_indexes() {
        for app_index in 0..u16::MAX {
            let m = Memo::new(1, TransactionType::Earn, app_index, &[]);
            assert_memo_params_eq(&m, 1, TransactionType::Earn, app_index, &EMPTY_FK);
        }
    }

    #[test]
    fn new_with_all_foreign_key_byte_values() {
        for byte_value in 0..u8::MAX {
            let mut fk = [0; 29];

            for (index, byte) in fk.iter_mut().enumerate() {
                *byte = ((byte_value as u16 + index as u16) & 0xFF) as u8;
            }

            let m = Memo::new(1, TransactionType::Earn, 1, &fk);
            assert_memo_params_eq(&m, 1, TransactionType::Earn, 1, &fk[..28]);

            // Compare only 6 bits of the last byte, as the
            // memo can only hold 230 bits of the 232-bit FK.
            assert_eq!(m.foreign_key()[28], fk[28] & 0x3f);
        }
    }

    #[test]
    fn new_with_short_foreign_key() {
        let version = 1;
        let app_index = 1;
        let fk = [0, 255];

        let m = Memo::new(version, TransactionType::Earn, app_index, &fk);

        assert_eq!(&m.foreign_key()[..fk.len()], &fk);
        for byte in &m.foreign_key()[fk.len()..] {
            assert_eq!(*byte, 0);
        }
    }

    #[test]
    #[should_panic(expected = "Invalid version")]
    fn new_with_invalid_version() {
        Memo::new(8, TransactionType::Earn, 1, &[]);
    }

    #[test]
    #[should_panic(expected = "Invalid transaction type")]
    fn new_with_invalid_transaction_type() {
        Memo::new(1, TransactionType::Unknown, 1, &[]);
    }

    #[test]
    #[should_panic(expected = "Invalid foreign key length")]
    fn new_with_invalid_foreign_key() {
        Memo::new(1, TransactionType::Earn, 1, &[0; 30]);
    }

    #[test]
    fn is_valid() {
        let mut m = Memo::new(1, TransactionType::Earn, 1, &[]);
        assert!(m.is_valid(false));
        assert!(m.is_valid(true));

        // Invalid magic byte
        m.bytes[0] = Memo::MAGIC_BYTE >> 1;
        assert!(!m.is_valid(false));
        assert!(!m.is_valid(true));

        // Unsupported version
        m = Memo::new(3, TransactionType::Earn, 1, &[]);
        assert!(m.is_valid(false));
        assert!(!m.is_valid(true));
    }

    #[test]
    fn from_stellar() {
        let valid_memo = Memo::new(2, TransactionType::Earn, 1, &[]);
        let stellar_memo = StellarMemo::MemoHash(valid_memo.bytes);
        let m = Memo::from_stellar(&stellar_memo, false).unwrap();
        assert_eq!(&m.bytes, &valid_memo.bytes);

        let strictly_valid_memo = Memo::new(1, TransactionType::Earn, 1, &[]);
        let stellar_memo = StellarMemo::MemoHash(strictly_valid_memo.bytes);
        let m = Memo::from_stellar(&stellar_memo, true).unwrap();
        assert_eq!(&m.bytes, &strictly_valid_memo.bytes);
    }

    #[test]
    fn from_stellar_returns_none_on_invalid_variant() {
        let stellar_memo = StellarMemo::MemoText(LimitedString::new(b"text".to_vec()).unwrap());
        let m = Memo::from_stellar(&stellar_memo, false);

        assert!(m.is_none());
    }

    #[test]
    fn parsing_bytes_generated_by_go_sdk() {
        test_parsing_memo_with_empty_fk();
        test_parsing_memo_with_unknown_tx_type();
        test_parsing_memo_with_invoice_hash();
    }

    fn test_parsing_memo_with_empty_fk() {
        let bs64_memo = "PVwrAwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let m = Memo::from_base64(bs64_memo, false).unwrap();
        assert_memo_params_eq(&m, 7, TransactionType::Earn, 51927, &EMPTY_FK);
    }

    fn test_parsing_memo_with_unknown_tx_type() {
        let bs64_memo = "RQUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let m = Memo::from_base64(bs64_memo, false).unwrap();
        assert_memo_params_eq(&m, 1, TransactionType::Unknown, 1, &EMPTY_FK);
    }

    fn test_parsing_memo_with_invoice_hash() {
        let bs64_memo = "ZQQAiLyJQCfEDmO0QOygz/PZOLDcbwP1FmbdtZ9E+wM=";
        let invoice = Invoice::new(
            "Important Payment",
            100000,
            Some("A very important payment"),
            Some("some sku".as_bytes()),
        );
        let invoice_list = InvoiceList::new(&[invoice]);
        let expected_fk = invoice_list.get_sha244_hash();

        let m = Memo::from_base64(bs64_memo, true).unwrap();
        assert_memo_params_eq(&m, 1, TransactionType::P2P, 1, &expected_fk);
    }

    /// Asserts that the memo contains all expected values, including the magic byte.
    fn assert_memo_params_eq(
        memo: &Memo,
        expected_version: u8,
        expected_tx_type: TransactionType,
        expected_app_index: u16,
        expected_fk: &[u8],
    ) {
        assert_eq!(memo.bytes[0] & 0x3, Memo::MAGIC_BYTE);
        assert_eq!(memo.version(), expected_version);
        assert_eq!(memo.tx_type(), expected_tx_type);
        assert_eq!(memo.app_index(), expected_app_index);
        assert_eq!(&memo.foreign_key()[..expected_fk.len()], expected_fk);
    }
}
