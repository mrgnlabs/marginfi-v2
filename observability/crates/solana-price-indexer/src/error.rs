use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexingError {
    #[error("An unknown error occurred")]
    Unknown,
}
