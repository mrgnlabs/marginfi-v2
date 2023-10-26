use anchor_lang::prelude::*;

use super::marginfi_group::WrappedI80F48;

// CDP Actions
// ==========
// - Desposit collateral
// - Mint stablecoin
// - Repay stablecoin
// - Withdraw collateral
// - Liquidate
//
// Admin Actions
// =============
// - Create CDP bank
// - Add collateral
// - Set interest rate
//
//
// Liquidation mechanism
// =====================
// Once a CDP is below maintenance margin, it can be liquidated by anyone.
// The liquidator pays off the CDP's debt and receives discounted collateral,
// until a CDP is above maintenance margin.
//
// Discount difference is split between the liquidator and the bank insurance fund.

/// Multi collateral CDP bank
#[zero_copy]
#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize)]
struct CdpBank {
    pub group: Pubkey,
    pub mint_authority: Pubkey,
    pub mint: Pubkey,

    pub total_liability_shares: WrappedI80F48,
    /// Accrues interest
    pub liability_share_value: WrappedI80F48,
    /// Interest rate APR
    pub liability_interest_rate: WrappedI80F48,

    // Config
    /// Max amount of stablecoin that can be minted
    pub liability_limit: u64,
}

/// Supported collateral for a bank
#[zero_copy]
#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize)]
struct CdpCollateral {
    pub lending_bank: Pubkey,
    pub cdp_bank: Pubkey,
    /// - 0: Disabled
    /// - 1: Enabled
    /// - 2: ReduceOnly
    pub status: u8,
    // Optional parameters for additional margin requirement configurations
    // pub asset_weight_init_haircut: WrappedI80F48,
    // pub asset_weight_maint_haircut: WrappedI80F48,
}

/// Cdp account
#[zero_copy]
#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize)]
struct Cdp {
    pub group: Pubkey,
    pub authority: Pubkey,
    pub cdp_collateral_config: Pubkey,

    pub collateral_shares: WrappedI80F48,
    pub liability_shares: WrappedI80F48,
    pub flags: u64,
}
