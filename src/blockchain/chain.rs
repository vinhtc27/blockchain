use crate::blockchain::proof::ProofOfWork;

use super::block::Block;

pub struct BlockChain {
    blocks: Vec<Block>,
}

impl BlockChain {
    pub fn init_blockchain() -> Self {
        Self {
            blocks: vec![Block::genesis()],
        }
    }

    pub fn add_block(&mut self, data: impl Into<Vec<u8>>) {
        let prev_block = self.blocks.last().unwrap();
        let new_block = Block::create_block(data, prev_block.hash.clone());
        self.blocks.push(new_block);
    }

    pub fn println(&self) {
        self.blocks.iter().for_each(|block| {
            println!("Block prev: {:?}", hex::encode(&block.prevhash));
            println!("Block data:  {:?}", std::str::from_utf8(&block.data));
            println!("Block hash:  {:?}", hex::encode(&block.hash));
            println!("PoW check: {:?}", ProofOfWork::new_proof(block).validate());

            println!()
        })
    }
}
