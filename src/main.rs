use sha2::{Digest, Sha256};

struct Block {
    hash: Vec<u8>,
    data: Vec<u8>,
    prev_hash: Vec<u8>,
}

impl Block {
    fn genesis() -> Self {
        Self::create_block("Genesis".to_owned(), vec![])
    }

    fn create_block(data: impl Into<Vec<u8>>, prev_hash: Vec<u8>) -> Self {
        let mut block = Block {
            hash: vec![],
            data: data.into(),
            prev_hash,
        };
        block.derive_hash();
        block
    }

    fn derive_hash(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        hasher.update(&self.prev_hash);
        self.hash = hasher.finalize().to_vec();
    }
}

struct BlockChain {
    blocks: Vec<Block>,
}

impl BlockChain {
    fn init_blockchain() -> Self {
        Self {
            blocks: vec![Block::genesis()],
        }
    }

    fn add_block(&mut self, data: impl Into<Vec<u8>>) {
        let prev_block = self.blocks.last().unwrap();
        let new_block = Block::create_block(data, prev_block.hash.clone());
        self.blocks.push(new_block);
    }
}

fn main() {
    let mut chain = BlockChain::init_blockchain();

    chain.add_block("first block");
    chain.add_block("second block");
    chain.add_block("third block");

    let _ = chain.blocks.iter().for_each(|block| {
        println!("Block prev: {:?}", hex::encode(&block.prev_hash));
        println!("Block data:  {:?}", std::str::from_utf8(&block.data));
        println!("Block hash:  {:?}", hex::encode(&block.hash));
        println!()
    });
}
