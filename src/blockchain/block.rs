use super::{proof::ProofOfWork, transaction::Transaction};

use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Result;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Block {
    pub(crate) transactions: Vec<Transaction>,
    pub(crate) prevhash: Vec<u8>,
    pub(crate) hash: Vec<u8>,
    pub(crate) nonce: i64,
}

impl<'a> Block {
    pub(crate) fn hash_transactions(&self) -> Vec<u8> {
        let mut tx_hash = [0u8; 32];
        let mut hasher = Sha256::default();
        for tx in self.transactions.iter() {
            hasher.update(&tx.id)
        }

        tx_hash.copy_from_slice(&hasher.finalize());
        tx_hash.to_vec()
    }

    pub(crate) fn genesis(coinbase: Transaction) -> Self {
        Self::create_block(vec![coinbase], vec![])
    }

    pub(crate) fn create_block(transactions: Vec<Transaction>, prevhash: Vec<u8>) -> Self {
        let mut block = Block {
            transactions,
            prevhash,
            hash: vec![],
            nonce: 0i64,
        };
        let (nonce, block_hash) = ProofOfWork::new_proof(&block).run();

        block.nonce = nonce;
        block.hash = block_hash.to_vec();
        block
    }

    pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    pub(crate) fn deserialize(bytes: &'a [u8]) -> Result<Self> {
        Ok(bincode::deserialize(bytes)?)
    }
}
