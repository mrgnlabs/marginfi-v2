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
/// Marginfi v2 program entrypoint.
/// 
/// Instructions:
/// Admin instructions:
/// - `initialize_marginfi_group` - Initializes a new marginfi group.
/// - `configure_marginfi_group` - Configures a marginfi group.
/// - `lending_pool_add_reserve` - Adds a reserve to a lending pool.
/// - `lending_pool_configure_reserve` - Configures a reserve in a lending pool.
/// 
/// User instructions:
/// - `create_margin_account` - Creates a new margin account.
/// - `lending_pool_deposit` - Deposits liquidity into a reserve.
/// - `lending_pool_withdraw` - Withdraws liquidity from a reserve.
/// - `liquidate` - Liquidates a margin account.
/// 
/// Operational instructions:
/// - `accrue_interest` - Accrues interest for a reserve.

#[program]
pub mod marginfi {
    use super::*;

    pub fn initialize_marginfi_group(ctx: Context<InitializeMarginfiGroup>) -> MarginfiResult {
        marginfi_group::initialize(ctx)
    }


    // User instructions
    pub fn create_margin_account(ctx: Context<CreateMarginfiAccount>) -> MarginfiResult {
        margin_account::create(ctx)
    }
}
