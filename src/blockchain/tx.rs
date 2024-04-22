use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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
