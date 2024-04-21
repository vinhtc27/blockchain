use super::{proof::ProofOfWork, transaction::Transaction};

use sha2::{Digest, Sha256};
use speedy::{BigEndian, Readable, Writable};

use crate::Result;

#[derive(Readable, Writable)]
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
        Ok(self.write_to_vec_with_ctx(BigEndian::default())?)
    }

    pub(crate) fn deserialize(bytes: &'a [u8]) -> Result<Self> {
        Ok(Self::read_from_buffer_with_ctx(
            BigEndian::default(),
            bytes,
        )?)
    }
}
