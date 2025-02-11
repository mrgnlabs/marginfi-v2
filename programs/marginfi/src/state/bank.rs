use std::ops::Not;

#[cfg(not(feature = "client"))]
use crate::events::{GroupEventHeader, LendingPoolBankAccrueInterestEvent};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::*;
use fixed::types::I80F48;
use type_layout::TypeLayout;

use crate::{
    assert_struct_align, assert_struct_size, check,
    constants::{
        EMISSION_FLAGS, FREEZE_SETTINGS, GROUP_FLAGS, PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG,
    },
    debug, math_error, set_if_some,
    state::{
        bank_interest::{calc_interest_rate_accrual_state_changes, InterestRateStateChanges},
        marginfi_account::calc_value,
    },
    MarginfiError, MarginfiGroup, MarginfiResult,
};

use super::{
    bank_configuration::{BankConfig, BankConfigOpt, BankOperationalState},
    marginfi_group::WrappedI80F48,
};

assert_struct_size!(Bank, 1856);
assert_struct_align!(Bank, 8);
#[account(zero_copy)]
#[repr(C)]
#[derive(Default, Debug, PartialEq, Eq, TypeLayout)]
pub struct Bank {
    pub mint: Pubkey,
    pub mint_decimals: u8,

    pub group: Pubkey,

    // Note: The padding is here, not after mint_decimals. Pubkey has alignment 1, so those 32
    // bytes can cross the alignment 8 threshold, but WrappedI80F48 has alignment 8 and cannot
    pub _pad0: [u8; 7], // 1x u8 + 7 = 8

    pub asset_share_value: WrappedI80F48,
    pub liability_share_value: WrappedI80F48,

    pub liquidity_vault: Pubkey,
    pub liquidity_vault_bump: u8,
    pub liquidity_vault_authority_bump: u8,

    pub insurance_vault: Pubkey,
    pub insurance_vault_bump: u8,
    pub insurance_vault_authority_bump: u8,

    pub _pad1: [u8; 4], // 4x u8 + 4 = 8

    /// Fees collected and pending withdraw for the `insurance_vault`
    pub collected_insurance_fees_outstanding: WrappedI80F48,

    pub fee_vault: Pubkey,
    pub fee_vault_bump: u8,
    pub fee_vault_authority_bump: u8,

    pub _pad2: [u8; 6], // 2x u8 + 6 = 8

    /// Fees collected and pending withdraw for the `fee_vault`
    pub collected_group_fees_outstanding: WrappedI80F48,

    pub total_liability_shares: WrappedI80F48,
    pub total_asset_shares: WrappedI80F48,

    pub last_update: i64,

    pub config: BankConfig,

    /// Bank Config Flags
    ///
    /// - EMISSIONS_FLAG_BORROW_ACTIVE: 1
    /// - EMISSIONS_FLAG_LENDING_ACTIVE: 2
    /// - PERMISSIONLESS_BAD_DEBT_SETTLEMENT: 4
    ///
    pub flags: u64,
    /// Emissions APR.
    /// Number of emitted tokens (emissions_mint) per 1e(bank.mint_decimal) tokens (bank mint) (native amount) per 1 YEAR.
    pub emissions_rate: u64,
    pub emissions_remaining: WrappedI80F48,
    pub emissions_mint: Pubkey,

    /// Fees collected and pending withdraw for the `FeeState.global_fee_wallet`'s cannonical ATA for `mint`
    pub collected_program_fees_outstanding: WrappedI80F48,

    pub _padding_0: [[u64; 2]; 27],
    pub _padding_1: [[u64; 2]; 32], // 16 * 2 * 32 = 1024B
}

impl Bank {
    pub const LEN: usize = std::mem::size_of::<Bank>();

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        marginfi_group_pk: Pubkey,
        config: BankConfig,
        mint: Pubkey,
        mint_decimals: u8,
        liquidity_vault: Pubkey,
        insurance_vault: Pubkey,
        fee_vault: Pubkey,
        current_timestamp: i64,
        liquidity_vault_bump: u8,
        liquidity_vault_authority_bump: u8,
        insurance_vault_bump: u8,
        insurance_vault_authority_bump: u8,
        fee_vault_bump: u8,
        fee_vault_authority_bump: u8,
    ) -> Bank {
        Bank {
            mint,
            mint_decimals,
            group: marginfi_group_pk,
            asset_share_value: I80F48::ONE.into(),
            liability_share_value: I80F48::ONE.into(),
            liquidity_vault,
            liquidity_vault_bump,
            liquidity_vault_authority_bump,
            insurance_vault,
            insurance_vault_bump,
            insurance_vault_authority_bump,
            collected_insurance_fees_outstanding: I80F48::ZERO.into(),
            fee_vault,
            fee_vault_bump,
            fee_vault_authority_bump,
            collected_group_fees_outstanding: I80F48::ZERO.into(),
            total_liability_shares: I80F48::ZERO.into(),
            total_asset_shares: I80F48::ZERO.into(),
            last_update: current_timestamp,
            config,
            flags: 0,
            emissions_rate: 0,
            emissions_remaining: I80F48::ZERO.into(),
            emissions_mint: Pubkey::default(),
            collected_program_fees_outstanding: I80F48::ZERO.into(),
            ..Default::default()
        }
    }

    pub fn get_liability_amount(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.liability_share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub fn get_asset_amount(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.asset_share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub fn get_liability_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.liability_share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub fn get_asset_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.asset_share_value.into())
            .ok_or_else(math_error!())?)
    }

    pub fn change_asset_shares(
        &mut self,
        shares: I80F48,
        bypass_deposit_limit: bool,
    ) -> MarginfiResult {
        let total_asset_shares: I80F48 = self.total_asset_shares.into();
        self.total_asset_shares = total_asset_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?
            .into();

        if shares.is_positive() && self.config.is_deposit_limit_active() && !bypass_deposit_limit {
            let total_deposits_amount = self.get_asset_amount(self.total_asset_shares.into())?;
            let deposit_limit = I80F48::from_num(self.config.deposit_limit);

            check!(
                total_deposits_amount < deposit_limit,
                crate::prelude::MarginfiError::BankAssetCapacityExceeded
            )
        }

        Ok(())
    }

    pub fn maybe_get_asset_weight_init_discount(
        &self,
        price: I80F48,
    ) -> MarginfiResult<Option<I80F48>> {
        if self.config.usd_init_limit_active() {
            let bank_total_assets_value = calc_value(
                self.get_asset_amount(self.total_asset_shares.into())?,
                price,
                self.mint_decimals,
                None,
            )?;

            let total_asset_value_init_limit =
                I80F48::from_num(self.config.total_asset_value_init_limit);

            #[cfg(target_os = "solana")]
            debug!(
                "Init limit active, limit: {}, total_assets: {}",
                total_asset_value_init_limit, bank_total_assets_value
            );

            if bank_total_assets_value > total_asset_value_init_limit {
                let discount = total_asset_value_init_limit
                    .checked_div(bank_total_assets_value)
                    .ok_or_else(math_error!())?;

                #[cfg(target_os = "solana")]
                debug!(
                    "Discounting assets by {:.2} because of total deposits {} over {} usd cap",
                    discount, bank_total_assets_value, total_asset_value_init_limit
                );

                Ok(Some(discount))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn change_liability_shares(
        &mut self,
        shares: I80F48,
        bypass_borrow_limit: bool,
    ) -> MarginfiResult {
        let total_liability_shares: I80F48 = self.total_liability_shares.into();
        self.total_liability_shares = total_liability_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?
            .into();

        if bypass_borrow_limit.not() && shares.is_positive() && self.config.is_borrow_limit_active()
        {
            let total_liability_amount =
                self.get_liability_amount(self.total_liability_shares.into())?;
            let borrow_limit = I80F48::from_num(self.config.borrow_limit);

            check!(
                total_liability_amount < borrow_limit,
                crate::prelude::MarginfiError::BankLiabilityCapacityExceeded
            )
        }

        Ok(())
    }

    pub fn check_utilization_ratio(&self) -> MarginfiResult {
        let total_assets = self.get_asset_amount(self.total_asset_shares.into())?;
        let total_liabilities = self.get_liability_amount(self.total_liability_shares.into())?;

        check!(
            total_assets >= total_liabilities,
            crate::prelude::MarginfiError::IllegalUtilizationRatio
        );

        Ok(())
    }

    pub fn configure(&mut self, config: &BankConfigOpt) -> MarginfiResult {
        set_if_some!(self.config.asset_weight_init, config.asset_weight_init);
        set_if_some!(self.config.asset_weight_maint, config.asset_weight_maint);
        set_if_some!(
            self.config.liability_weight_init,
            config.liability_weight_init
        );
        set_if_some!(
            self.config.liability_weight_maint,
            config.liability_weight_maint
        );
        set_if_some!(self.config.deposit_limit, config.deposit_limit);

        set_if_some!(self.config.borrow_limit, config.borrow_limit);

        set_if_some!(self.config.operational_state, config.operational_state);

        if let Some(ir_config) = &config.interest_rate_config {
            self.config.interest_rate_config.update(ir_config);
        }

        set_if_some!(self.config.risk_tier, config.risk_tier);

        set_if_some!(self.config.asset_tag, config.asset_tag);

        set_if_some!(
            self.config.total_asset_value_init_limit,
            config.total_asset_value_init_limit
        );

        set_if_some!(self.config.oracle_max_age, config.oracle_max_age);

        if let Some(flag) = config.permissionless_bad_debt_settlement {
            self.update_flag(flag, PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG);
        }

        if let Some(flag) = config.freeze_settings {
            self.update_flag(flag, FREEZE_SETTINGS);
        }

        self.config.validate()?;

        Ok(())
    }

    /// Configures just the borrow and deposit limits, ignoring all other values
    pub fn configure_unfrozen_fields_only(&mut self, config: &BankConfigOpt) -> MarginfiResult {
        set_if_some!(self.config.deposit_limit, config.deposit_limit);
        set_if_some!(self.config.borrow_limit, config.borrow_limit);
        // weights didn't change so no validation is needed
        Ok(())
    }

    /// Calculate the interest rate accrual state changes for a given time period
    ///
    /// Collected protocol and insurance fees are stored in state.
    /// A separate instruction is required to withdraw these fees.
    pub fn accrue_interest(
        &mut self,
        current_timestamp: i64,
        group: &MarginfiGroup,
        #[cfg(not(feature = "client"))] bank: Pubkey,
    ) -> MarginfiResult<()> {
        #[cfg(all(not(feature = "client"), feature = "debug"))]
        solana_program::log::sol_log_compute_units();

        let time_delta: u64 = (current_timestamp - self.last_update).try_into().unwrap();
        if time_delta == 0 {
            return Ok(());
        }

        let total_assets = self.get_asset_amount(self.total_asset_shares.into())?;
        let total_liabilities = self.get_liability_amount(self.total_liability_shares.into())?;

        self.last_update = current_timestamp;

        if (total_assets == I80F48::ZERO) || (total_liabilities == I80F48::ZERO) {
            #[cfg(not(feature = "client"))]
            emit!(LendingPoolBankAccrueInterestEvent {
                header: GroupEventHeader {
                    marginfi_group: self.group,
                    signer: None
                },
                bank,
                mint: self.mint,
                delta: time_delta,
                fees_collected: 0.,
                insurance_collected: 0.,
            });

            return Ok(());
        }
        let ir_calc = self
            .config
            .interest_rate_config
            .create_interest_rate_calculator(group);

        let InterestRateStateChanges {
            new_asset_share_value: asset_share_value,
            new_liability_share_value: liability_share_value,
            insurance_fees_collected,
            group_fees_collected,
            protocol_fees_collected,
        } = calc_interest_rate_accrual_state_changes(
            time_delta,
            total_assets,
            total_liabilities,
            &ir_calc,
            self.asset_share_value.into(),
            self.liability_share_value.into(),
        )
        .ok_or_else(math_error!())?;

        debug!("deposit share value: {}\nliability share value: {}\nfees collected: {}\ninsurance collected: {}",
            asset_share_value, liability_share_value, group_fees_collected, insurance_fees_collected);

        self.asset_share_value = asset_share_value.into();
        self.liability_share_value = liability_share_value.into();

        if group_fees_collected > I80F48::ZERO {
            self.collected_group_fees_outstanding = {
                group_fees_collected
                    .checked_add(self.collected_group_fees_outstanding.into())
                    .ok_or_else(math_error!())?
                    .into()
            };
        }

        if insurance_fees_collected > I80F48::ZERO {
            self.collected_insurance_fees_outstanding = {
                insurance_fees_collected
                    .checked_add(self.collected_insurance_fees_outstanding.into())
                    .ok_or_else(math_error!())?
                    .into()
            };
        }
        if protocol_fees_collected > I80F48::ZERO {
            self.collected_program_fees_outstanding = {
                protocol_fees_collected
                    .checked_add(self.collected_program_fees_outstanding.into())
                    .ok_or_else(math_error!())?
                    .into()
            };
        }

        #[cfg(not(feature = "client"))]
        {
            #[cfg(feature = "debug")]
            solana_program::log::sol_log_compute_units();

            emit!(LendingPoolBankAccrueInterestEvent {
                header: GroupEventHeader {
                    marginfi_group: self.group,
                    signer: None
                },
                bank,
                mint: self.mint,
                delta: time_delta,
                fees_collected: group_fees_collected.to_num::<f64>(),
                insurance_collected: insurance_fees_collected.to_num::<f64>(),
            });
        }

        Ok(())
    }

    pub fn deposit_spl_transfer<'info>(
        &self,
        amount: u64,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        maybe_mint: Option<&InterfaceAccount<'info, Mint>>,
        program: AccountInfo<'info>,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> MarginfiResult {
        check!(
            to.key.eq(&self.liquidity_vault),
            MarginfiError::InvalidTransfer
        );

        debug!(
            "deposit_spl_transfer: amount: {} from {} to {}, auth {}",
            amount, from.key, to.key, authority.key
        );

        if let Some(mint) = maybe_mint {
            spl_token_2022::onchain::invoke_transfer_checked(
                program.key,
                from,
                mint.to_account_info(),
                to,
                authority,
                remaining_accounts,
                amount,
                mint.decimals,
                &[],
            )?;
        } else {
            #[allow(deprecated)]
            transfer(
                CpiContext::new_with_signer(
                    program,
                    Transfer {
                        from,
                        to,
                        authority,
                    },
                    &[],
                ),
                amount,
            )?;
        }

        Ok(())
    }

    pub fn withdraw_spl_transfer<'info>(
        &self,
        amount: u64,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        maybe_mint: Option<&InterfaceAccount<'info, Mint>>,
        program: AccountInfo<'info>,
        signer_seeds: &[&[&[u8]]],
        remaining_accounts: &[AccountInfo<'info>],
    ) -> MarginfiResult {
        debug!(
            "withdraw_spl_transfer: amount: {} from {} to {}, auth {}",
            amount, from.key, to.key, authority.key
        );

        if let Some(mint) = maybe_mint {
            spl_token_2022::onchain::invoke_transfer_checked(
                program.key,
                from,
                mint.to_account_info(),
                to,
                authority,
                remaining_accounts,
                amount,
                mint.decimals,
                signer_seeds,
            )?;
        } else {
            // `transfer_checked` and `transfer` does the same thing, the additional `_checked` logic
            // is only to assert the expected attributes by the user (mint, decimal scaling),
            //
            // Security of `transfer` is equal to `transfer_checked`.
            #[allow(deprecated)]
            transfer(
                CpiContext::new_with_signer(
                    program,
                    Transfer {
                        from,
                        to,
                        authority,
                    },
                    signer_seeds,
                ),
                amount,
            )?;
        }

        Ok(())
    }

    /// Socialize a loss `loss_amount` among depositors,
    /// the `total_deposit_shares` stays the same, but total value of deposits is
    /// reduced by `loss_amount`;
    pub fn socialize_loss(&mut self, loss_amount: I80F48) -> MarginfiResult {
        let total_asset_shares: I80F48 = self.total_asset_shares.into();
        let old_asset_share_value: I80F48 = self.asset_share_value.into();

        let new_share_value = total_asset_shares
            .checked_mul(old_asset_share_value)
            .ok_or_else(math_error!())?
            .checked_sub(loss_amount)
            .ok_or_else(math_error!())?
            .checked_div(total_asset_shares)
            .ok_or_else(math_error!())?;

        self.asset_share_value = new_share_value.into();

        Ok(())
    }

    pub fn assert_operational_mode(
        &self,
        is_asset_or_liability_amount_increasing: Option<bool>,
    ) -> Result<()> {
        match self.config.operational_state {
            BankOperationalState::Paused => Err(MarginfiError::BankPaused.into()),
            BankOperationalState::Operational => Ok(()),
            BankOperationalState::ReduceOnly => {
                if let Some(is_asset_or_liability_amount_increasing) =
                    is_asset_or_liability_amount_increasing
                {
                    check!(
                        !is_asset_or_liability_amount_increasing,
                        MarginfiError::BankReduceOnly
                    );
                }

                Ok(())
            }
        }
    }

    pub fn get_flag(&self, flag: u64) -> bool {
        (self.flags & flag) == flag
    }

    pub(crate) fn override_emissions_flag(&mut self, flag: u64) {
        assert!(Self::verify_emissions_flags(flag));
        self.flags = flag;
    }

    pub(crate) fn update_flag(&mut self, value: bool, flag: u64) {
        assert!(Self::verify_group_flags(flag));

        if value {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }

    const fn verify_emissions_flags(flags: u64) -> bool {
        flags & EMISSION_FLAGS == flags
    }

    const fn verify_group_flags(flags: u64) -> bool {
        flags & GROUP_FLAGS == flags
    }
}
