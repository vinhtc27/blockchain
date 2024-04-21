pub mod blockchain;

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
    #[error("Speedy error")]
    SpeedyError {
        #[from]
        source: speedy::Error,
    },
    #[error("Hex error")]
    HexError {
        #[from]
        source: hex::FromHexError,
    },
    #[error("Custom error")]
    CustomError(String),
}
