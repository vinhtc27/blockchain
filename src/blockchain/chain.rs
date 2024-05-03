use std::{collections::HashMap, path::Path};

use super::{block::Block, transaction::Transaction, tx::TxOutputs};

use crate::{blockchain::proof::ProofOfWork, Error, Result};

use sled::{Batch, Config, Db};

static LH_KEY: &[u8; 2] = b"LH";
static DB_PATH: &str = "./tmp/blocks/block";

#[derive(Clone)]
pub struct BlockChain {
    pub lasthash: Vec<u8>,
    pub database: Db,
}

impl BlockChain {
    pub fn init_blockchain(node_id: &str, address: &str) -> Result<Self> {
        let database_path = &format!("{}_{}", DB_PATH, node_id);
        let database_file = &format!("{}/db", database_path);

        if Path::new(database_file).exists() {
            return Err(Error::CustomError(
                "Blockchain is already exists!".to_owned(),
            ));
        }

        let genesis = Block::genesis(Transaction::coinbase_tx(address)?)?;
        let database: Db = Config::default().path(database_path).open()?;

        let mut batch = Batch::default();
        batch.insert(genesis.hash.clone(), genesis.serialize()?);
        batch.insert(LH_KEY, genesis.hash.clone());
        database.apply_batch(batch)?;

        Ok(Self {
            lasthash: genesis.hash,
            database,
        })
    }

    pub fn continue_blockchain(node_id: &str) -> Result<Self> {
        let database_path = &format!("{}_{}", DB_PATH, node_id);
        let database_file = &format!("{}/db", database_path);

        if !Path::new(database_file).exists() {
            return Err(Error::CustomError("Blockchain doesn't exists!".to_owned()));
        }
        let database: Db = Config::default().path(database_path).open()?;

        let lasthash = match database.get(LH_KEY)? {
            Some(lh) => lh.as_ref().into(),
            None => return Err(Error::CustomError("LH_KEY doesn't exists!".to_owned())),
        };

        Ok(Self { lasthash, database })
    }

    pub fn iterator(&self) -> BlockChainIterator {
        BlockChainIterator {
            current_hash: self.lasthash.clone(),
            database: self.database.clone(),
        }
    }

    pub fn mine_block(&mut self, transactions: Vec<Transaction>) -> Result<Block> {
        for tx in transactions.iter() {
            self.verify_transaction(tx)?;
        }

        let lasthash = self.database.get(LH_KEY)?.unwrap().to_vec();
        let last_block_bytes = self.database.get(lasthash.clone())?.unwrap();
        let last_block: Block = bincode::deserialize(&last_block_bytes)?;

        let new_block = Block::create_block(transactions, lasthash, last_block.height + 1)?;

        let mut batch = Batch::default();
        batch.insert(new_block.hash.clone(), new_block.serialize()?);
        batch.insert(LH_KEY, new_block.hash.clone());
        self.database.apply_batch(batch)?;

        self.lasthash = new_block.hash.clone();

        Ok(new_block)
    }

    pub fn add_block(&mut self, block: &Block) -> Result<()> {
        if self.database.get(&block.hash)?.is_none() {
            self.database
                .insert(&block.hash, bincode::serialize(block)?)?;

            let lasthash = self.database.get(LH_KEY)?.unwrap().to_vec();
            let last_block_bytes = self.database.get(lasthash)?.unwrap();
            let last_block: Block = bincode::deserialize(&last_block_bytes)?;

            if block.height > last_block.height {
                self.database.insert(LH_KEY, block.hash.clone())?;
                self.lasthash = block.hash.clone();
            }
        }

        Ok(())
    }

    pub fn get_block(&self, block_hash: &[u8]) -> Result<Option<Block>> {
        let block_bytes = self.database.get(block_hash)?;
        if block_bytes.is_some() {
            let block: Block = bincode::deserialize(&block_bytes.unwrap())?;
            return Ok(Some(block));
        }

        Ok(None)
    }

    pub fn get_block_hashes(&self) -> Result<Vec<Vec<u8>>> {
        let mut hashes = vec![];
        let mut iter = self.iterator();

        while let Some(block) = iter.next()? {
            hashes.push(block.hash)
        }

        Ok(hashes)
    }

    pub fn get_best_height(&self) -> Result<u64> {
        let lasthash = self.database.get(LH_KEY)?.unwrap().to_vec();
        let last_block_bytes = self.database.get(lasthash)?.unwrap();
        let last_block: Block = bincode::deserialize(&last_block_bytes).unwrap();

        Ok(last_block.height)
    }

    pub fn verify_transaction(&self, tx: &Transaction) -> Result<()> {
        if tx.is_coinbase() {
            return Ok(());
        }

        let mut prev_txs: HashMap<String, Transaction> = HashMap::new();

        for tx_input in tx.inputs.iter() {
            if let Some(prev_tx) = self.find_transaction(&tx_input.id)? {
                prev_txs.insert(hex::encode(&prev_tx.id), prev_tx);
            } else {
                return Err(Error::CustomError(
                    "Previous transaction doesn't exists (verify)!".to_owned(),
                ));
            }
        }

        tx.verify(&prev_txs)?;

        Ok(())
    }

    pub(crate) fn find_all_utxo(&self) -> Result<HashMap<String, TxOutputs>> {
        let mut utxo: HashMap<String, TxOutputs> = HashMap::new();

        let mut spent_tx_outputs: HashMap<String, Vec<i64>> = HashMap::new();

        let mut iter = self.iterator();
        while let Some(block) = iter.next()? {
            for tx in block.transactions.iter() {
                let tx_id = hex::encode(&tx.id);

                'outputs: for (out_index, tx_output) in tx.outputs.iter().enumerate() {
                    if let Some(tx_spent_tx_outputs) = spent_tx_outputs.get(&tx_id) {
                        for spent_out in tx_spent_tx_outputs {
                            if *spent_out == out_index as i64 {
                                continue 'outputs;
                            }
                        }
                    }

                    utxo.entry(tx_id.clone())
                        .or_insert(TxOutputs::new())
                        .outputs
                        .push(tx_output.clone());
                }

                if !tx.is_coinbase() {
                    for tx_input in tx.inputs.iter() {
                        let tx_input_id = hex::encode(&tx_input.id);
                        spent_tx_outputs
                            .entry(tx_input_id)
                            .or_default()
                            .push(tx_input.out);
                    }
                }
            }

            if block.prevhash.is_empty() {
                break;
            }
        }

        Ok(utxo)
    }

    pub(crate) fn sign_transaction(&self, tx: &mut Transaction, address: &str) -> Result<()> {
        let mut prev_txs: HashMap<String, Transaction> = HashMap::new();

        for tx_input in tx.inputs.iter() {
            if let Some(prev_tx) = self.find_transaction(&tx_input.id)? {
                prev_txs.insert(hex::encode(&prev_tx.id), prev_tx);
            } else {
                return Err(Error::CustomError(
                    "Previous transaction doesn't exists (sign)!".to_owned(),
                ));
            }
        }

        tx.sign(address, &prev_txs)?;

        Ok(())
    }

    fn find_transaction(&self, id: &[u8]) -> Result<Option<Transaction>> {
        let mut iter = self.iterator();
        while let Some(block) = iter.next()? {
            for tx in block.transactions {
                if tx.id == id {
                    return Ok(Some(tx));
                }
            }

            if block.prevhash.is_empty() {
                break;
            }
        }

        Ok(None)
    }
}

pub struct BlockChainIterator {
    current_hash: Vec<u8>,
    database: Db,
}

impl BlockChainIterator {
    pub(crate) fn next(&mut self) -> Result<Option<Block>> {
        match self.database.get(&self.current_hash)? {
            None => Ok(None),
            Some(bytes) => match Block::deserialize(&bytes) {
                Ok(block) => {
                    self.current_hash = block.prevhash.clone();
                    Ok(Some(block))
                }
                Err(err) => Err(err),
            },
        }
    }

    pub fn next_print(&mut self) -> Result<Option<()>> {
        match self.database.get(&self.current_hash)? {
            None => Ok(None),
            Some(bytes) => match Block::deserialize(&bytes) {
                Ok(block) => {
                    self.current_hash = block.prevhash.clone();
                    println!("PoW: {}", ProofOfWork::new_proof(&block).validate()?);
                    println!("Hash: {:?}", hex::encode(&block.hash));
                    println!("Prevhash: {:?}", hex::encode(&block.prevhash));
                    println!("Transactions:");
                    for tx in block.transactions.iter() {
                        println!("{tx}");
                    }
                    Ok(Some(()))
                }
                Err(err) => Err(err),
            },
        }
    }
}
