use std::collections::HashMap;

use sled::Batch;

use super::{
    block::Block,
    tx::{TxOutput, TxOutputs},
    BlockChain,
};

use crate::Result;

static UTXO_PREFIX: &[u8] = "utxo-".as_bytes();
static BATCH_SIZE: usize = 100000;

pub struct UTXOSet {
    pub(crate) chain: BlockChain,
}

impl UTXOSet {
    pub fn new(chain: BlockChain) -> Self {
        Self { chain }
    }

    pub fn reindex(&self) -> Result<()> {
        self.delete_by_prefix(UTXO_PREFIX)?;

        let mut batch = Batch::default();
        let all_utxo = self.chain.find_all_utxo()?;
        for (tx_id, tx_outputs) in all_utxo {
            let mut key = UTXO_PREFIX.to_vec();
            key.extend_from_slice(&hex::decode(tx_id)?);
            batch.insert(key, tx_outputs.serialize()?)
        }
        self.chain.database.apply_batch(batch)?;

        Ok(())
    }

    pub fn count_transaction(&self) -> usize {
        self.chain.database.scan_prefix(UTXO_PREFIX).count()
    }

    pub fn update(&self, block: &Block) -> Result<()> {
        self.chain.database.transaction(|db| {
            for tx in block.transactions.iter() {
                if !tx.is_coinbase() {
                    for tx_input in tx.inputs.iter() {
                        let mut update_tx_outputs = TxOutputs::new();
                        let mut tx_id = UTXO_PREFIX.to_vec();
                        tx_id.extend_from_slice(&tx_input.id);

                        let tx_outputs = TxOutputs::deserialize(&db.get(&tx_id)?.unwrap()).unwrap();

                        for (out_index, tx_output) in tx_outputs.outputs.into_iter().enumerate() {
                            if out_index != tx_input.out as usize {
                                update_tx_outputs.outputs.push(tx_output);
                            }
                        }

                        if update_tx_outputs.outputs.is_empty() {
                            db.remove(tx_id)?;
                        } else {
                            db.insert(tx_id, update_tx_outputs.serialize().unwrap())?;
                        }
                    }
                }

                let mut new_tx_outputs = TxOutputs::new();
                for out in tx.outputs.iter() {
                    new_tx_outputs.outputs.push(out.clone());
                }

                let mut tx_id = UTXO_PREFIX.to_vec();
                tx_id.extend_from_slice(&tx.id);
                db.insert(tx_id, new_tx_outputs.serialize().unwrap())?;
            }

            Ok(())
        })?;

        Ok(())
    }

    pub fn get_balance(&self, address: &str) -> Result<u64> {
        let mut address_utxo: Vec<TxOutput> = vec![];

        for bytes in self.chain.database.scan_prefix(UTXO_PREFIX).values() {
            let tx_outputs = TxOutputs::deserialize(&bytes?)?;
            for tx_output in tx_outputs.outputs {
                if tx_output.is_locked_with_key(address)? {
                    address_utxo.push(tx_output)
                }
            }
        }

        let mut balance = 0u64;
        for utxo in address_utxo {
            if utxo.is_locked_with_key(address)? {
                balance += utxo.value
            }
        }

        Ok(balance)
    }

    pub(crate) fn find_address_unspent_outputs(
        &self,
        address: &str,
        amount: u64,
    ) -> Result<(u64, HashMap<String, Vec<i64>>)> {
        let mut unspent_outputs: HashMap<String, Vec<i64>> = HashMap::new();
        let mut accumulated = 0;

        for item in self.chain.database.scan_prefix(UTXO_PREFIX) {
            let (key, bytes) = item?;

            let tx_id = hex::encode(
                key.iter()
                    .copied()
                    .skip(UTXO_PREFIX.len())
                    .collect::<Vec<u8>>(),
            );

            let tx_outputs = TxOutputs::deserialize(&bytes)?;
            for (out_index, tx_output) in tx_outputs.outputs.iter().enumerate() {
                if tx_output.is_locked_with_key(address)? && accumulated < amount {
                    accumulated += tx_output.value;
                    unspent_outputs
                        .entry(tx_id.clone())
                        .or_insert(vec![])
                        .push(out_index as i64);
                }
            }
        }

        Ok((accumulated, unspent_outputs))
    }

    fn delete_by_prefix(&self, prefix: &[u8]) -> Result<()> {
        let mut batchs: Vec<Batch> = vec![];
        let mut batch = Batch::default();
        let mut size = 0;

        for key in self.chain.database.scan_prefix(prefix).keys() {
            batch.remove(key?);
            size += 1;

            if size == BATCH_SIZE {
                batchs.push(batch);
                batch = Batch::default();
                size = 0;
            }
        }
        batchs.push(batch);

        self.chain.database.transaction(|db| {
            for batch in batchs.iter() {
                db.apply_batch(batch)?;
            }
            Ok(())
        })?;

        Ok(())
    }
}
