use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeyserServiceError {
    #[error("parsing error in conversion from proto message")]
    ProtoMessageConversionFailed,
}
