use num_bigint::BigUint;
use sha2::{Digest, Sha256};

use super::block::Block;

//? Proof Of Work (PoW)
//1. Take data from the block

//2. Create a counter (nonce) start at 0

//3. Create a hash of the data plus the counter

//4. Check the hash to see if it meets a set of requirements

//? Requirements:
//1. The first few bytes must contain 0s (more 0 mean harder)

static DIFFICULTY: u64 = 12;

pub(crate) struct ProofOfWork<'a> {
    block: &'a Block,
    target: BigUint,
}

impl<'a> ProofOfWork<'a> {
    fn init_data(&self, nonce: u64) -> Vec<u8> {
        let mut data = vec![];
        data.extend_from_slice(&self.block.prevhash);
        data.extend_from_slice(&self.block.data);
        data.extend_from_slice(&nonce.to_be_bytes());
        data.extend_from_slice(&DIFFICULTY.to_be_bytes());
        data
    }

    pub(crate) fn new_proof(block: &'a Block) -> Self {
        let target = BigUint::from(1u64);
        let target = target << (256 - DIFFICULTY);

        Self { block, target }
    }

    pub(crate) fn run(&self) -> (u64, [u8; 32]) {
        let mut hash = [0u8; 32];
        let mut nonce = 0u64;

        loop {
            let data = self.init_data(nonce);
            let mut hasher = Sha256::new();
            hasher.update(&data);
            hash.copy_from_slice(&hasher.finalize());

            if BigUint::from_bytes_be(&hash) < self.target {
                println!("Nonce {:?} - Hash {:?}\r", nonce, hex::encode(&hash));
                break;
            } else {
                nonce += 1;
            }
        }

        println!();

        (nonce, hash)
    }

    pub(crate) fn validate(&self) -> bool {
        let data = self.init_data(self.block.nonce);
        let mut hasher = Sha256::new();
        hasher.update(&data);

        BigUint::from_bytes_be(&hasher.finalize()) < self.target
    }
}
