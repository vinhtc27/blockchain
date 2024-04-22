use std::{collections::HashMap, fmt};

use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{wallet::WalletPrivateKey, Error, Result};

use super::tx::{TxInput, TxOutput};

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
        private_key: &mut WalletPrivateKey,
        prev_txs: &HashMap<String, Transaction>,
    ) -> Result<()> {
        if self.is_coinbase() {
            return Ok(());
        }

        for input in &self.inputs {
            let prev_tx = prev_txs.get(&hex::encode(&input.id));
            if prev_tx.is_none() || prev_tx.unwrap().id.is_empty() {
                return Err(Error::CustomError(
                    "Previous transaction does not exist (sign)".to_owned(),
                ));
            }
        }

        let mut tx_copy = self.trimmed_copy();
        let tx_copy_inputs: Vec<_> = tx_copy.inputs.clone();
        for (in_index, tx_input) in tx_copy_inputs.iter().enumerate() {
            let prev_tx = prev_txs.get(&hex::encode(&tx_input.id)).unwrap();
            tx_copy.inputs[in_index].signature = vec![];
            tx_copy.inputs[in_index].public_key = prev_tx.outputs[tx_input.out as usize]
                .public_key_hash
                .clone();

            tx_copy.hash()?;
            tx_copy.inputs[in_index].public_key = vec![];

            let signature: Signature = private_key.sign(&tx_copy.id);
            self.inputs[in_index].signature = signature.to_vec();
        }

        Ok(())
    }

    pub(crate) fn verify(&self, prev_txs: &HashMap<String, Transaction>) -> Result<bool> {
        if self.is_coinbase() {
            return Ok(true);
        }

        for input in self.inputs.iter() {
            let prev_tx = prev_txs.get(&hex::encode(&input.id));
            if prev_tx.is_none() || prev_tx.unwrap().id == vec![] {
                return Err(Error::CustomError(
                    "Previous transaction does not exists (verify)".to_owned(),
                ));
            }
        }

        let mut tx_copy = self.trimmed_copy();
        let tx_copy_inputs: Vec<_> = tx_copy.inputs.clone();
        for (in_index, tx_input) in tx_copy_inputs.iter().enumerate() {
            let prev_tx = prev_txs.get(&hex::encode(&tx_input.id)).unwrap();
            tx_copy.inputs[in_index].signature = vec![];
            tx_copy.inputs[in_index].public_key = prev_tx.outputs[tx_input.out as usize]
                .public_key_hash
                .clone();

            tx_copy.hash()?;
            tx_copy.inputs[in_index].public_key = vec![];

            let public_key = VerifyingKey::from_sec1_bytes(&tx_input.public_key)?;
            let signature = Signature::from_der(&tx_input.signature)?;

            if public_key.verify(&tx_copy.id, &signature).is_err() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub(crate) fn coinbase_tx(to: &str, mut data: String) -> Result<Self> {
        if data.is_empty() {
            data = format!("Coin to {}", to);
        }

        let tx_input = TxInput {
            id: vec![],
            out: -1,
            signature: vec![],
            public_key: data.as_bytes().to_vec(),
        };

        let tx_ouput = TxOutput::new(50, to)?; //? 50₿ to Satoshi Nakamoto

        let mut tx = Transaction {
            id: vec![],
            inputs: vec![tx_input],
            outputs: vec![tx_ouput],
        };

        tx.hash()?;

        Ok(tx)
    }

    fn trimmed_copy(&self) -> Self {
        let mut inputs = vec![];
        let mut outputs = vec![];

        for input in self.inputs.iter() {
            inputs.push(TxInput {
                id: input.id.clone(),
                out: input.out,
                signature: vec![],
                public_key: vec![],
            })
        }

        for output in self.outputs.iter() {
            outputs.push(TxOutput {
                value: output.value,
                public_key_hash: output.public_key_hash.clone(),
            })
        }

        Self {
            id: self.id.clone(),
            inputs,
            outputs,
        }
    }
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tx: String = "".to_owned();

        tx.push_str(&format!("--- Transaction: {:?}\n", hex::encode(&self.id)));
        for (in_index, tx_input) in self.inputs.iter().enumerate() {
            tx.push_str(&format!(" + Input - index: {:?}\n", in_index));
            tx.push_str(&format!("         - id: {:?}\n", hex::encode(&tx_input.id)));
            tx.push_str(&format!("         - out: {:?}\n", tx_input.out));
            tx.push_str(&format!(
                "         - signature: {:?}\n",
                hex::encode(&tx_input.signature)
            ));
            tx.push_str(&format!(
                "         - public_key: {:?}\n",
                hex::encode(&tx_input.public_key)
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

        write!(f, "{}", tx)
    }
}
