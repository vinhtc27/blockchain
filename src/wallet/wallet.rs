use rand::rngs::OsRng;
use ripemd::{Digest, Ripemd160};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_derive::{Deserialize, Serialize};
use sha2::Sha256;

use p256::ecdsa::{SigningKey, VerifyingKey};

static CHECKSUM_LENGTH: usize = 4;
static VERSION: u8 = 0x00;

#[derive(Serialize, Deserialize)]
pub(crate) struct Wallet {
    private_key: WalletPrivateKey,
    public_key: WalletPublicKey,
}

impl Default for Wallet {
    fn default() -> Self {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        Self {
            private_key: WalletPrivateKey(signing_key),
            public_key: WalletPublicKey(verifying_key.to_sec1_bytes().to_vec()),
        }
    }
}

impl Wallet {
    pub(crate) fn address(&self) -> String {
        let public_key_hash = public_key_hash(&self.public_key);
        let mut full_hash = vec![VERSION];
        full_hash.extend_from_slice(&public_key_hash);
        let checksum = checksum(&full_hash);
        full_hash.extend_from_slice(&checksum);

        bs58::encode(&full_hash).into_string()
    }
}

fn public_key_hash(public_key: &WalletPublicKey) -> Vec<u8> {
    let public_key_hash_sha256 = Sha256::digest(&public_key.0);
    let public_key_hash_ripemd160 = Ripemd160::digest(public_key_hash_sha256);

    public_key_hash_ripemd160.to_vec()
}

fn checksum(payload: &[u8]) -> Vec<u8> {
    let first_hash = Sha256::digest(payload);
    let second_hash = Sha256::digest(first_hash);
    second_hash.into_iter().take(CHECKSUM_LENGTH).collect()
}

#[derive(Debug)]
struct WalletPrivateKey(SigningKey);

impl<'de> Deserialize<'de> for WalletPrivateKey {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <&[u8]>::deserialize(deserializer)?;
        SigningKey::from_slice(bytes)
            .map(WalletPrivateKey)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

impl Serialize for WalletPrivateKey {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = self.0.to_bytes();
        serializer.serialize_bytes(&encoded)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct WalletPublicKey(Vec<u8>);
