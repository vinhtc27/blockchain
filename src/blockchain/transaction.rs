use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Result;

use super::tx::{TxInput, TxOutput};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub(crate) id: Vec<u8>,
    pub(crate) inputs: Vec<TxInput>,
    pub(crate) outputs: Vec<TxOutput>,
}

impl Transaction {
    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    pub(crate) fn set_id(&mut self) -> Result<()> {
        let encoded_tx = self.serialize()?;
        let mut id_hash = [0u8; 32];
        id_hash.copy_from_slice(&Sha256::digest(encoded_tx));
        self.id = id_hash.to_vec();
        Ok(())
    }

    pub(crate) fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1 && self.inputs[0].id.is_empty() && self.inputs[0].out == -1
    }

    pub(crate) fn coinbase_tx(to: String, mut data: String) -> Result<Self> {
        if data.is_empty() {
            data = format!("Coin to {}", to);
        }

        let txin = TxInput {
            id: vec![],
            out: -1,
            sig: data,
        };
        let tout = TxOutput {
            value: 50, //? 50₿ to Satoshi Nakamoto
            pubkey: to,
        };

        let mut tx = Transaction {
            id: vec![],
            inputs: vec![txin],
            outputs: vec![tout],
        };

        tx.set_id()?;

        Ok(tx)
    }
}
