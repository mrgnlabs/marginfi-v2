pub mod macros;
pub mod state;

use anchor_lang::{prelude::*, solana_program::entrypoint::ProgramResult};

declare_id!("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");

declare_program!(kamino_lending);
declare_program!(kamino_farms);

#[program]
pub mod kamino_mocks {}

#[error_code]
pub enum KaminoMocksError {
    #[msg("Math error")]
    MathError,
}

// A lightweight mock that accepts any ix and returns Ok(()).
// Used in Rust tests.
pub fn mock_kamino_lending_processor(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _ix_data: &[u8],
) -> ProgramResult {
    Ok(())
}
