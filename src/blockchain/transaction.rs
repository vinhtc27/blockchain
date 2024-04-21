use sha2::{Digest, Sha256};
use speedy::{BigEndian, Readable, Writable};

use crate::Result;

#[derive(Debug, Clone, Readable, Writable)]
pub struct Transaction {
    pub(crate) id: Vec<u8>,
    pub(crate) inputs: Vec<TxInput>,
    pub(crate) outputs: Vec<TxOutput>,
}

impl Transaction {
    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(self.write_to_vec_with_ctx(BigEndian::default())?)
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
            value: 100,
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

#[derive(Debug, Clone, Readable, Writable)]
pub struct TxInput {
    pub(crate) id: Vec<u8>,
    pub(crate) out: i64,
    pub(crate) sig: String,
}

impl TxInput {
    pub(crate) fn can_unlock(&self, data: &str) -> bool {
        self.sig == data
    }
}

#[derive(Debug, Clone, Readable, Writable)]
pub struct TxOutput {
    pub(crate) value: u64,
    pub(crate) pubkey: String,
}

impl TxOutput {
    pub(crate) fn can_be_unlocked(&self, data: &str) -> bool {
        self.pubkey == data
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}
