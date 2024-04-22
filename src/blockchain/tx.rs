use serde_derive::{Deserialize, Serialize};

use crate::{
    wallet::{hash_public_key, CHECKSUM_LENGTH},
    Result,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxInput {
    pub(crate) id: Vec<u8>,
    pub(crate) out: i64,
    pub(crate) signature: Vec<u8>,
    pub(crate) public_key: Vec<u8>,
}

impl TxInput {
    pub(crate) fn uses_key(&self, public_key_hash: &[u8]) -> bool {
        let locking_hash = hash_public_key(&self.public_key);
        locking_hash == public_key_hash
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxOutput {
    pub(crate) value: u64,
    pub(crate) public_key_hash: Vec<u8>,
}

impl TxOutput {
    pub(crate) fn new(value: u64, to: &str) -> Result<Self> {
        let public_key_hash: Vec<u8> = bs58::decode(to).into_vec()?;

        let mut tx_output = Self {
            value,
            public_key_hash: vec![],
        };
        tx_output.lock(&public_key_hash);

        Ok(tx_output)
    }

    fn lock(&mut self, public_key_hash: &[u8]) {
        let public_key_hash: Vec<u8> = public_key_hash
            .iter()
            .skip(1)
            .copied()
            .take(public_key_hash.len() - CHECKSUM_LENGTH - 1)
            .clone()
            .collect();
        self.public_key_hash = public_key_hash
    }

    pub(crate) fn is_locked_with_key(&self, public_key_hash: &[u8]) -> Result<bool> {
        let public_key_hash: Vec<u8> = public_key_hash
            .iter()
            .skip(1)
            .copied()
            .take(public_key_hash.len() - CHECKSUM_LENGTH - 1)
            .collect();

        Ok(self.public_key_hash == public_key_hash)
    }
}
