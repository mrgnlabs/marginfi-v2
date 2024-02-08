use anchor_lang::prelude::*;
use bytemuck::Zeroable;
use fixed::types::{I80F48, I89F39};

use crate::{
    check, constants::MAX_PRICE_AGE_SEC, math_error, prelude::MarginfiResult,
    state::price::PriceAdapter,
};

use super::{
    marginfi_group::{Bank, WrappedI80F48},
    price::{OraclePriceFeedAdapter, OraclePriceType, PriceBias},
};

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

impl CdpBank {
    pub fn get_liability_shares(&self, amount: I80F48) -> Option<I80F48> {
        amount.checked_div(self.liability_share_value.into())
    }

    pub fn get_liability_amount(&self, shares: I80F48) -> Option<I80F48> {
        shares.checked_mul(self.liability_share_value.into())
    }

    pub fn update_liability_share_amount(&mut self, shares: I80F48) -> MarginfiResult {
        self.total_liability_shares = WrappedI80F48::from(
            shares
                .checked_add(self.total_liability_shares.into())
                .ok_or_else(math_error!())?,
        );

        Ok(())
    }
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

unsafe impl Pod for CdpCollateralBank {}
unsafe impl Zeroable for CdpCollateralBank {}

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

pub struct CdpRiskEngine<'a> {
    deposit: &'a Cdp,
    bank: &'a CdpBank,
    collateral_bank: &'a CdpCollateralBank,
    lending_bank: &'a Bank,
    oracle_adapter: OraclePriceFeedAdapter,
}

impl<'a> CdpRiskEngine<'a> {
    pub fn new(
        deposit: &'a Cdp,
        bank: &'a CdpBank,
        collateral_bank: &'a CdpCollateralBank,
        lending_bank: &'a Bank,
        oracle_ais: &[AccountInfo],
        current_timestamp: i64,
    ) -> Result<Self> {
        let oracle_adapter = OraclePriceFeedAdapter::try_from_bank_config(
            &lending_bank.config,
            oracle_ais,
            current_timestamp,
            MAX_PRICE_AGE_SEC,
        )?;

        Ok(Self {
            deposit,
            bank,
            collateral_bank,
            lending_bank,
            oracle_adapter,
        })
    }

    pub fn check_init_health(&self) -> Result<()> {
        let liab_shares = I80F48::from(self.deposit.liability_shares);
        let liab_value = self
            .bank
            .get_liability_amount(liab_shares)
            .ok_or_else(math_error!())?;

        let collateral_shares = I80F48::from(self.deposit.collateral_shares);
        let collateral_price = self
            .oracle_adapter
            .get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::Low))?;

        let collateral_init_weight: I80F48 = self.lending_bank.config.asset_weight_init.into();

        let weighted_collateral_value = collateral_shares
            .checked_mul(collateral_price)
            .ok_or_else(math_error!())?
            .checked_mul(collateral_init_weight)
            .ok_or_else(math_error!())?;

        check!(
            weighted_collateral_value >= liab_value,
            crate::prelude::MarginfiError::BadAccountHealth
        );

        Ok(())
    }
}
