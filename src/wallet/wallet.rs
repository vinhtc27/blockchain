use rand::rngs::OsRng;
use ripemd::{Digest, Ripemd160};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_derive::{Deserialize, Serialize};
use sha2::Sha256;

use p256::ecdsa::{signature::SignerMut, Signature, SigningKey, VerifyingKey};

use crate::Result;

static CHECKSUM_LENGTH: usize = 4;
static VERSION: u8 = 0x00;

#[derive(Serialize, Deserialize)]
pub struct Wallet {
    private_key: WalletPrivateKey,
    public_key: Vec<u8>,
}

impl Default for Wallet {
    fn default() -> Self {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        Self {
            private_key: WalletPrivateKey(signing_key),
            public_key: verifying_key.to_sec1_bytes().to_vec(),
        }
    }
}

impl Wallet {
    pub(crate) fn address(&self) -> String {
        let public_key_hash = hash_public_key(&self.public_key);

        let mut full_hash = vec![VERSION];
        full_hash.extend_from_slice(&public_key_hash);

        let checksum = checksum(&full_hash);
        full_hash.extend_from_slice(&checksum);

        bs58::encode(&full_hash).into_string()
    }

    pub(crate) fn sign(&mut self, tx_id: &[u8]) -> Result<Signature> {
        Ok(self.private_key.0.sign(tx_id))
    }
}

pub fn public_key_hash_from_address(address: &str) -> Result<Vec<u8>> {
    let decoded_address: Vec<u8> = bs58::decode(address).into_vec()?;
    Ok(decoded_address
        .iter()
        .skip(1)
        .take(decoded_address.len() - CHECKSUM_LENGTH - 1)
        .cloned()
        .collect())
}

pub fn validate_address(address: &str) -> Result<bool> {
    let decoded_address: Vec<u8> = bs58::decode(address).into_vec()?;

    let public_key_hash_len = decoded_address.len();

    let version = &decoded_address[0];

    let actual_checksum: Vec<u8> = decoded_address
        .iter()
        .skip(1)
        .skip(public_key_hash_len - CHECKSUM_LENGTH - 1)
        .cloned()
        .collect();

    let public_key_hash: Vec<u8> = decoded_address
        .iter()
        .skip(1)
        .take(public_key_hash_len - CHECKSUM_LENGTH - 1)
        .cloned()
        .collect();

    let mut full_hash = vec![*version];
    full_hash.extend_from_slice(&public_key_hash);

    let tartget_checksum = checksum(&full_hash);

    Ok(actual_checksum == tartget_checksum)
}

fn hash_public_key(public_key: &[u8]) -> Vec<u8> {
    let public_key_hash_sha256 = Sha256::digest(public_key);
    let public_key_hash_ripemd160 = Ripemd160::digest(public_key_hash_sha256);

    public_key_hash_ripemd160.to_vec()
}

fn checksum(payload: &[u8]) -> Vec<u8> {
    let first_hash = Sha256::digest(payload);
    let second_hash = Sha256::digest(first_hash);
    second_hash.into_iter().take(CHECKSUM_LENGTH).collect()
}

pub struct WalletPrivateKey(SigningKey);

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
