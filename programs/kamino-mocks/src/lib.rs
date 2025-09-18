pub mod constants;
pub mod macros;
pub mod state;

use anchor_lang::prelude::*;

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
