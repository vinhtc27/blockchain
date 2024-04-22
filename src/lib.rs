pub mod blockchain;
pub mod cli;
pub mod wallet;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Sled error")]
    SledError {
        #[from]
        source: sled::Error,
    },
    #[error("Sled transaction error")]
    SledTransaction {
        #[from]
        source: sled::transaction::TransactionError,
    },
    #[error("Bincode error")]
    BincodeError {
        #[from]
        source: bincode::Error,
    },
    #[error("Hex error")]
    HexError {
        #[from]
        source: hex::FromHexError,
    },
    #[error("Base58 error")]
    Base58Error {
        #[from]
        source: bs58::decode::Error,
    },
    #[error("Io error")]
    IoError {
        #[from]
        source: std::io::Error,
    },
    #[error("Custom error")]
    CustomError(String),
}
