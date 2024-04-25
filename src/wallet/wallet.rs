use rand::rngs::OsRng;
use secp256k1::{ecdsa::Signature, rand, Message, PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Result;

static CHECKSUM_LENGTH: usize = 4;
static VERSION: u8 = 0x00;

#[derive(Serialize, Deserialize)]
pub struct Wallet {
    private_key: WalletPrivateKey,
    public_key: WalletPublicKey,
}

impl Wallet {
    pub(crate) fn new() -> Result<Self> {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

        Ok(Self {
            private_key: WalletPrivateKey(secret_key),
            public_key: WalletPublicKey(public_key),
        })
    }

    pub(crate) fn address(&self) -> String {
        let public_key_hash = &self.public_key.0.serialize().to_vec();

        let mut full_hash = vec![VERSION];
        full_hash.extend_from_slice(public_key_hash);

        let checksum = checksum(&full_hash);
        full_hash.extend_from_slice(&checksum);

        bs58::encode(&full_hash).into_string()
    }

    pub(crate) fn sign(&self, tx_id: &[u8]) -> Result<Signature> {
        let secp = Secp256k1::new();
        let digest = Sha256::digest(tx_id);
        let message = Message::from_digest(digest.into());

        Ok(secp.sign_ecdsa(&message, &self.private_key.0))
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

fn checksum(payload: &[u8]) -> Vec<u8> {
    let first_hash = Sha256::digest(payload);
    let second_hash = Sha256::digest(first_hash);
    second_hash.into_iter().take(CHECKSUM_LENGTH).collect()
}

struct WalletPrivateKey(SecretKey);

impl<'de> Deserialize<'de> for WalletPrivateKey {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <&[u8]>::deserialize(deserializer)?;
        SecretKey::from_slice(bytes)
            .map(WalletPrivateKey)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

impl Serialize for WalletPrivateKey {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = self.0.secret_bytes();
        serializer.serialize_bytes(&encoded)
    }
}

struct WalletPublicKey(PublicKey);

impl<'de> Deserialize<'de> for WalletPublicKey {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <&[u8]>::deserialize(deserializer)?;
        PublicKey::from_slice(bytes)
            .map(WalletPublicKey)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

impl Serialize for WalletPublicKey {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = self.0.serialize();
        serializer.serialize_bytes(&encoded)
    }
}
