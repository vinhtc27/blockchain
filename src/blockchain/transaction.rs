use std::{collections::HashMap, fmt};

use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{wallet::Wallets, Error, Result};

use super::{
    tx::{TxInput, TxOutput},
    utxo::UTXOSet,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub(crate) id: Vec<u8>,
    pub(crate) inputs: Vec<TxInput>,
    pub(crate) outputs: Vec<TxOutput>,
}

impl Transaction {
    pub(crate) fn hash(&mut self) -> Result<()> {
        let serialized_tx = bincode::serialize(&self)?;
        self.id = vec![];
        let mut tx_hash = [0u8; 32];
        tx_hash.copy_from_slice(&Sha256::digest(serialized_tx));
        self.id = tx_hash.to_vec();
        Ok(())
    }

    pub(crate) fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1 && self.inputs[0].id.is_empty() && self.inputs[0].out == -1
    }

    pub(crate) fn sign(
        &mut self,
        address: &str,
        prev_txs: &HashMap<String, Transaction>,
    ) -> Result<()> {
        if self.is_coinbase() {
            return Ok(());
        }

        for tx_input in &self.inputs {
            let prev_tx = prev_txs.get(&hex::encode(&tx_input.id));
            if prev_tx.is_none() || prev_tx.unwrap().id.is_empty() {
                return Err(Error::CustomError(
                    "Previous transaction does not exist!".to_owned(),
                ));
            }
        }

        let mut tx_copy = self.trimmed_copy()?;
        let tx_copy_inputs: Vec<_> = tx_copy.inputs.clone();
        for (in_index, tx_input) in tx_copy_inputs.iter().enumerate() {
            let prev_tx = prev_txs.get(&hex::encode(&tx_input.id)).unwrap();
            tx_copy.inputs[in_index].signature = vec![];
            tx_copy.inputs[in_index].public_key_hash = prev_tx.outputs[tx_input.out as usize]
                .public_key_hash
                .clone();

            tx_copy.hash()?;
            tx_copy.inputs[in_index].public_key_hash = vec![];

            let mut wallets = Wallets::create_wallets()?;
            let signature: Signature = wallets.sign_tx(&tx_copy.id, address)?;
            self.inputs[in_index].signature = signature.to_vec();
        }

        Ok(())
    }

    pub(crate) fn verify(&self, prev_txs: &HashMap<String, Transaction>) -> Result<bool> {
        if self.is_coinbase() {
            return Ok(true);
        }

        for tx_input in self.inputs.iter() {
            let prev_tx = prev_txs.get(&hex::encode(&tx_input.id));
            if prev_tx.is_none() || prev_tx.unwrap().id == vec![] {
                return Err(Error::CustomError(
                    "Previous transaction doesn't exists!".to_owned(),
                ));
            }
        }

        let mut tx_copy = self.trimmed_copy()?;
        let tx_copy_inputs: Vec<_> = tx_copy.inputs.clone();
        for (in_index, tx_input) in tx_copy_inputs.iter().enumerate() {
            let prev_tx = prev_txs.get(&hex::encode(&tx_input.id)).unwrap();
            tx_copy.inputs[in_index].signature = vec![];
            tx_copy.inputs[in_index].public_key_hash = prev_tx.outputs[tx_input.out as usize]
                .public_key_hash
                .clone();

            tx_copy.hash()?;
            tx_copy.inputs[in_index].public_key_hash = vec![];

            let public_key = VerifyingKey::from_sec1_bytes(&tx_input.public_key_hash)?;
            let signature = Signature::from_der(&tx_input.signature)?;

            if public_key.verify(&tx_copy.id, &signature).is_err() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub(crate) fn genesis(to: &str) -> Result<Self> {
        let tx_input = TxInput::new(vec![], -1, vec![], "Genesis")?;
        let tx_ouput = TxOutput::new(50, to)?;

        let mut tx = Transaction {
            id: vec![],
            inputs: vec![tx_input],
            outputs: vec![tx_ouput],
        };

        tx.hash()?;

        Ok(tx)
    }

    pub fn new(from: &str, to: &str, amount: u64, utxo_set: &UTXOSet) -> Result<Transaction> {
        let mut inputs = vec![];
        let mut outputs = vec![];

        let (accumulated, valid_ouputs) = utxo_set.find_address_unspent_outputs(from, amount)?;

        if accumulated < amount {
            return Err(Error::CustomError("Not enough funds".to_owned()));
        }

        for (tx_id, outs) in valid_ouputs {
            let tx_id = hex::decode(&tx_id)?;

            for out in outs {
                inputs.push(TxInput::new(tx_id.clone(), out, vec![], from)?);
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

        utxo_set.chain.sign_transaction(&mut tx, from)?;

        Ok(tx)
    }

    fn trimmed_copy(&self) -> Result<Self> {
        let mut inputs = vec![];
        let mut outputs = vec![];

        for tx_input in self.inputs.iter() {
            inputs.push(TxInput::new(tx_input.id.clone(), tx_input.out, vec![], "")?)
        }

        for tx_output in self.outputs.iter() {
            outputs.push(TxOutput {
                value: tx_output.value,
                public_key_hash: tx_output.public_key_hash.clone(),
            })
        }

        Ok(Self {
            id: self.id.clone(),
            inputs,
            outputs,
        })
    }
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tx: String = "".to_owned();

        tx.push_str(&format!(" # Id: {:?}\n", hex::encode(&self.id)));
        for (in_index, tx_input) in self.inputs.iter().enumerate() {
            tx.push_str(&format!(" + Input - index: {:?}\n", in_index));
            tx.push_str(&format!("         - id: {:?}\n", hex::encode(&tx_input.id)));
            tx.push_str(&format!("         - out: {:?}\n", tx_input.out));
            tx.push_str(&format!(
                "         - signature: {:?}\n",
                hex::encode(&tx_input.signature)
            ));
            tx.push_str(&format!(
                "         - public_key_hash: {:?}\n",
                hex::encode(&tx_input.public_key_hash)
            ));
        }
        tx.push_str(" \n");
        for (out_index, tx_output) in self.outputs.iter().enumerate() {
            tx.push_str(&format!(" + Output - index: {:?}\n", out_index));
            tx.push_str(&format!("          - value: {:?}\n", tx_output.value));
            tx.push_str(&format!(
                "          - public_key_hash: {:?}\n",
                hex::encode(&tx_output.public_key_hash)
            ));
        }
        tx.push_str(" \n");

        write!(f, "{tx}")
    }
}
