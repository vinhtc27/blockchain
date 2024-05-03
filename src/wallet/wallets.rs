use secp256k1::ecdsa::Signature;
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{create_dir_all, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

use crate::{Error, Result};

use super::wallet::{validate_address, Wallet};

static WALLET_PATH: &str = "./tmp/wallets/wallet";

#[derive(Serialize, Deserialize)]
pub struct Wallets {
    wallets: HashMap<String, Wallet>,
}

impl Wallets {
    pub fn create_wallets(node_id: &str) -> Result<Self> {
        let mut wallets = Wallets {
            wallets: HashMap::new(),
        };
        let wallet_path = &format!("{}_{}", WALLET_PATH, node_id);
        let wallet_file = &format!("{}/wallet.data", wallet_path);

        if Path::new(wallet_file).exists() {
            let mut file = File::open(wallet_file)?;
            let mut buffer = vec![];
            file.read_to_end(&mut buffer)?;
            wallets = bincode::deserialize(&buffer)?;
        } else {
            create_dir_all(wallet_path)?;
            File::create(wallet_file)?;
            wallets.save_file(node_id)?;
        }
        Ok(wallets)
    }

    pub fn add_wallet(&mut self) -> Result<String> {
        let wallet = Wallet::new()?;
        let address = wallet.address();
        self.wallets.insert(address.clone(), wallet);

        Ok(address)
    }

    pub fn get_wallet(&mut self, address: &str) -> Result<Option<&mut Wallet>> {
        if !validate_address(address)? {
            return Err(Error::CustomError("Wallet is invalid!".to_owned()));
        };

        if let Some(wallet) = self.wallets.get_mut(address) {
            Ok(Some(wallet))
        } else {
            Err(Error::CustomError("Address doesn't exists!".to_owned()))
        }
    }

    pub fn list_addresses(&self) -> Vec<String> {
        self.wallets.keys().cloned().collect()
    }

    pub fn save_file(&self, node_id: &str) -> Result<()> {
        let wallet_path = &format!("{}_{}", WALLET_PATH, node_id);
        let wallet_file = &format!("{}/wallet.data", wallet_path);

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(wallet_file)?;

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
