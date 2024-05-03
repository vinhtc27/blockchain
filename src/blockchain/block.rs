use std::time::SystemTime;

use super::{merkle::MerkleTree, proof::ProofOfWork, transaction::Transaction};

use serde_derive::{Deserialize, Serialize};

use crate::Result;

#[derive(Serialize, Deserialize)]
pub struct Block {
    pub(crate) transactions: Vec<Transaction>,
    pub(crate) prevhash: Vec<u8>,
    pub(crate) hash: Vec<u8>,
    pub(crate) nonce: u64,
    pub(crate) height: u64,
    pub(crate) timestamp: u64,
}

impl<'a> Block {
    pub(crate) fn hash_transactions(&self) -> Result<Vec<u8>> {
        let mut hashes = vec![];
        for tx in self.transactions.iter() {
            hashes.push(tx.serialize()?)
        }

        let merkle_tree = MerkleTree::new(hashes)?;

        Ok(merkle_tree.root_hash())
    }

    pub(crate) fn genesis(coinbase: Transaction) -> Result<Self> {
        Ok(Self::create_block(vec![coinbase], vec![], 0)?)
    }

    pub(crate) fn create_block(
        transactions: Vec<Transaction>,
        prevhash: Vec<u8>,
        height: u64,
    ) -> Result<Self> {
        let mut block = Block {
            transactions,
            prevhash,
            hash: vec![],
            nonce: 0u64,
            height,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("System time is earlier than Unix epoch")
                .as_secs(),
        };
        let (nonce, block_hash) = ProofOfWork::new_proof(&block).run()?;

        block.nonce = nonce;
        block.hash = block_hash.to_vec();

        Ok(block)
    }

    pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    pub(crate) fn deserialize(bytes: &'a [u8]) -> Result<Self> {
        Ok(bincode::deserialize(bytes)?)
    }
}
