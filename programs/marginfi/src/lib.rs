pub mod errors;
pub mod instructions;
pub mod macros;
pub mod prelude;
pub mod state;

use anchor_lang::prelude::*;
use instructions::*;
use prelude::*;
use static_assertions::assert_cfg;

#[cfg(feature = "mainnet-beta")] // mainnet
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
#[cfg(feature = "devnet")] // devnet
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
#[cfg(all(not(feature = "mainnet-beta"), not(feature = "devnet")))] // other
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

assert_cfg!(
    not(all(feature = "mainnet-beta", feature = "devnet")),
    "Devnet feature must be disabled for a mainnet release"
);
assert_cfg!(
    not(all(feature = "mainnet-beta", feature = "test")),
    "Test feature must be disabled for a mainnet release"
);

#[program]
pub mod marginfi {
    use super::*;

    pub fn initialize_marginfi_group(ctx: Context<InitializeMarginfiGroup>) -> MarginfiResult {
        initialize_marginfi_group::process(ctx)
    }
}
