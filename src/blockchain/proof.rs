use num_bigint::{BigInt, Sign};
use sha2::{Digest, Sha256};

use super::block::Block;

//? Proof Of Work (PoW):
//Step 1: Take data from the block

//Step 2: Create a counter (nonce) start at 0

//Step 3: Create a hash of the data plus the counter

//Step 4: Check the hash to see if it meets a set of requirements

//? Requirements: The first few bytes must contain 0s (more 0 mean harder)

static DIFFICULTY: u64 = 12;

pub(crate) struct ProofOfWork<'a> {
    block: &'a Block,
    target: BigInt,
}

impl<'a> ProofOfWork<'a> {
    fn init_data(&self, nonce: u64) -> Vec<u8> {
        let mut data = vec![];
        data.extend_from_slice(&self.block.prevhash);
        data.extend_from_slice(&self.block.hash_transactions());
        data.extend_from_slice(&nonce.to_be_bytes());
        data.extend_from_slice(&DIFFICULTY.to_be_bytes());
        data
    }

    pub(crate) fn new_proof(block: &'a Block) -> Self {
        let target = BigInt::from(1u64);
        let target = target << (256 - DIFFICULTY);

        Self { block, target }
    }

    pub(crate) fn run(&self) -> (u64, [u8; 32]) {
        let mut block_hash = [0u8; 32];
        let mut nonce = 0u64;

        loop {
            let data = self.init_data(nonce);
            block_hash.copy_from_slice(&Sha256::digest(&data));

            if BigInt::from_bytes_be(Sign::Plus, &block_hash) < self.target {
                println!("Nonce {:?} - Hash {:?}\r", nonce, hex::encode(block_hash));
                break;
            } else {
                nonce += 1;
            }
        }

        println!();

        (nonce, block_hash)
    }

    pub(crate) fn validate(&self) -> bool {
        let data = self.init_data(self.block.nonce);
        BigInt::from_bytes_be(Sign::Plus, &Sha256::digest(data)) < self.target
    }
}
