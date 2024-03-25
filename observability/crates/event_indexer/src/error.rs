use solana_sdk::{pubkey::Pubkey, signature::Signature};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexingError {
    #[error("Failed to parse account data for: {0}")]
    FailedToParseAccountData(Pubkey),

    #[error("Failed to fetch block for slot {0} after {1} retries")]
    BoundarySignatureNotFound(u64, u64),

    #[error("Failed to fetch signatures between {0:?} and {1:?}")]
    FailedToFetchSignatures(Signature, Signature),

    #[error("Failed to find slot for tx signature {0:?}")]
    FailedToFindTransactionSlot(Signature),

    #[error(transparent)]
    FailedToFetchEntity(FetchEntityError),

    #[error("Failed to insert event: {0}")]
    FailedToInsertEvent(String),

    #[error("An unknown error occurred")]
    Unknown,
}

#[derive(Error, Debug)]
pub enum FetchEntityError {
    #[error("Failed to fetch {0} entity: {1}")]
    FetchError(String, String),

    #[error("Failed to unpack {0} entity: {1}")]
    UnpackError(String, String),
}
