use std::{collections::HashMap, path::Path};

use super::{
    block::Block,
    transaction::Transaction,
    tx::{TxInput, TxOutput},
};

use crate::{
    blockchain::proof::ProofOfWork,
    wallet::{hash_public_key, WalletPrivateKey, Wallets},
    Error, Result,
};

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
    pub fn init_blockchain(address: &str) -> Result<Self> {
        if Path::new(DB_FILE).exists() {
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
        if !Path::new(DB_FILE).exists() {
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

    pub fn iterator(&self) -> BlockChainIterator {
        BlockChainIterator {
            current_hash: self.lasthash.clone(),
            database: self.database.clone(),
        }
    }

    pub fn find_utxo(&self, address: &str) -> Result<Vec<TxOutput>> {
        let mut utxos: Vec<TxOutput> = vec![];

        let public_key_hash = bs58::decode(address).into_vec()?;

        let unspent_txs = self.find_unspent_transactions(&public_key_hash)?;

        for tx in unspent_txs {
            for tx_output in tx.outputs {
                if tx_output.is_locked_with_key(&public_key_hash)? {
                    utxos.push(tx_output);
                }
            }
        }

        Ok(utxos)
    }

    pub fn new_transaction(&self, from: &str, to: &str, amount: u64) -> Result<Transaction> {
        let mut inputs = vec![];
        let mut outputs = vec![];

        let mut wallets = Wallets::create_wallets()?;
        let wallet = wallets.get_wallet(from).expect("Wallet is not exists");

        let public_key_hash = bs58::decode(from).into_vec()?;
        let (accumulated, valid_ouputs) = self.find_spendable_outputs(&public_key_hash, amount)?;

        if accumulated < amount {
            return Err(Error::CustomError("Not enough funds".to_owned()));
        }

        for (tx_id, outs) in valid_ouputs {
            let tx_id = hex::decode(&tx_id)?;

            for out in outs {
                inputs.push(TxInput {
                    id: tx_id.clone(),
                    out,
                    signature: vec![],
                    public_key: wallet.public_key.clone(),
                })
            }
        }

        outputs.push(TxOutput::new(amount, to)?);
        if accumulated > amount {
            outputs.push(TxOutput::new(accumulated - amount, from)?)
        }

        let mut tx = Transaction {
            id: vec![],
            inputs,
            outputs,
        };
        tx.hash()?;

        self.sign_transaction(&mut tx, &mut wallet.private_key)?;

        Ok(tx)
    }

    fn find_unspent_transactions(&self, public_key_hash: &[u8]) -> Result<Vec<Transaction>> {
        let mut unspent_txs: Vec<Transaction> = vec![];

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
                    if tx_output.is_locked_with_key(public_key_hash)? {
                        unspent_txs.push(tx.to_owned())
                    }
                }

                if !tx.is_coinbase() {
                    for tx_input in tx.inputs.iter() {
                        if tx_input.uses_key(public_key_hash) {
                            let tx_input_id = hex::encode(&tx_input.id);
                            spent_tx_outputs
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

    fn find_spendable_outputs(
        &self,
        public_key_hash: &[u8],
        amount: u64,
    ) -> Result<(u64, HashMap<String, Vec<i64>>)> {
        let mut unspent_outputs: HashMap<String, Vec<i64>> = HashMap::new();
        let mut accumulated = 0;

        let unspent_txs = self.find_unspent_transactions(public_key_hash)?;

        println!("unspent_txs {:?}", unspent_txs);

        'work: for tx in unspent_txs {
            let tx_id = hex::encode(&tx.id);
            for (out_index, tx_output) in tx.outputs.iter().enumerate() {
                if tx_output.is_locked_with_key(public_key_hash)? && accumulated < amount {
                    accumulated += tx_output.value;

                    unspent_outputs
                        .entry(tx_id.clone())
                        .or_default()
                        .push(out_index as i64);

                    if accumulated >= amount {
                        break 'work;
                    }
                }
            }
        }

        Ok((accumulated, unspent_outputs))
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

    fn sign_transaction(
        &self,
        tx: &mut Transaction,
        private_key: &mut WalletPrivateKey,
    ) -> Result<()> {
        let mut prev_txs: HashMap<String, Transaction> = HashMap::new();

        for input in tx.inputs.iter() {
            if let Some(prev_tx) = self.find_transaction(&input.id)? {
                prev_txs.insert(hex::encode(&prev_tx.id), prev_tx);
            } else {
                return Err(Error::CustomError(
                    "Previous transaction is not exists".to_owned(),
                ));
            }
        }

        tx.sign(private_key, &prev_txs)?;

        Ok(())
    }

    fn verify_transaction(&self, tx: &Transaction) -> Result<bool> {
        let mut prev_txs: HashMap<String, Transaction> = HashMap::new();

        for input in tx.inputs.iter() {
            if let Some(prev_tx) = self.find_transaction(&input.id)? {
                prev_txs.insert(hex::encode(&prev_tx.id), prev_tx);
            } else {
                return Err(Error::CustomError(
                    "Previous transaction is not exists".to_owned(),
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
                    println!("Prevhash: {:?}", hex::encode(&block.prevhash));
                    println!("Transactions:");
                    for tx in block.transactions.iter() {
                        println!("{}", tx);
                    }
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
