use {
    bs58,
    solana_sdk::pubkey::Pubkey as SolanaPublicKey,
    std::convert::{TryFrom, TryInto},
    stellar::types::PublicKey as StellarPublicKey,
    substrate_stellar_sdk as stellar,
};

/// Represents a ed25519 public key.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PublicKey([u8; PublicKey::LEN]);

impl PublicKey {
    /// Byte length of a public key.
    pub const LEN: usize = 32;

    /// Returns a public key from the provided slice.
    pub fn new(slice: &[u8]) -> PublicKey {
        PublicKey::try_from(slice).unwrap()
    }

    /// Returns a public key from a base58-encoded string.
    pub fn from_base58(address: &str) -> PublicKey {
        let decoded = bs58::decode(address).into_vec().unwrap();

        PublicKey::try_from(&decoded).unwrap()
    }

    /// Returns a public key from a Stellar-encoded address.
    pub fn from_stellar(address: &str) -> PublicKey {
        if address.len() != 56 {
            panic!("Address format not supported.")
        }
        if !address.starts_with('G') {
            panic!("Provided address is not a public key.")
        }

        let decoded = StellarPublicKey::from_encoding(address)
            .unwrap()
            .into_binary();

        PublicKey(decoded)
    }

    /// Returns the raw bytes of the public key.
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Returns the public key as a base58-encoded string.
    pub fn to_base58(self) -> String {
        bs58::encode(self.0).into_string()
    }

    /// Returns the public key as a Stellar-encoded address.
    pub fn to_stellar(self) -> String {
        let stellar_key = StellarPublicKey::from_binary(self.0);

        String::from_utf8(stellar_key.to_encoding()).unwrap()
    }

    /// Returns the public key as a solana public key object.
    pub fn to_solana_key(self) -> SolanaPublicKey {
        SolanaPublicKey::new(&self.0)
    }
}

impl TryFrom<&[u8]> for PublicKey {
    type Error = ();

    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() == PublicKey::LEN {
            let bytes: [u8; 32] = slice.try_into().unwrap();

            Ok(PublicKey(bytes))
        } else {
            panic!(
                "Expected a {} byte public key. Received {} bytes.",
                PublicKey::LEN,
                slice.len()
            )
        }
    }
}

impl TryFrom<&Vec<u8>> for PublicKey {
    type Error = ();

    fn try_from(vec: &Vec<u8>) -> Result<Self, Self::Error> {
        let slice: &[u8] = vec;

        PublicKey::try_from(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STELLAR_SEED: &str = "SCZ4KGTCMAFIJQCCJDMMKDFUB7NYV56VBNEU7BKMR4PQFUETJCWLV6GN";
    const STELLAR_ADDRESS: &str = "GCABWU4FHL3RGOIWCX5TOVLIAMLEU2YXXLCMHVXLDOFHKLNLGCSBRJYP";
    const BASE58_ADDRESS: &str = "9d5MTnMor78dfRHn7Csi8hXkaTFm6rQRiJ5HbnRgmfrP";

    #[test]
    fn base58_round_trip() {
        let pubkey_1 = PublicKey::from_base58(BASE58_ADDRESS);
        let pubkey_2 = PublicKey::from_base58(&pubkey_1.to_base58());

        assert!(pubkey_1.eq(&pubkey_2));
    }

    #[test]
    fn stellar_round_trip() {
        let pubkey_1 = PublicKey::from_stellar(STELLAR_ADDRESS);
        let pubkey_2 = PublicKey::from_stellar(&pubkey_1.to_stellar());

        assert!(pubkey_1.eq(&pubkey_2));
    }

    #[test]
    #[should_panic(expected = "Provided address is not a public key.")]
    fn from_stellar_with_invalid_address() {
        PublicKey::from_stellar(STELLAR_SEED);
    }

    #[test]
    #[should_panic(expected = "Address format not supported.")]
    fn from_stellar_with_invalid_address_len() {
        PublicKey::from_stellar("GCABW");
    }
}
