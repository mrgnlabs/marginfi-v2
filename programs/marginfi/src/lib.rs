pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod macros;
pub mod prelude;
pub mod state;
pub mod utils;

use anchor_lang::prelude::*;
use instructions::*;
use prelude::*;
use state::marginfi_group::{BankConfig, BankConfigOpt};

cfg_if::cfg_if! {
    if #[cfg(feature = "mainnet-beta")] {
        declare_id!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");
    } else if #[cfg(feature = "devnet")] {
        declare_id!("neetcne3Ctrrud7vLdt2ypMm21gZHGN2mCmqWaMVcBQ");
    } else {
        declare_id!("Mfi1111111111111111111111111111111111111111");
    }
}

#[program]
pub mod marginfi {
    use super::*;

    pub fn marginfi_group_initialize(ctx: Context<MarginfiGroupInitialize>) -> MarginfiResult {
        marginfi_group::initialize(ctx)
    }

    pub fn marginfi_group_configure(
        ctx: Context<MarginfiGroupConfigure>,
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

    pub fn lending_account_deposit(
        ctx: Context<LendingAccountDeposit>,
        amount: u64,
    ) -> MarginfiResult {
        marginfi_account::lending_account_deposit(ctx, amount)
    }

    pub fn lending_account_repay(
        ctx: Context<LendingAccountRepay>,
        amount: u64,
        repay_all: Option<bool>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_repay(ctx, amount, repay_all)
    }

    pub fn lending_account_withdraw(
        ctx: Context<LendingAccountWithdraw>,
        amount: u64,
        withdraw_all: Option<bool>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_withdraw(ctx, amount, withdraw_all)
    }

    pub fn lending_account_borrow(
        ctx: Context<LendingAccountBorrow>,
        amount: u64,
    ) -> MarginfiResult {
        marginfi_account::lending_account_borrow(ctx, amount)
    }

    /// Liquidate a lending account balance of an unhealthy marginfi account
    pub fn lending_account_liquidate(
        ctx: Context<LendingAccountLiquidate>,
        asset_amount: u64,
    ) -> MarginfiResult {
        marginfi_account::lending_account_liquidate(ctx, asset_amount)
    }

    // Operational instructions
    pub fn lending_pool_accrue_bank_interest(
        ctx: Context<LendingPoolAccrueBankInterest>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_accrue_bank_interest(ctx)
    }

    pub fn lending_pool_collect_bank_fees(
        ctx: Context<LendingPoolCollectBankFees>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_collect_bank_fees(ctx)
    }
}
