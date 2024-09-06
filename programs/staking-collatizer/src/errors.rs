use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Math error")] // 6000
    MathError,
}