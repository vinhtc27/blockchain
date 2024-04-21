use std::collections::HashMap;

use super::{
    block::Block,
    transaction::{Transaction, TxInput, TxOutput},
};

use crate::{blockchain::proof::ProofOfWork, Error, Result};

use sled::{Config, Db};

static LH_KEY: &[u8; 2] = b"LH";
static DB_PATH: &str = "./tmp/blocks";
static DB_FILE: &str = "./tmp/blocks/db";
static GENESIS_DATA: &str = "First Transaction from Genesis";

pub struct BlockChain {
    lasthash: Vec<u8>,
    database: Db,
}

impl BlockChain {
    pub fn init_blockchain(address: String) -> Result<Self> {
        if std::fs::metadata(DB_FILE).is_ok() {
            return Err(Error::CustomError(
                "Blockchain is already exists!".to_owned(),
            ));
        }
        let database: Db = Config::default().path(DB_PATH).open()?;
        let coinbase_tx = Transaction::coinbase_tx(address, GENESIS_DATA.to_owned())?;
        let genesis = Block::genesis(coinbase_tx);

        let lasthash = database.transaction(|db| {
            db.insert(genesis.hash.clone(), genesis.serialize().unwrap())?;
            db.insert(LH_KEY, genesis.hash.clone())?;
            Ok(genesis.hash.clone())
        })?;

        Ok(Self { lasthash, database })
    }

    pub fn continue_blockchain() -> Result<Self> {
        if std::fs::metadata(DB_FILE).is_err() {
            return Err(Error::CustomError(
                "Blockchain is not exists, create one!".to_owned(),
            ));
        }
        let database: Db = Config::default().path(DB_PATH).open()?;

        let lasthash = match database.get(LH_KEY)? {
            Some(lh) => lh.as_ref().into(),
            None => return Err(Error::CustomError("LH is not exists!".to_owned())),
        };

        Ok(Self { lasthash, database })
    }

    pub fn add_block(&mut self, transactions: Vec<Transaction>) -> Result<()> {
        let new_block = Block::create_block(transactions, self.lasthash.clone());
        self.database.transaction(|db| {
            db.insert(new_block.hash.clone(), new_block.serialize().unwrap())?;
            db.insert(LH_KEY, new_block.hash.clone())?;

            Ok(())
        })?;

        Ok(())
    }

    pub fn new_txs(&self, from: &str, to: &str, amount: u64) -> Result<Transaction> {
        let mut inputs: Vec<TxInput> = vec![];
        let mut outputs: Vec<TxOutput> = vec![];

        let (accumulated, valid_ouputs) = self.find_spendable_outputs(from, amount)?;

        if accumulated < amount {
            return Err(Error::CustomError("Not enough funds".to_owned()));
        }

        for (tx_id, outputs) in valid_ouputs {
            for output in outputs {
                inputs.push(TxInput {
                    id: hex::decode(&tx_id)?,
                    out: output,
                    sig: from.to_owned(),
                })
            }
        }

        outputs.push(TxOutput {
            value: amount,
            pubkey: to.to_owned(),
        });

        if accumulated > amount {
            outputs.push(TxOutput {
                value: accumulated - amount,
                pubkey: from.to_owned(),
            });
        }

        let mut tx = Transaction {
            id: vec![],
            inputs,
            outputs,
        };
        tx.set_id()?;

        Ok(tx)
    }

    pub fn iterator(&self) -> BlockChainIterator {
        BlockChainIterator {
            current_hash: self.lasthash.clone(),
            database: self.database.clone(),
        }
    }

    fn find_unspent_transactions(&self, address: &str) -> Result<Vec<Transaction>> {
        let mut unspent_txs: Vec<Transaction> = vec![];

        let mut spent_txos: HashMap<String, Vec<i64>> = HashMap::new();

        let mut iter = self.iterator();
        while let Some(block) = iter.next()? {
            for tx in block.transactions.iter() {
                let tx_id = hex::encode(&tx.id);

                'outputs: for (out_index, tx_output) in tx.outputs.iter().enumerate() {
                    let tx_spent_txos = spent_txos.get(&tx_id);
                    if tx_spent_txos.is_some() {
                        for spent_out in tx_spent_txos.unwrap() {
                            if *spent_out == out_index as i64 {
                                continue 'outputs;
                            }
                        }
                    }
                    if tx_output.can_be_unlocked(address) {
                        unspent_txs.push(tx.to_owned())
                    }
                }

                if !tx.is_coinbase() {
                    for tx_input in tx.inputs.iter() {
                        if tx_input.can_unlock(address) {
                            let tx_input_id = hex::encode(&tx_input.id);
                            spent_txos
                                .entry(tx_input_id)
                                .or_default()
                                .push(tx_input.out);
                        }
                    }
                }
            }

            if block.prevhash.is_empty() {
                break;
            }
        }

        Ok(unspent_txs)
    }

    pub fn find_utxo(&self, address: &str) -> Result<Vec<TxOutput>> {
        let mut utxos: Vec<TxOutput> = vec![];

        let unspent_txs = self.find_unspent_transactions(address)?;

        for tx in unspent_txs {
            for tx_output in tx.outputs {
                if tx_output.can_be_unlocked(address) {
                    utxos.push(tx_output);
                }
            }
        }

        Ok(utxos)
    }

    pub(crate) fn find_spendable_outputs(
        &self,
        address: &str,
        amount: u64,
    ) -> Result<(u64, HashMap<String, Vec<i64>>)> {
        let mut unspent_outputs: HashMap<String, Vec<i64>> = HashMap::new();
        let mut accumulated = 0;

        let unspent_txs = self.find_unspent_transactions(address)?;

        'work: for tx in unspent_txs {
            for (output_index, output) in tx.outputs.iter().enumerate() {
                if output.can_be_unlocked(address) && accumulated < amount {
                    accumulated += output.value;

                    let tx_id = hex::encode(&tx.id);
                    unspent_outputs
                        .entry(tx_id)
                        .or_default()
                        .push(output_index as i64);

                    if accumulated >= amount {
                        break 'work;
                    }
                }
            }
        }

        Ok((accumulated, unspent_outputs))
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
                    println!("Prev. hash: {:?}", hex::encode(&block.prevhash));
                    println!("Transactions: {:?}", &block.transactions);
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
