use super::proof::ProofOfWork;

use speedy::{BigEndian, Readable, Writable};

use crate::Result;

#[derive(PartialEq, Debug, Readable, Writable)]
pub(crate) struct Block {
    pub(crate) hash: Vec<u8>,
    pub(crate) data: Vec<u8>,
    pub(crate) prevhash: Vec<u8>,
    pub(crate) nonce: u64,
}

impl<'a> Block {
    pub(crate) fn genesis() -> Self {
        Self::create_block("Genesis".to_owned(), vec![])
    }

    pub(crate) fn create_block(data: impl Into<Vec<u8>>, prevhash: impl Into<Vec<u8>>) -> Self {
        let mut block = Block {
            hash: vec![],
            data: data.into(),
            prevhash: prevhash.into(),
            nonce: 0u64,
        };
        let (nonce, hash) = ProofOfWork::new_proof(&block).run();

        block.nonce = nonce;
        block.hash = hash.to_vec();
        block
    }

    pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
        Ok(self.write_to_vec_with_ctx(BigEndian::default())?.into())
    }

    pub(crate) fn deserialize(bytes: &'a [u8]) -> Result<Self> {
        Ok(Self::read_from_buffer_with_ctx(
            BigEndian::default(),
            bytes,
        )?)
    }
}
