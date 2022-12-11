use anchor_lang::prelude::*;

#[error_code]
pub enum MarginfiError {
    #[msg("Math error")]
    MathError,
    #[msg("Invalid bank index")]
    BankNotFound,
    #[msg("Lending account balance not found")]
    LendingAccountBalanceNotFound,
    #[msg("Bank deposit capacity exceeded")]
    BankDepositCapacityExceeded
}

impl From<MarginfiError> for ProgramError {
    fn from(e: MarginfiError) -> Self {
        ProgramError::Custom(e as u32)
    }
}