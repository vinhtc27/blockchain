use std::{collections::HashMap, fmt};

use secp256k1::{
    ecdsa::Signature,
    rand::{rngs::OsRng, RngCore},
    Message, PublicKey,
};
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    wallet::{public_key_hash_from_address, Wallets},
    Error, Result,
};

use super::{
    tx::{TxInput, TxOutput},
    utxo::UTXOSet,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub(crate) id: Vec<u8>,
    pub(crate) inputs: Vec<TxInput>,
    pub(crate) outputs: Vec<TxOutput>,
}

impl Transaction {
    pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    pub fn new(
        node_id: &str,
        from: &str,
        to: &str,
        amount: u64,
        utxo_set: &UTXOSet,
    ) -> Result<Transaction> {
        let mut inputs = vec![];
        let mut outputs = vec![];

        let (accumulated, valid_ouputs) = utxo_set.find_address_unspent_outputs(from, amount)?;

        if accumulated < amount {
            return Err(Error::CustomError("Address funds isn't enough!".to_owned()));
        }

        for (tx_id, outs) in valid_ouputs {
            let tx_id = hex::decode(&tx_id)?;

            for out in outs {
                inputs.push(TxInput::new(
                    tx_id.clone(),
                    out,
                    vec![],
                    public_key_hash_from_address(from)?,
                )?);
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

        utxo_set.chain.sign_transaction(node_id, &mut tx, from)?;

        Ok(tx)
    }

    pub(crate) fn coinbase_tx(to: &str) -> Result<Self> {
        let mut random = [0u8; 24];
        OsRng.fill_bytes(&mut random);

        let tx_input = TxInput::new(vec![], -1, vec![], random.to_vec())?;
        let tx_ouput = TxOutput::new(20, to)?; //? Reward 20

        let mut tx = Transaction {
            id: vec![],
            inputs: vec![tx_input],
            outputs: vec![tx_ouput],
        };

        tx.hash()?;

        Ok(tx)
    }

    pub(crate) fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1 && self.inputs[0].id.is_empty() && self.inputs[0].out == -1
    }

    pub(crate) fn hash(&mut self) -> Result<()> {
        let serialized_tx = bincode::serialize(&self)?;
        self.id = vec![];
        let mut tx_hash = [0u8; 32];
        tx_hash.copy_from_slice(&Sha256::digest(serialized_tx));
        self.id = tx_hash.to_vec();
        Ok(())
    }

    pub(crate) fn sign(
        &mut self,
        node_id: &str,
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
        let tx_copy_inputs = tx_copy.inputs.clone();
        for (in_index, tx_input) in tx_copy_inputs.iter().enumerate() {
            let prev_tx = prev_txs.get(&hex::encode(&tx_input.id)).unwrap();
            tx_copy.inputs[in_index].signature = vec![];
            tx_copy.inputs[in_index].public_key_hash = prev_tx.outputs[tx_input.out as usize]
                .public_key_hash
                .clone();

            tx_copy.hash()?;
            tx_copy.inputs[in_index].public_key_hash = vec![];

            let mut wallets = Wallets::create_wallets(node_id)?;
            let signature: Signature = wallets.sign_tx(&tx_copy.id, address)?;
            self.inputs[in_index].signature = signature.serialize_der().to_vec();
        }

        Ok(())
    }

    pub(crate) fn verify(&self, prev_txs: &HashMap<String, Transaction>) -> Result<()> {
        if self.is_coinbase() {
            return Ok(());
        }

        for tx_input in self.inputs.iter() {
            let prev_tx = prev_txs.get(&hex::encode(&tx_input.id));
            if prev_tx.is_none() || prev_tx.unwrap().id == vec![] {
                return Err(Error::CustomError(
                    "Previous transaction id is empty!".to_owned(),
                ));
            }
        }

        let mut tx_copy = self.trimmed_copy()?;
        let tx_copy_inputs = self.inputs.clone();
        for (in_index, tx_input) in tx_copy_inputs.iter().enumerate() {
            let prev_tx = prev_txs.get(&hex::encode(&tx_input.id)).unwrap();
            tx_copy.inputs[in_index].signature = vec![];
            tx_copy.inputs[in_index].public_key_hash = prev_tx.outputs[tx_input.out as usize]
                .public_key_hash
                .clone();

            tx_copy.hash()?;
            tx_copy.inputs[in_index].public_key_hash = vec![];

            let signature = Signature::from_der(&self.inputs[in_index].signature)?;
            let public_key = PublicKey::from_slice(&tx_input.public_key_hash)?;

            let digest = Sha256::digest(&tx_copy.id);
            let message = Message::from_digest(digest.into());

            signature.verify(&message, &public_key)?
        }

        Ok(())
    }

    fn trimmed_copy(&self) -> Result<Self> {
        let mut inputs = vec![];
        let mut outputs = vec![];

        for tx_input in self.inputs.iter() {
            inputs.push(TxInput::new(
                tx_input.id.clone(),
                tx_input.out,
                vec![],
                vec![],
            )?)
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

        tx.push_str(&format!("  + Id: {:?}\n", hex::encode(&self.id)));
        for (in_index, tx_input) in self.inputs.iter().enumerate() {
            tx.push_str(&format!("  + In:   - index: {:?}\n", in_index));
            tx.push_str(&format!(
                "          - id: {:?}\n",
                hex::encode(&tx_input.id)
            ));
            tx.push_str(&format!("          - out: {:?}\n", tx_input.out));
            tx.push_str(&format!(
                "          - signature: {:?}\n",
                hex::encode(&tx_input.signature)
            ));
            tx.push_str(&format!(
                "          - public_key_hash: {:?}\n",
                hex::encode(&tx_input.public_key_hash)
            ));
        }
        tx.push_str(" \n");
        for (out_index, tx_output) in self.outputs.iter().enumerate() {
            tx.push_str(&format!("  + Out:  - index: {:?}\n", out_index));
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
