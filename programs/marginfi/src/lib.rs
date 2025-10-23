pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod ix_utils;
pub mod macros;
pub mod prelude;
pub mod state;
pub mod utils;

// #[cfg(target_os = "solana")]
// use anchor_lang::solana_program::entrypoint::{HEAP_LENGTH, HEAP_START_ADDRESS};
// #[cfg(target_os = "solana")]
// use std::alloc::Layout;
// #[cfg(target_os = "solana")]
// use std::mem::size_of;
// #[cfg(target_os = "solana")]
// use std::ptr::null_mut;

use anchor_lang::prelude::*;
use instructions::*;
use marginfi_type_crate::types::{
    BankConfigCompact, BankConfigOpt, EmodeEntry, InterestRateConfigOpt, WrappedI80F48,
    MAX_EMODE_ENTRIES,
};
use prelude::*;

pub use id_crate::ID;

// #[cfg(target_os = "solana")]
// /// Custom heap allocator that exposes a move_cursor method. This allows us to manually deallocate
// /// space. NOTE: This is very unsafe, use wisely
// pub struct BumpAllocator {
//     pub start: usize,
//     pub len: usize,
// }

// #[cfg(target_os = "solana")]
// impl BumpAllocator {
//     const RESERVED_MEM: usize = 1 * size_of::<*mut u8>();

//     pub unsafe fn pos(&self) -> usize {
//         let pos_ptr = self.start as *mut usize;
//         *pos_ptr
//     }

//     /// ### This is very unsafe, use wisely
//     pub unsafe fn move_cursor(&self, pos: usize) {
//         let pos_ptr = self.start as *mut usize;
//         *pos_ptr = pos;
//     }
// }
// #[cfg(target_os = "solana")]
// unsafe impl std::alloc::GlobalAlloc for BumpAllocator {
//     #[inline]
//     unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
//         let pos_ptr = self.start as *mut usize;

//         let mut pos = *pos_ptr;
//         if pos == 0 {
//             // First time, set starting position
//             pos = self.start + self.len;
//         }
//         pos = pos.saturating_sub(layout.size());
//         pos &= !(layout.align().wrapping_sub(1));
//         if pos < self.start + BumpAllocator::RESERVED_MEM {
//             return null_mut();
//         }
//         *pos_ptr = pos;
//         pos as *mut u8
//     }
//     #[inline]
//     unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
// }

// #[cfg(target_os = "solana")]
// #[global_allocator]
// static A: BumpAllocator = BumpAllocator {
//     start: HEAP_START_ADDRESS as usize,
//     len: HEAP_LENGTH,
// };

#[program]
pub mod marginfi {
    use super::*;

    pub fn marginfi_group_initialize(
        ctx: Context<MarginfiGroupInitialize>,
        is_arena_group: bool,
    ) -> MarginfiResult {
        marginfi_group::initialize_group(ctx, is_arena_group)
    }

    pub fn marginfi_group_configure(
        ctx: Context<MarginfiGroupConfigure>,
        new_admin: Pubkey,
        new_emode_admin: Pubkey,
        new_curve_admin: Pubkey,
        new_limit_admin: Pubkey,
        new_emissions_admin: Pubkey,
        is_arena_group: bool,
    ) -> MarginfiResult {
        marginfi_group::configure(
            ctx,
            new_admin,
            new_emode_admin,
            new_curve_admin,
            new_limit_admin,
            new_emissions_admin,
            is_arena_group,
        )
    }

    pub fn lending_pool_add_bank(
        ctx: Context<LendingPoolAddBank>,
        bank_config: BankConfigCompact,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_add_bank(ctx, bank_config)
    }

    /// A copy of lending_pool_add_bank with an additional bank seed.
    /// This seed is used to create a PDA for the bank's signature.
    /// lending_pool_add_bank is preserved for backwards compatibility.
    pub fn lending_pool_add_bank_with_seed(
        ctx: Context<LendingPoolAddBankWithSeed>,
        bank_config: BankConfigCompact,
        bank_seed: u64,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_add_bank_with_seed(ctx, bank_config, bank_seed)
    }

    /// Staging or localnet only, panics on mainnet
    pub fn lending_pool_clone_bank(
        ctx: Context<LendingPoolCloneBank>,
        bank_seed: u64,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_clone_bank(ctx, bank_seed)
    }

    pub fn lending_pool_add_bank_permissionless(
        ctx: Context<LendingPoolAddBankPermissionless>,
        bank_seed: u64,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_add_bank_permissionless(ctx, bank_seed)
    }

    /// (admin only)
    pub fn lending_pool_configure_bank(
        ctx: Context<LendingPoolConfigureBank>,
        bank_config_opt: BankConfigOpt,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_configure_bank(ctx, bank_config_opt)
    }

    /// (delegate_curve_admin only)
    pub fn lending_pool_configure_bank_interest_only(
        ctx: Context<LendingPoolConfigureBankInterestOnly>,
        interest_rate_config: InterestRateConfigOpt,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_configure_bank_interest_only(ctx, interest_rate_config)
    }

    /// (delegate_limits_admin only)
    pub fn lending_pool_configure_bank_limits_only(
        ctx: Context<LendingPoolConfigureBankLimitsOnly>,
        deposit_limit: Option<u64>,
        borrow_limit: Option<u64>,
        total_asset_value_init_limit: Option<u64>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_configure_bank_limits_only(
            ctx,
            deposit_limit,
            borrow_limit,
            total_asset_value_init_limit,
        )
    }

    /// (admin only)
    pub fn lending_pool_configure_bank_oracle(
        ctx: Context<LendingPoolConfigureBankOracle>,
        setup: u8,
        oracle: Pubkey,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_configure_bank_oracle(ctx, setup, oracle)
    }

    /// (emode_admin only)
    pub fn lending_pool_configure_bank_emode(
        ctx: Context<LendingPoolConfigureBankEmode>,
        emode_tag: u16,
        entries: [EmodeEntry; MAX_EMODE_ENTRIES],
    ) -> MarginfiResult {
        marginfi_group::lending_pool_configure_bank_emode(ctx, emode_tag, entries)
    }

    /// (delegate_emissions_admin only)
    pub fn lending_pool_setup_emissions(
        ctx: Context<LendingPoolSetupEmissions>,
        flags: u64,
        rate: u64,
        total_emissions: u64,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_setup_emissions(ctx, flags, rate, total_emissions)
    }

    /// (delegate_emissions_admin only)
    pub fn lending_pool_update_emissions_parameters(
        ctx: Context<LendingPoolUpdateEmissionsParameters>,
        emissions_flags: Option<u64>,
        emissions_rate: Option<u64>,
        additional_emissions: Option<u64>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_update_emissions_parameters(
            ctx,
            emissions_flags,
            emissions_rate,
            additional_emissions,
        )
    }

    /// Handle bad debt of a bankrupt marginfi account for a given bank.
    pub fn lending_pool_handle_bankruptcy<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingPoolHandleBankruptcy<'info>>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_handle_bankruptcy(ctx)
    }

    // User instructions

    /// Initialize a marginfi account for a given group. The account is a fresh keypair, and must
    /// sign. If you are a CPI caller, consider using `marginfi_account_initialize_pda` instead, or
    /// create the account manually and use `transfer_to_new_account` to gift it to the owner you
    /// wish.
    pub fn marginfi_account_initialize(ctx: Context<MarginfiAccountInitialize>) -> MarginfiResult {
        marginfi_account::initialize_account(ctx)
    }

    pub fn marginfi_account_init_liq_record(ctx: Context<InitLiquidationRecord>) -> MarginfiResult {
        marginfi_account::initialize_liquidation_record(ctx)
    }

    /// The same as `marginfi_account_initialize`, except the created marginfi account uses a PDA
    /// (Program Derived Address)
    ///
    /// seeds:
    /// - marginfi_group
    /// - authority: The account authority (owner)  
    /// - account_index: A u16 value to allow multiple accounts per authority
    /// - third_party_id: Optional u16 for third-party tagging. Seeds < PDA_FREE_THRESHOLD can be
    ///   used freely. For a dedicated seed used by just your program (via CPI), contact us.
    pub fn marginfi_account_initialize_pda(
        ctx: Context<MarginfiAccountInitializePda>,
        account_index: u16,
        third_party_id: Option<u16>,
    ) -> MarginfiResult {
        marginfi_account::initialize_account_pda(ctx, account_index, third_party_id)
    }

    pub fn lending_account_deposit<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingAccountDeposit<'info>>,
        amount: u64,
        deposit_up_to_limit: Option<bool>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_deposit(ctx, amount, deposit_up_to_limit)
    }

    pub fn lending_account_repay<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingAccountRepay<'info>>,
        amount: u64,
        repay_all: Option<bool>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_repay(ctx, amount, repay_all)
    }

    pub fn lending_account_withdraw<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingAccountWithdraw<'info>>,
        amount: u64,
        withdraw_all: Option<bool>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_withdraw(ctx, amount, withdraw_all)
    }

    pub fn lending_account_borrow<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingAccountBorrow<'info>>,
        amount: u64,
    ) -> MarginfiResult {
        marginfi_account::lending_account_borrow(ctx, amount)
    }

    pub fn lending_account_close_balance(
        ctx: Context<LendingAccountCloseBalance>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_close_balance(ctx)
    }

    pub fn lending_account_withdraw_emissions<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingAccountWithdrawEmissions<'info>>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_withdraw_emissions(ctx)
    }

    pub fn lending_account_settle_emissions(
        ctx: Context<LendingAccountSettleEmissions>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_settle_emissions(ctx)
    }

    /// Liquidate a lending account balance of an unhealthy marginfi account
    pub fn lending_account_liquidate<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingAccountLiquidate<'info>>,
        asset_amount: u64,
    ) -> MarginfiResult {
        marginfi_account::lending_account_liquidate(ctx, asset_amount)
    }

    pub fn lending_account_start_flashloan(
        ctx: Context<LendingAccountStartFlashloan>,
        end_index: u64,
    ) -> MarginfiResult {
        marginfi_account::lending_account_start_flashloan(ctx, end_index)
    }

    pub fn lending_account_end_flashloan<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingAccountEndFlashloan<'info>>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_end_flashloan(ctx)
    }

    pub fn marginfi_account_update_emissions_destination_account<'info>(
        ctx: Context<'_, '_, 'info, 'info, MarginfiAccountUpdateEmissionsDestinationAccount<'info>>,
    ) -> MarginfiResult {
        marginfi_account::marginfi_account_update_emissions_destination_account(ctx)
    }

    // Operational instructions
    pub fn lending_pool_accrue_bank_interest(
        ctx: Context<LendingPoolAccrueBankInterest>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_accrue_bank_interest(ctx)
    }

    pub fn lending_pool_collect_bank_fees<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingPoolCollectBankFees<'info>>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_collect_bank_fees(ctx)
    }

    pub fn lending_pool_withdraw_fees<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingPoolWithdrawFees<'info>>,
        amount: u64,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_withdraw_fees(ctx, amount)
    }

    pub fn lending_pool_withdraw_fees_permissionless<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingPoolWithdrawFeesPermissionless<'info>>,
        amount: u64,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_withdraw_fees_permissionless(ctx, amount)
    }

    pub fn lending_pool_update_fees_destination_account<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingPoolUpdateFeesDestinationAccount<'info>>,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_update_fees_destination_account(ctx)
    }

    pub fn lending_pool_withdraw_insurance<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingPoolWithdrawInsurance<'info>>,
        amount: u64,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_withdraw_insurance(ctx, amount)
    }

    pub fn lending_pool_close_bank(ctx: Context<LendingPoolCloseBank>) -> MarginfiResult {
        marginfi_group::lending_pool_close_bank(ctx)
    }

    pub fn transfer_to_new_account(ctx: Context<TransferToNewAccount>) -> MarginfiResult {
        marginfi_account::transfer_to_new_account(ctx)
    }

    /// Same as `transfer_to_new_account` except the resulting account is a PDA
    ///
    /// seeds:
    /// - marginfi_group
    /// - authority: The account authority (owner)  
    /// - account_index: A u32 value to allow multiple accounts per authority
    /// - third_party_id: Optional u32 for third-party tagging. Seeds < PDA_FREE_THRESHOLD can be
    ///   used freely. For a dedicated seed used by just your program (via CPI), contact us.
    pub fn transfer_to_new_account_pda(
        ctx: Context<TransferToNewAccountPda>,
        account_index: u16,
        third_party_id: Option<u16>,
    ) -> MarginfiResult {
        marginfi_account::transfer_to_new_account_pda(ctx, account_index, third_party_id)
    }

    pub fn marginfi_account_close(ctx: Context<MarginfiAccountClose>) -> MarginfiResult {
        marginfi_account::close_account(ctx)
    }

    pub fn lending_account_withdraw_emissions_permissionless<'info>(
        ctx: Context<'_, '_, 'info, 'info, LendingAccountWithdrawEmissionsPermissionless<'info>>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_withdraw_emissions_permissionless(ctx)
    }

    /// (Permissionless) Refresh the internal risk engine health cache. Useful for liquidators and
    /// other consumers that want to see the internal risk state of a user account. This cache is
    /// read-only and serves no purpose except being populated by this ix.
    /// * remaining accounts expected in the same order as borrow, etc. I.e., for each balance the
    ///   user has, pass bank and oracle: <bank1, oracle1, bank2, oracle2>
    pub fn lending_account_pulse_health<'info>(
        ctx: Context<'_, '_, 'info, 'info, PulseHealth<'info>>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_pulse_health(ctx)
    }

    /// (Permissionless) Sorts the lending account balances in descending order and removes the "gaps"
    /// (i.e. inactive balances in between the active ones), if any.
    /// This is necessary to ensure any legacy marginfi accounts are compliant with the
    /// "gapless and sorted" requirements we now have.
    pub fn lending_account_sort_balances<'info>(
        ctx: Context<'_, '_, 'info, 'info, SortBalances<'info>>,
    ) -> MarginfiResult {
        marginfi_account::lending_account_sort_balances(ctx)
    }

    /// (Runs once per program) Configures the fee state account, where the global admin sets fees
    /// that are assessed to the protocol
    pub fn init_global_fee_state(
        ctx: Context<InitFeeState>,
        admin: Pubkey,
        fee_wallet: Pubkey,
        bank_init_flat_sol_fee: u32,
        liquidation_flat_sol_fee: u32,
        program_fee_fixed: WrappedI80F48,
        program_fee_rate: WrappedI80F48,
        liquidation_max_fee: WrappedI80F48,
    ) -> MarginfiResult {
        marginfi_group::initialize_fee_state(
            ctx,
            admin,
            fee_wallet,
            bank_init_flat_sol_fee,
            liquidation_flat_sol_fee,
            program_fee_fixed,
            program_fee_rate,
            liquidation_max_fee,
        )
    }

    /// (global fee admin only) Adjust fees, admin, or the destination wallet
    pub fn edit_global_fee_state(
        ctx: Context<EditFeeState>,
        admin: Pubkey,
        fee_wallet: Pubkey,
        bank_init_flat_sol_fee: u32,
        liquidation_flat_sol_fee: u32,
        program_fee_fixed: WrappedI80F48,
        program_fee_rate: WrappedI80F48,
        liquidation_max_fee: WrappedI80F48,
    ) -> MarginfiResult {
        marginfi_group::edit_fee_state(
            ctx,
            admin,
            fee_wallet,
            bank_init_flat_sol_fee,
            liquidation_flat_sol_fee,
            program_fee_fixed,
            program_fee_rate,
            liquidation_max_fee,
        )
    }

    /// (Permissionless) Force any group to adopt the current FeeState settings
    pub fn propagate_fee_state(ctx: Context<PropagateFee>) -> MarginfiResult {
        marginfi_group::propagate_fee(ctx)
    }

    /// (global fee admin only) Enable or disable program fees for any group. Does not require the
    /// group admin to sign: the global fee state admin can turn program fees on or off for any
    /// group
    pub fn config_group_fee(
        ctx: Context<ConfigGroupFee>,
        enable_program_fee: bool,
    ) -> MarginfiResult {
        marginfi_group::config_group_fee(ctx, enable_program_fee)
    }

    /// (group admin only) Init the Staked Settings account, which is used to create staked
    /// collateral banks, and must run before any staked collateral bank can be created with
    /// `add_pool_permissionless`. Running this ix effectively opts the group into the staked
    /// collateral feature.
    pub fn init_staked_settings(
        ctx: Context<InitStakedSettings>,
        settings: StakedSettingsConfig,
    ) -> MarginfiResult {
        marginfi_group::initialize_staked_settings(ctx, settings)
    }

    pub fn edit_staked_settings(
        ctx: Context<EditStakedSettings>,
        settings: StakedSettingsEditConfig,
    ) -> MarginfiResult {
        marginfi_group::edit_staked_settings(ctx, settings)
    }

    pub fn propagate_staked_settings(ctx: Context<PropagateStakedSettings>) -> MarginfiResult {
        marginfi_group::propagate_staked_settings(ctx)
    }

    pub fn start_liquidation<'info>(
        ctx: Context<'_, '_, 'info, 'info, StartLiquidation<'info>>,
    ) -> MarginfiResult {
        marginfi_account::start_liquidation(ctx)
    }

    pub fn end_liquidation<'info>(
        ctx: Context<'_, '_, 'info, 'info, EndLiquidation<'info>>,
    ) -> MarginfiResult {
        marginfi_account::end_liquidation(ctx)
    }

    pub fn panic_pause(ctx: Context<PanicPause>) -> MarginfiResult {
        marginfi_group::panic_pause(ctx)
    }

    pub fn panic_unpause(ctx: Context<PanicUnpause>) -> MarginfiResult {
        marginfi_group::panic_unpause(ctx)
    }

    /// (permissionless) Unpause the protocol when pause time has expired
    pub fn panic_unpause_permissionless(
        ctx: Context<PanicUnpausePermissionless>,
    ) -> MarginfiResult {
        marginfi_group::panic_unpause_permissionless(ctx)
    }

    // TODO deprecate in 1.7
    /// (Permissionless) Convert a bank from the legacy curve setup to the new setup, with no effect
    /// on how interest accrues.
    pub fn migrate_curve(ctx: Context<MigrateCurve>) -> MarginfiResult {
        marginfi_group::migrate_curve(ctx)
    }

    /****** Kamino integration instructions *****/

    /// (permissionless) Initialize a Kamino obligation for a marginfi bank
    /// * amount - In token, in native decimals. Must be >10 (i.e. 10 lamports, not 10 tokens). Lost
    ///   forever. Generally, try to make this the equivalent of around $1, in case Kamino ever
    ///   rounds small balances down to zero.
    pub fn kamino_init_obligation(
        ctx: Context<KaminoInitObligation>,
        amount: u64,
    ) -> MarginfiResult {
        kamino::kamino_init_obligation(ctx, amount)
    }

    /// (user) Deposit into a Kamino pool through a marginfi account
    /// * amount - in the liquidity token (e.g. if there is a Kamino USDC bank, pass the amount of
    ///   USDC desired), in native decimals.
    pub fn kamino_deposit(ctx: Context<KaminoDeposit>, amount: u64) -> MarginfiResult {
        kamino::kamino_deposit(ctx, amount)
    }

    /// (user) Withdraw from a Kamino pool through a marginfi account
    /// * amount - in the collateral token (NOT liquidity token), in native decimals. Must convert
    ///     from collateral to liquidity token amounts using the current exchange rate.
    /// * withdraw_all - if true, withdraw the entire mrgn balance (Note: due to rounding down, a
    ///   deposit and withdraw back to back may result in several lamports less)
    pub fn kamino_withdraw<'info>(
        ctx: Context<'_, '_, 'info, 'info, KaminoWithdraw<'info>>,
        amount: u64,
        withdraw_all: Option<bool>,
    ) -> MarginfiResult {
        kamino::kamino_withdraw(ctx, amount, withdraw_all)
    }

    /// (group admin only) Add a Kamino bank to the group. Pass the oracle and reserve in remaining
    /// accounts 0 and 1 respectively.
    pub fn lending_pool_add_bank_kamino(
        ctx: Context<LendingPoolAddBankKamino>,
        bank_config: state::kamino::KaminoConfigCompact,
        bank_seed: u64,
    ) -> MarginfiResult {
        kamino::lending_pool_add_bank_kamino(ctx, bank_config, bank_seed)
    }

    /// (fee admin only) Harvest the specified reward index from the Kamino Farm attached to this bank.
    ///
    /// * `reward_index` — index of the reward token in the Kamino Farm's reward list
    pub fn kamino_harvest_reward(
        ctx: Context<KaminoHarvestReward>,
        reward_index: u64,
    ) -> MarginfiResult {
        kamino::kamino_harvest_reward(ctx, reward_index)
    }
}

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;
#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "marginfi v2",
    project_url: "https://app.marginfi.com/",
    contacts: "email:security@mrgn.group",
    policy: "https://github.com/mrgnlabs/marginfi-v2/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/mrgnlabs/marginfi-v2"
}
