pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
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
use prelude::*;
use state::emode::{EmodeEntry, MAX_EMODE_ENTRIES};
use state::marginfi_group::WrappedI80F48;
use state::marginfi_group::{BankConfigCompact, BankConfigOpt};

cfg_if::cfg_if! {
    if #[cfg(feature = "mainnet-beta")] {
        declare_id!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");
    } else if #[cfg(feature = "devnet")] {
        declare_id!("neetcne3Ctrrud7vLdt2ypMm21gZHGN2mCmqWaMVcBQ");
    } else if #[cfg(feature = "staging")] {
        declare_id!("stag8sTKds2h4KzjUw3zKTsxbqvT4XKHdaR9X9E6Rct");
    } else {
        declare_id!("2jGhuVUuy3umdzByFx8sNWUAaf5vaeuDm78RDPEnhrMr");
    }
}

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
        is_arena_group: bool,
    ) -> MarginfiResult {
        marginfi_group::configure(ctx, new_admin, new_emode_admin, is_arena_group)
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

    pub fn lending_pool_add_bank_permissionless(
        ctx: Context<LendingPoolAddBankPermissionless>,
        bank_seed: u64,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_add_bank_permissionless(ctx, bank_seed)
    }

    pub fn lending_pool_configure_bank(
        ctx: Context<LendingPoolConfigureBank>,
        bank_config_opt: BankConfigOpt,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_configure_bank(ctx, bank_config_opt)
    }

    pub fn lending_pool_configure_bank_oracle(
        ctx: Context<LendingPoolConfigureBankOracle>,
        setup: u8,
        oracle: Pubkey,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_configure_bank_oracle(ctx, setup, oracle)
    }

    pub fn lending_pool_configure_bank_emode(
        ctx: Context<LendingPoolConfigureBankEmode>,
        emode_tag: u16,
        entries: [EmodeEntry; MAX_EMODE_ENTRIES],
    ) -> MarginfiResult {
        marginfi_group::lending_pool_configure_bank_emode(ctx, emode_tag, entries)
    }

    pub fn lending_pool_setup_emissions(
        ctx: Context<LendingPoolSetupEmissions>,
        flags: u64,
        rate: u64,
        total_emissions: u64,
    ) -> MarginfiResult {
        marginfi_group::lending_pool_setup_emissions(ctx, flags, rate, total_emissions)
    }

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

    /// Initialize a marginfi account for a given group
    pub fn marginfi_account_initialize(ctx: Context<MarginfiAccountInitialize>) -> MarginfiResult {
        marginfi_account::initialize_account(ctx)
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

    pub fn set_account_flag(ctx: Context<SetAccountFlag>, flag: u64) -> MarginfiResult {
        marginfi_group::set_account_flag(ctx, flag)
    }

    pub fn unset_account_flag(ctx: Context<UnsetAccountFlag>, flag: u64) -> MarginfiResult {
        marginfi_group::unset_account_flag(ctx, flag)
    }

    pub fn set_new_account_authority(
        ctx: Context<MarginfiAccountSetAccountAuthority>,
    ) -> MarginfiResult {
        marginfi_account::set_account_transfer_authority(ctx)
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
        program_fee_fixed: WrappedI80F48,
        program_fee_rate: WrappedI80F48,
    ) -> MarginfiResult {
        marginfi_group::initialize_fee_state(
            ctx,
            admin,
            fee_wallet,
            bank_init_flat_sol_fee,
            program_fee_fixed,
            program_fee_rate,
        )
    }

    /// (global fee admin only) Adjust fees, admin, or the destination wallet
    pub fn edit_global_fee_state(
        ctx: Context<EditFeeState>,
        admin: Pubkey,
        fee_wallet: Pubkey,
        bank_init_flat_sol_fee: u32,
        program_fee_fixed: WrappedI80F48,
        program_fee_rate: WrappedI80F48,
    ) -> MarginfiResult {
        marginfi_group::edit_fee_state(
            ctx,
            admin,
            fee_wallet,
            bank_init_flat_sol_fee,
            program_fee_fixed,
            program_fee_rate,
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
