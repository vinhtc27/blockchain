use serde_derive::{Deserialize, Serialize};

use crate::{wallet::public_key_hash_from_address, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct TxInput {
    pub(crate) id: Vec<u8>,
    pub(crate) out: i64,
    pub(crate) signature: Vec<u8>,
    pub(crate) public_key_hash: Vec<u8>,
}

impl TxInput {
    pub(crate) fn new(id: Vec<u8>, out: i64, signature: Vec<u8>, address: &str) -> Result<Self> {
        let public_key_hash = if address.is_empty() {
            vec![]
        } else {
            public_key_hash_from_address(address)?
        };

        Ok(Self {
            id,
            out,
            signature,
            public_key_hash,
        })
    }

    pub(crate) fn uses_key(&self, address: &str) -> Result<bool> {
        let public_key_hash = public_key_hash_from_address(address)?;

        Ok(self.public_key_hash == public_key_hash)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct TxOutput {
    pub(crate) value: u64,
    pub(crate) public_key_hash: Vec<u8>,
}

impl TxOutput {
    pub(crate) fn new(value: u64, address: &str) -> Result<Self> {
        let public_key_hash = public_key_hash_from_address(address)?;

        Ok(Self {
            value,
            public_key_hash,
        })
    }

    pub(crate) fn is_locked_with_key(&self, address: &str) -> Result<bool> {
        let public_key_hash = public_key_hash_from_address(address)?;

        Ok(self.public_key_hash == public_key_hash)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct TxOutputs {
    pub(crate) outputs: Vec<TxOutput>,
}

impl<'a> TxOutputs {
    pub(crate) fn new() -> Self {
        Self { outputs: vec![] }
    }

    pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    pub(crate) fn deserialize(bytes: &'a [u8]) -> Result<Self> {
        Ok(bincode::deserialize(bytes)?)
    }
}
