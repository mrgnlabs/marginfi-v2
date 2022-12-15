pub mod constants;
pub mod errors;
pub mod instructions;
pub mod macros;
pub mod prelude;
pub mod state;

use anchor_lang::prelude::*;
use instructions::*;
use prelude::*;
use state::marginfi_group::{BankConfig, BankConfigOpt};
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
/// - `lending_pool_add_bank` - Adds a bank to a lending pool.
/// - `lending_pool_configure_bank` - Configures a bank in a lending pool.
///
/// User instructions:
/// - `create_margin_account` - Creates a new margin account.
/// - `lending_pool_deposit` - Deposits liquidity into a bank.
/// - `lending_pool_withdraw` - Withdraws liquidity from a bank.
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

    pub fn configure_marginfi_group(
        ctx: Context<ConfigureMarginfiGroup>,
        config: GroupConfig,
    ) -> MarginfiResult {
        marginfi_group::configure(ctx, config)
    }

    pub fn lending_pool_add_bank(
        ctx: Context<LendingPoolAddBank>,
        bank_index: u16,
        bank_config: BankConfig,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_add_bank(ctx, bank_index, bank_config)
    }

    pub fn lending_pool_configure_bank(
        ctx: Context<LendingPoolConfigureBank>,
        bank_index: u8,
        bank_config_opt: BankConfigOpt,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_configure_bank(ctx, bank_index, bank_config_opt)
    }

    // User instructions
    pub fn create_margin_account(ctx: Context<CreateMarginfiAccount>) -> MarginfiResult {
        marginfi_account::create(ctx)
    }

    pub fn bank_deposit(ctx: Context<LendingPoolDeposit>, amount: u64) -> MarginfiResult {
        marginfi_account::bank_deposit(ctx, amount)
    }

    pub fn bank_withdraw(ctx: Context<LendingPoolWithdraw>, amount: u64) -> MarginfiResult {
        marginfi_account::bank_withdraw(ctx, amount)
    }
}
