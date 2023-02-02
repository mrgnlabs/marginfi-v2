pub mod constants;
pub mod errors;
pub mod instructions;
pub mod macros;
pub mod prelude;
pub mod state;
pub mod utils;

use anchor_lang::prelude::*;
use instructions::*;
use prelude::*;
use state::marginfi_group::{BankConfig, BankConfigOpt};
use static_assertions::assert_cfg;

#[cfg(feature = "mainnet-beta")] // mainnet
declare_id!("yyyxaNHJP5FiDhmQW8RkBkp1jTL2cyxJmhMdWpJfsiy");
#[cfg(feature = "devnet")] // devnet
declare_id!("uwuyG6VmYrDk8Q3FfQ7wZhuhbi8ExweW74BrmT3vM1i");
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

    pub fn marginfi_group_initialize(ctx: Context<InitializeMarginfiGroup>) -> MarginfiResult {
        marginfi_group::initialize(ctx)
    }

    pub fn marginfi_group_configure(
        ctx: Context<ConfigureMarginfiGroup>,
        config: GroupConfig,
    ) -> MarginfiResult {
        marginfi_group::configure(ctx, config)
    }

    pub fn lending_pool_add_bank(
        ctx: Context<LendingPoolAddBank>,
        bank_config: BankConfig,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_add_bank(ctx, bank_config)
    }

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
    pub fn marginfi_account_initialize(ctx: Context<MarginfiAccountInitialize>) -> MarginfiResult {
        marginfi_account::initialize(ctx)
    }

    pub fn lending_pool_deposit(ctx: Context<LendingPoolDeposit>, amount: u64) -> MarginfiResult {
        marginfi_account::lending_pool_deposit(ctx, amount)
    }

    pub fn lending_pool_repay(
        ctx: Context<LendingPoolRepay>,
        amount: u64,
        repay_all: Option<bool>,
    ) -> MarginfiResult {
        marginfi_account::lending_pool_repay(ctx, amount, repay_all)
    }

    pub fn lending_pool_withdraw(
        ctx: Context<LendingPoolWithdraw>,
        amount: u64,
        withdraw_all: Option<bool>,
    ) -> MarginfiResult {
        marginfi_account::lending_pool_withdraw(ctx, amount, withdraw_all)
    }

    pub fn lending_pool_borrow(ctx: Context<LendingPoolBorrow>, amount: u64) -> MarginfiResult {
        marginfi_account::lending_pool_borrow(ctx, amount)
    }

    /// Liquidate a lending account balance of an unhealthy marginfi account
    pub fn marginfi_account_liquidate(
        ctx: Context<MarginfiAccountLiquidate>,
        asset_amount: u64,
    ) -> MarginfiResult {
        marginfi_account::liquidate(ctx, asset_amount)
    }

    // Operational instructions
    pub fn lending_pool_bank_accrue_interest(
        ctx: Context<LendingPoolBankAccrueInterest>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_bank_accrue_interest(ctx)
    }

    pub fn lending_pool_bank_collect_fees(
        ctx: Context<LendingPoolBankCollectFees>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_bank_collect_fees(ctx)
    }
}
