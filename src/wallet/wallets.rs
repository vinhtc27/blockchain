use p256::ecdsa::Signature;
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{create_dir_all, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

use crate::{Error, Result};

use super::{validate_address, wallet::Wallet};

static WALLET_PATH: &str = "./tmp/wallet";
static WALLET_FILE: &str = "./tmp/wallet/wallets.data";

#[derive(Serialize, Deserialize)]
pub struct Wallets {
    wallets: HashMap<String, Wallet>,
}

impl Wallets {
    pub fn create_wallets() -> Result<Self> {
        let mut wallets = Wallets {
            wallets: HashMap::new(),
        };
        if Path::new(WALLET_FILE).exists() {
            let mut file = File::open(WALLET_FILE)?;
            let mut buffer = vec![];
            file.read_to_end(&mut buffer)?;
            wallets = bincode::deserialize(&buffer)?;
        } else {
            create_dir_all(WALLET_PATH)?;
            File::create(WALLET_FILE)?;
            wallets.save_file()?;
        }
        Ok(wallets)
    }

    pub fn add_wallet(&mut self) -> String {
        let wallet = Wallet::default();
        let address = wallet.address();
        self.wallets.insert(address.clone(), wallet);
        address
    }

    pub fn get_wallet(&mut self, address: &str) -> Result<Option<&mut Wallet>> {
        if !validate_address(address)? {
            return Err(Error::CustomError("Address is invalid!".to_owned()));
        };

        Ok(self.wallets.get_mut(address))
    }

    pub fn list_addresses(&self) -> Vec<String> {
        self.wallets.keys().cloned().collect()
    }

    pub fn save_file(&self) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(WALLET_FILE)?;

        let encoded = bincode::serialize(&self.wallets)?;
        file.write_all(&encoded)?;
        Ok(())
    }

    pub fn sign_tx(&mut self, tx_id: &[u8], address: &str) -> Result<Signature> {
        self.get_wallet(address)?
            .expect("Wallet doesn't exists!")
            .sign(tx_id)
    }
}
