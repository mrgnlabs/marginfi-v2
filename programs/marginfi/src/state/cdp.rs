use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::{math_error, prelude::MarginfiResult};

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
#[account(zero_copy)]
#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct CdpBank {
    pub group: Pubkey,
    pub mint: Pubkey,

    pub mint_authority: Pubkey,
    pub mint_authority_bump: u8,

    pub total_liability_shares: WrappedI80F48,
    /// Accrues interest
    pub liability_share_value: WrappedI80F48,
    /// Interest rate APR
    pub liability_interest_rate: WrappedI80F48,

    // Config
    /// Max amount of stablecoin that can be minted
    pub liability_limit: u64,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub enum CdpCollateralBankStatus {
    Disabled,
    Enabled,
    /// Desposits and minting are disabled
    /// Repayments, withdrawing, and liquidations are enabled
    ReduceOnly,
}

/// Supported collateral for a bank
#[account(zero_copy)]
#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct CdpCollateralBank {
    pub lending_bank: Pubkey,
    pub cdp_bank: Pubkey,
    /// - 0: Disabled
    /// - 1: Enabled
    /// - 2: ReduceOnly
    pub status: CdpCollateralBankStatus,
    // Optional parameters for additional margin requirement configurations
    // pub asset_weight_init_haircut: WrappedI80F48,
    // pub asset_weight_maint_haircut: WrappedI80F48,
}

/// Cdp account
#[account(zero_copy)]
#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct Cdp {
    pub authority: Pubkey,
    pub cdp_collateral_bank: Pubkey,

    pub collateral_shares: WrappedI80F48,
    pub liability_shares: WrappedI80F48,
    pub flags: u64,
}

impl Cdp {
    pub fn change_collateral_shares(&mut self, shares: I80F48) -> MarginfiResult {
        let current_coll_shares = I80F48::from(self.collateral_shares);

        self.collateral_shares = current_coll_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?
            .into();

        Ok(())
    }

    pub fn change_liability_shares(&mut self, shares: I80F48) -> MarginfiResult {
        let current_liab_shares = I80F48::from(self.liability_shares);

        self.liability_shares = current_liab_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?
            .into();

        Ok(())
    }
}
