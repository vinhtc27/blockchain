use super::proof::ProofOfWork;

pub(crate) struct Block {
    pub(crate) hash: Vec<u8>,
    pub(crate) data: Vec<u8>,
    pub(crate) prevhash: Vec<u8>,
    pub(crate) nonce: u64,
}

impl Block {
    pub(crate) fn genesis() -> Self {
        Self::create_block("Genesis".to_owned(), vec![])
    }

    pub(crate) fn create_block(data: impl Into<Vec<u8>>, prevhash: Vec<u8>) -> Self {
        let mut block = Block {
            hash: vec![],
            data: data.into(),
            prevhash,
            nonce: 0u64,
        };
        let (nonce, hash) = ProofOfWork::new_proof(&block).run();

        block.nonce = nonce;
        block.hash = hash.to_vec();
        block
    }
}
