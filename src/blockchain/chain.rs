use std::{collections::HashMap, path::Path};

use super::{block::Block, transaction::Transaction, tx::TxOutputs};

use crate::{blockchain::proof::ProofOfWork, Error, Result};

use sled::{Config, Db};

static LH_KEY: &[u8; 2] = b"LH";
static DB_PATH: &str = "./tmp/blocks";
static DB_FILE: &str = "./tmp/blocks/db";

pub struct BlockChain {
    pub lasthash: Vec<u8>,
    pub database: Db,
}

impl BlockChain {
    pub fn init_blockchain(address: &str) -> Result<Self> {
        if Path::new(DB_FILE).exists() {
            return Err(Error::CustomError(
                "Blockchain is already exists!".to_owned(),
            ));
        }
        let database: Db = Config::default().path(DB_PATH).open()?;
        let genesis = Block::genesis(Transaction::genesis(address)?);

        let lasthash = database.transaction(|db| {
            db.insert(genesis.hash.clone(), genesis.serialize().unwrap())?;
            db.insert(LH_KEY, genesis.hash.clone())?;
            Ok(genesis.hash.clone())
        })?;

        Ok(Self { lasthash, database })
    }

    pub fn continue_blockchain() -> Result<Self> {
        if !Path::new(DB_FILE).exists() {
            return Err(Error::CustomError("Blockchain doesn't exists!".to_owned()));
        }
        let database: Db = Config::default().path(DB_PATH).open()?;

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

    pub fn add_block(&mut self, transactions: Vec<Transaction>) -> Result<()> {
        for tx in transactions.iter() {
            if !self.verify_transaction(&tx)? {
                return Err(Error::CustomError("Invalid transaction!".to_owned()));
            }
        }

        let new_block = Block::create_block(transactions, self.lasthash.clone());
        self.database.transaction(|db| {
            db.insert(new_block.hash.clone(), new_block.serialize().unwrap())?;
            db.insert(LH_KEY, new_block.hash.clone())?;
            Ok(())
        })?;
        self.lasthash = new_block.hash;

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
                    let tx_spent_tx_outputs = spent_tx_outputs.get(&tx_id);
                    if tx_spent_tx_outputs.is_some() {
                        for spent_out in tx_spent_tx_outputs.unwrap() {
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
                    "Previous transaction doesn't exists!".to_owned(),
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

    fn verify_transaction(&self, tx: &Transaction) -> Result<bool> {
        let mut prev_txs: HashMap<String, Transaction> = HashMap::new();

        for tx_input in tx.inputs.iter() {
            if let Some(prev_tx) = self.find_transaction(&tx_input.id)? {
                prev_txs.insert(hex::encode(&prev_tx.id), prev_tx);
            } else {
                return Err(Error::CustomError(
                    "Previous transaction doesn't exists!".to_owned(),
                ));
            }
        }

        tx.verify(&prev_txs)
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
                    println!("PoW: {}", ProofOfWork::new_proof(&block).validate());
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
