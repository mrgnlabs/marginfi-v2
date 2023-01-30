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

    pub fn marginfi_account_deposit(
        ctx: Context<MarginfiAccountDeposit>,
        amount: u64,
    ) -> MarginfiResult {
        marginfi_account::deposit(ctx, amount)
    }

    pub fn marginfi_account_withdraw(
        ctx: Context<MarginfiAccountWithdraw>,
        amount: u64,
        withdraw_all: Option<bool>,
    ) -> MarginfiResult {
        marginfi_account::withdraw(ctx, amount, withdraw_all)
    }

    pub fn marginfi_account_borrow(
        ctx: Context<MarginfiAccountBorrow>,
        amount: u64,
    ) -> MarginfiResult {
        marginfi_account::borrow(ctx, amount)
    }

    pub fn marginfi_account_repay(
        ctx: Context<MarginfiAccountRepay>,
        amount: u64,
        repay_all: Option<bool>,
    ) -> MarginfiResult {
        marginfi_account::repay(ctx, amount, repay_all)
    }

    /// Liquidate a lending account balance of an unhealthy marginfi account
    pub fn marginfi_account_liquidate(
        ctx: Context<MarginfiAccountLiquidate>,
        asset_amount: u64,
    ) -> MarginfiResult {
        marginfi_account::liquidate(ctx, asset_amount)
    }

    // Operational instructions
    pub fn bank_accrue_interest(ctx: Context<LendingPoolBankAccrueInterest>) -> MarginfiResult {
        marginfi_group::lending_pool_bank_accrue_interest(ctx)
    }

    pub fn bank_collect_fees(ctx: Context<LendingPoolCollectFees>) -> MarginfiResult {
        marginfi_group::lending_pool_collect_fees(ctx)
    }
}
