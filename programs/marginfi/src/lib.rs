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
declare_id!("HfHBtENWH9C27kXMwP62WCSMm734kzKj9YnzUaHPzk6i");
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

    /// Initialize a new Marginfi Group with initial config
    pub fn initialize_marginfi_group(ctx: Context<InitializeMarginfiGroup>) -> MarginfiResult {
        marginfi_group::initialize(ctx)
    }

    /// Configure a Marginfi Group
    pub fn configure_marginfi_group(
        ctx: Context<ConfigureMarginfiGroup>,
        config: GroupConfig,
    ) -> MarginfiResult {
        marginfi_group::configure(ctx, config)
    }

    /// Add a new bank to the Marginfi Group
    pub fn lending_pool_add_bank(
        ctx: Context<LendingPoolAddBank>,
        bank_config: BankConfig,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_add_bank(ctx, bank_config)
    }

    /// Configure a bank in the Marginfi Group
    pub fn lending_pool_configure_bank(
        ctx: Context<LendingPoolConfigureBank>,
        bank_config_opt: BankConfigOpt,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_configure_bank(ctx, bank_config_opt)
    }

    /// Handle bad debt of a bankrupt marginfi account for a given bank.
    pub fn lending_pool_handle_bankruptcy(
        ctx: Context<LendingPoolHandleBankruptcy>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_handle_bankruptcy(ctx)
    }

    // User instructions
    /// Initialize a marginfi account for a given group
    pub fn initialize_marginfi_account(ctx: Context<InitializeMarginfiAccount>) -> MarginfiResult {
        marginfi_account::initialize(ctx)
    }

    /// Deposit assets into a lending account
    /// Repay borrowed assets, if any exist.
    pub fn bank_deposit(ctx: Context<BankDeposit>, amount: u64) -> MarginfiResult {
        marginfi_account::bank_deposit(ctx, amount)
    }

    /// Withdraw assets from a lending account
    /// Withdraw deposited assets, if any exist, otherwise borrow assets.
    /// Account health checked.
    pub fn bank_withdraw(ctx: Context<BankWithdraw>, amount: u64) -> MarginfiResult {
        marginfi_account::bank_withdraw(ctx, amount)
    }

    /// Liquidate a lending account balance of an unhealthy marginfi account
    pub fn lending_account_liquidate(
        ctx: Context<LendingAccountLiquidate>,
        asset_amount: u64,
    ) -> MarginfiResult {
        marginfi_account::lending_account_liquidate(ctx, asset_amount)
    }

    // Operational instructions
    pub fn bank_accrue_interest(ctx: Context<LendingPoolBankAccrueInterest>) -> MarginfiResult {
        marginfi_group::lending_pool_bank_accrue_interest(ctx)
    }
}
