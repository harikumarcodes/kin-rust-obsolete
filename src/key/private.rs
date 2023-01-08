use {
    crate::key::public::PublicKey, bs58, ed25519_compact::*,
    substrate_stellar_sdk::SecretKey as StellarSecretKey,
};

/// Represents a ed25519 private key.
#[derive(Copy, Clone)]
pub struct PrivateKey {
    keypair: KeyPair,
}

impl PartialEq for PrivateKey {
    fn eq(&self, other: &PrivateKey) -> bool {
        self.secret_key() == other.secret_key()
    }
}

impl PrivateKey {
    /// Returns a private key from a randomly generated seed.
    pub fn rand() -> PrivateKey {
        let seed = Seed::generate();
        let keypair = KeyPair::from_seed(seed);

        PrivateKey { keypair }
    }

    /// Returns a private key from a base58-encoded seed.
    pub fn from_base58(seed: &str) -> PrivateKey {
        let decoded = bs58::decode(seed).into_vec().unwrap();
        let seed = Seed::from_slice(&decoded).unwrap();
        let keypair = KeyPair::from_seed(seed);

        PrivateKey { keypair }
    }

    /// Returns a private key from a Stellar-encoded seed.
    pub fn from_stellar(seed: &str) -> PrivateKey {
        if seed.len() != 56 {
            panic!("Seed format not supported.")
        }
        if !seed.starts_with('S') {
            panic!("Provided seed is not a private key.")
        }

        let stellar_key = StellarSecretKey::from_encoding(seed).unwrap();
        let seed = Seed::from_slice(stellar_key.as_binary()).unwrap();
        let keypair = KeyPair::from_seed(seed);

        PrivateKey { keypair }
    }

    /// Returns the public key corresponding to this private key.
    pub fn public_key(self) -> PublicKey {
        let pk = self.keypair.pk.as_ref();

        PublicKey::new(pk)
    }

    /// Returns the raw bytes of the secret key, where the first 32 bytes are
    /// the secret seed and the remaining 32 bytes are the public key.
    pub fn secret_key(&self) -> [u8; 64] {
        *self.keypair.sk
    }

    /// Returns the seed as a base58-encoded string.
    pub fn to_base58(self) -> String {
        let seed = self.keypair.sk.seed();

        bs58::encode(seed.as_ref()).into_string()
    }

    pub fn to_stellar(self) -> String {
        let stellar_key = StellarSecretKey::from_binary(*self.keypair.sk.seed());

        String::from_utf8(stellar_key.to_encoding()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::{bs58, PrivateKey};

    const STELLAR_SEED: &str = "SCZ4KGTCMAFIJQCCJDMMKDFUB7NYV56VBNEU7BKMR4PQFUETJCWLV6GN";
    const STELLAR_ADDRESS: &str = "GCABWU4FHL3RGOIWCX5TOVLIAMLEU2YXXLCMHVXLDOFHKLNLGCSBRJYP";

    #[test]
    fn base58_round_trip() {
        let key_1 = PrivateKey::rand();
        let key_2 = PrivateKey::from_base58(&key_1.to_base58());

        assert!(key_1.eq(&key_2));
    }

    #[test]
    fn stellar_round_trip() {
        let key_1 = PrivateKey::from_stellar(STELLAR_SEED);
        let key_2 = PrivateKey::from_stellar(&key_1.to_stellar());

        assert!(key_1.eq(&key_2));
    }

    #[test]
    #[should_panic(expected = "Provided seed is not a private key.")]
    fn from_stellar_with_invalid_seed() {
        PrivateKey::from_stellar(STELLAR_ADDRESS);
    }

    #[test]
    #[should_panic(expected = "Seed format not supported.")]
    fn from_stellar_with_invalid_seed_len() {
        PrivateKey::from_stellar("SCZ4K");
    }

    #[test]
    fn secret_key() {
        let key = PrivateKey::rand();

        let first_half = &key.secret_key()[..32];
        let second_half = &key.secret_key()[32..64];

        let seed = bs58::decode(key.to_base58()).into_vec().unwrap();
        let pubkey = key.public_key().to_bytes();

        assert_eq!(first_half, &seed);
        assert_eq!(second_half, &pubkey);
    }
}
