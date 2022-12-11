use anchor_lang::prelude::*;

#[error_code]
pub enum MarginfiError {
    #[msg("Math error")]
    MathError,
}

impl From<MarginfiError> for ProgramError {
    fn from(e: MarginfiError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
