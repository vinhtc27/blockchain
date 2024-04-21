use super::block::Block;

use crate::{blockchain::proof::ProofOfWork, Result};

use sled::{Config, Db};

static DB_PATH: &str = "./tmp/blocks";
static LAST_HASH: &'static [u8; 9] = b"LAST_HASH";

pub struct BlockChain {
    last_hash: Vec<u8>,
    database: Db,
}

impl BlockChain {
    pub fn init_blockchain() -> Result<Self> {
        let database: Db = Config::default().path(DB_PATH.to_owned()).open()?;
        let last_hash = database.transaction(|db| {
            if let Some(lh) = db.get(LAST_HASH)? {
                Ok(lh.as_ref().into())
            } else {
                let genesis = Block::genesis();
                db.insert(genesis.hash.clone(), genesis.serialize().unwrap())?;
                db.insert(LAST_HASH, genesis.hash.clone())?;
                Ok(genesis.hash)
            }
        })?;

        Ok(Self {
            last_hash,
            database,
        })
    }

    pub fn add_block(&mut self, data: impl Into<Vec<u8>>) -> Result<()> {
        let new_block = Block::create_block(data, self.last_hash.clone());
        self.database.transaction(|db| {
            db.insert(new_block.hash.clone(), new_block.serialize().unwrap())?;
            db.insert(LAST_HASH, new_block.hash.clone())?;

            Ok(())
        })?;

        Ok(())
    }

    pub fn iterator(&self) -> BlockChainIterator {
        BlockChainIterator {
            current_hash: self.last_hash.clone(),
            database: self.database.clone(),
        }
    }
}

pub struct BlockChainIterator {
    current_hash: Vec<u8>,
    database: Db,
}

impl BlockChainIterator {
    pub fn next(&mut self) -> Result<Option<()>> {
        match self.database.get(&self.current_hash)? {
            None => Ok(None),
            Some(bytes) => match Block::deserialize(&bytes) {
                Ok(block) => {
                    self.current_hash = block.prevhash.clone();
                    println!("Prev. hash: {:?}", hex::encode(&block.prevhash));
                    println!("Data: {:?}", hex::encode(&block.data));
                    println!("Hash: {:?}", hex::encode(&block.hash));
                    println!("PoW: {}", ProofOfWork::new_proof(&block).validate());
                    println!();
                    Ok(Some(()))
                }
                Err(err) => Err(err),
            },
        }
    }
}
