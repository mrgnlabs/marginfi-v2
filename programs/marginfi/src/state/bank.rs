#[cfg(not(feature = "client"))]
use crate::events::{GroupEventHeader, LendingPoolBankAccrueInterestEvent};
use crate::{
    check,
    constants::PYTH_PUSH_MIGRATED,
    debug,
    errors::MarginfiError,
    math_error,
    prelude::{MarginfiGroup, MarginfiResult},
    set_if_some,
    state::{
        bank_cache::{update_interest_rates, ComputedInterestRates},
        marginfi_account::{calc_value, RequirementType},
        price::OraclePriceFeedAdapter,
    },
};
use anchor_lang::prelude::*;
use anchor_lang::{
    err,
    prelude::{AccountInfo, CpiContext, InterfaceAccount},
    ToAccountInfo,
};
use anchor_spl::{
    token::{transfer, Transfer},
    token_2022::spl_token_2022,
    token_interface::Mint,
};
use bytemuck::Zeroable;
use fixed::types::I80F48;
use marginfi_type_crate::{
    constants::{
        EMISSION_FLAGS, FEE_VAULT_AUTHORITY_SEED, FEE_VAULT_SEED, FREEZE_SETTINGS, GROUP_FLAGS,
        INSURANCE_VAULT_AUTHORITY_SEED, INSURANCE_VAULT_SEED, LIQUIDITY_VAULT_AUTHORITY_SEED,
        LIQUIDITY_VAULT_SEED, MAX_PYTH_ORACLE_AGE, ORACLE_MIN_AGE,
        PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG, SECONDS_PER_YEAR,
        TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE,
    },
    types::{
        BalanceSide, Bank, BankCache, BankConfig, BankConfigOpt, BankOperationalState,
        EmodeSettings, InterestRateConfig, InterestRateConfigOpt, OracleSetup, RiskTier,
    },
};
use pyth_solana_receiver_sdk::price_update::FeedId;

pub trait BankImpl {
    const LEN: usize = std::mem::size_of::<Bank>();

    #[allow(clippy::too_many_arguments)]
    fn new(
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
    ) -> Self;
    fn get_liability_amount(&self, shares: I80F48) -> MarginfiResult<I80F48>;
    fn get_asset_amount(&self, shares: I80F48) -> MarginfiResult<I80F48>;
    fn get_liability_shares(&self, value: I80F48) -> MarginfiResult<I80F48>;
    fn get_asset_shares(&self, value: I80F48) -> MarginfiResult<I80F48>;
    fn change_asset_shares(&mut self, shares: I80F48, bypass_deposit_limit: bool)
        -> MarginfiResult;
    fn maybe_get_asset_weight_init_discount(&self, price: I80F48)
        -> MarginfiResult<Option<I80F48>>;
    fn change_liability_shares(
        &mut self,
        shares: I80F48,
        bypass_borrow_limit: bool,
    ) -> MarginfiResult;
    fn check_utilization_ratio(&self) -> MarginfiResult;
    fn configure(&mut self, config: &BankConfigOpt) -> MarginfiResult;
    fn configure_unfrozen_fields_only(&mut self, config: &BankConfigOpt) -> MarginfiResult;
    fn accrue_interest(
        &mut self,
        current_timestamp: i64,
        group: &MarginfiGroup,
        #[cfg(not(feature = "client"))] bank: Pubkey,
    ) -> MarginfiResult<()>;
    fn update_bank_cache(&mut self, group: &MarginfiGroup) -> MarginfiResult<()>;
    fn deposit_spl_transfer<'info>(
        &self,
        amount: u64,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        maybe_mint: Option<&InterfaceAccount<'info, Mint>>,
        program: AccountInfo<'info>,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> MarginfiResult;
    fn withdraw_spl_transfer<'info>(
        &self,
        amount: u64,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        maybe_mint: Option<&InterfaceAccount<'info, Mint>>,
        program: AccountInfo<'info>,
        signer_seeds: &[&[&[u8]]],
        remaining_accounts: &[AccountInfo<'info>],
    ) -> MarginfiResult;
    fn socialize_loss(&mut self, loss_amount: I80F48) -> MarginfiResult;
    fn assert_operational_mode(
        &self,
        is_asset_or_liability_amount_increasing: Option<bool>,
    ) -> Result<()>;
    fn get_flag(&self, flag: u64) -> bool;
    fn override_emissions_flag(&mut self, flag: u64);
    fn update_flag(&mut self, value: bool, flag: u64);
    fn verify_emissions_flags(flags: u64) -> bool;
    fn verify_group_flags(flags: u64) -> bool;
}

impl BankImpl for Bank {
    #[allow(clippy::too_many_arguments)]
    fn new(
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
    ) -> Self {
        Self {
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
            emode: EmodeSettings::zeroed(),
            fees_destination_account: Pubkey::default(),
            ..Default::default()
        }
    }

    fn get_liability_amount(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.liability_share_value.into())
            .ok_or_else(math_error!())?)
    }

    fn get_asset_amount(&self, shares: I80F48) -> MarginfiResult<I80F48> {
        Ok(shares
            .checked_mul(self.asset_share_value.into())
            .ok_or_else(math_error!())?)
    }

    fn get_liability_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.liability_share_value.into())
            .ok_or_else(math_error!())?)
    }

    fn get_asset_shares(&self, value: I80F48) -> MarginfiResult<I80F48> {
        Ok(value
            .checked_div(self.asset_share_value.into())
            .ok_or_else(math_error!())?)
    }

    fn change_asset_shares(
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

            if total_deposits_amount >= deposit_limit {
                let deposits_num: f64 = total_deposits_amount.to_num();
                let limit_num: f64 = deposit_limit.to_num();
                msg!("deposits: {:?} deposit lim: {:?}", deposits_num, limit_num);
                return err!(MarginfiError::BankAssetCapacityExceeded);
            }
        }

        Ok(())
    }

    fn maybe_get_asset_weight_init_discount(
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

    fn change_liability_shares(
        &mut self,
        shares: I80F48,
        bypass_borrow_limit: bool,
    ) -> MarginfiResult {
        let total_liability_shares: I80F48 = self.total_liability_shares.into();
        self.total_liability_shares = total_liability_shares
            .checked_add(shares)
            .ok_or_else(math_error!())?
            .into();

        if !bypass_borrow_limit && shares.is_positive() && self.config.is_borrow_limit_active() {
            let total_liability_amount =
                self.get_liability_amount(self.total_liability_shares.into())?;
            let borrow_limit = I80F48::from_num(self.config.borrow_limit);

            if total_liability_amount >= borrow_limit {
                let liab_num: f64 = total_liability_amount.to_num();
                let borrow_num: f64 = borrow_limit.to_num();
                msg!("amt: {:?} borrow lim: {:?}", liab_num, borrow_num);
                return err!(MarginfiError::BankLiabilityCapacityExceeded);
            }
        }

        Ok(())
    }

    fn check_utilization_ratio(&self) -> MarginfiResult {
        let total_assets = self.get_asset_amount(self.total_asset_shares.into())?;
        let total_liabilities = self.get_liability_amount(self.total_liability_shares.into())?;

        if total_assets < total_liabilities {
            let assets_num: f64 = total_assets.to_num();
            let liabs_num: f64 = total_liabilities.to_num();
            msg!("assets: {:?} liabs: {:?}", assets_num, liabs_num);
            return err!(MarginfiError::IllegalUtilizationRatio);
        }

        Ok(())
    }

    fn configure(&mut self, config: &BankConfigOpt) -> MarginfiResult {
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

        set_if_some!(
            self.config.oracle_max_confidence,
            config.oracle_max_confidence
        );

        set_if_some!(self.config.oracle_max_age, config.oracle_max_age);

        if let Some(flag) = config.permissionless_bad_debt_settlement {
            msg!(
                "setting bad debt settlement: {:?}",
                config.permissionless_bad_debt_settlement.unwrap()
            );
            self.update_flag(flag, PERMISSIONLESS_BAD_DEBT_SETTLEMENT_FLAG);
        }

        if let Some(flag) = config.freeze_settings {
            msg!(
                "setting freeze settings: {:?}",
                config.freeze_settings.unwrap()
            );
            self.update_flag(flag, FREEZE_SETTINGS);
        }

        self.config.validate()?;

        Ok(())
    }

    /// Configures just the borrow and deposit limits, ignoring all other values
    fn configure_unfrozen_fields_only(&mut self, config: &BankConfigOpt) -> MarginfiResult {
        set_if_some!(self.config.deposit_limit, config.deposit_limit);
        set_if_some!(self.config.borrow_limit, config.borrow_limit);
        // weights didn't change so no validation is needed
        Ok(())
    }

    /// Calculate the interest rate accrual state changes for a given time period
    ///
    /// Collected protocol and insurance fees are stored in state.
    /// A separate instruction is required to withdraw these fees.
    fn accrue_interest(
        &mut self,
        current_timestamp: i64,
        group: &MarginfiGroup,
        #[cfg(not(feature = "client"))] bank: Pubkey,
    ) -> MarginfiResult<()> {
        #[cfg(all(not(feature = "client"), feature = "debug"))]
        anchor_lang::solana_program::log::sol_log_compute_units();

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

        self.cache.accumulated_since_last_update = asset_share_value
            .checked_sub(I80F48::from(self.asset_share_value))
            .and_then(|v| v.checked_mul(I80F48::from(self.total_asset_shares)))
            .ok_or_else(math_error!())?
            .into();
        self.cache.interest_accumulated_for = time_delta.min(u32::MAX as u64) as u32;
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
            anchor_lang::solana_program::log::sol_log_compute_units();

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

    /// Updates bank cache with the actual values for interest/fee rates.
    ///
    /// Should be called in the end of each instruction calling `accrue_interest` to ensure the cache is up to date.
    fn update_bank_cache(&mut self, group: &MarginfiGroup) -> MarginfiResult<()> {
        let total_assets_amount = self.get_asset_amount(self.total_asset_shares.into())?;
        let total_liabilities_amount =
            self.get_liability_amount(self.total_liability_shares.into())?;

        if (total_assets_amount == I80F48::ZERO) || (total_liabilities_amount == I80F48::ZERO) {
            self.cache = BankCache::default();
            return Ok(());
        }

        let ir_calc = self
            .config
            .interest_rate_config
            .create_interest_rate_calculator(group);

        let utilization_rate = total_liabilities_amount
            .checked_div(total_assets_amount)
            .ok_or_else(math_error!())?;
        let interest_rates = ir_calc
            .calc_interest_rate(utilization_rate)
            .ok_or_else(math_error!())?;

        update_interest_rates(&mut self.cache, &interest_rates);
        Ok(())
    }

    fn deposit_spl_transfer<'info>(
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

    fn withdraw_spl_transfer<'info>(
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
    fn socialize_loss(&mut self, loss_amount: I80F48) -> MarginfiResult {
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

    fn assert_operational_mode(
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

    fn get_flag(&self, flag: u64) -> bool {
        (self.flags & flag) == flag
    }

    fn override_emissions_flag(&mut self, flag: u64) {
        assert!(Self::verify_emissions_flags(flag));
        self.flags = flag;
    }

    fn update_flag(&mut self, value: bool, flag: u64) {
        assert!(Self::verify_group_flags(flag));

        if value {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }

    fn verify_emissions_flags(flags: u64) -> bool {
        flags & EMISSION_FLAGS == flags
    }

    fn verify_group_flags(flags: u64) -> bool {
        flags & GROUP_FLAGS == flags
    }
}

pub trait BankConfigImpl {
    fn get_weights(&self, req_type: RequirementType) -> (I80F48, I80F48);
    fn get_weight(&self, requirement_type: RequirementType, balance_side: BalanceSide) -> I80F48;
    fn validate(&self) -> MarginfiResult;
    fn is_deposit_limit_active(&self) -> bool;
    fn is_borrow_limit_active(&self) -> bool;
    fn is_pyth_push_migrated(&self) -> bool;
    fn update_config_flag(&mut self, value: bool, flag: u8);
    fn validate_oracle_setup(
        &self,
        ais: &[AccountInfo],
        lst_mint: Option<Pubkey>,
        stake_pool: Option<Pubkey>,
        sol_pool: Option<Pubkey>,
    ) -> MarginfiResult;
    fn validate_oracle_age(&self) -> MarginfiResult;
    fn usd_init_limit_active(&self) -> bool;
    fn get_oracle_max_age(&self) -> u64;
    fn get_pyth_push_oracle_feed_id(&self) -> Option<&FeedId>;
}

impl BankConfigImpl for BankConfig {
    #[inline]
    fn get_weights(&self, req_type: RequirementType) -> (I80F48, I80F48) {
        match req_type {
            RequirementType::Initial => (
                self.asset_weight_init.into(),
                self.liability_weight_init.into(),
            ),
            RequirementType::Maintenance => (
                self.asset_weight_maint.into(),
                self.liability_weight_maint.into(),
            ),
            RequirementType::Equity => (I80F48::ONE, I80F48::ONE),
        }
    }

    #[inline]
    fn get_weight(&self, requirement_type: RequirementType, balance_side: BalanceSide) -> I80F48 {
        match (requirement_type, balance_side) {
            (RequirementType::Initial, BalanceSide::Assets) => self.asset_weight_init.into(),
            (RequirementType::Initial, BalanceSide::Liabilities) => {
                self.liability_weight_init.into()
            }
            (RequirementType::Maintenance, BalanceSide::Assets) => self.asset_weight_maint.into(),
            (RequirementType::Maintenance, BalanceSide::Liabilities) => {
                self.liability_weight_maint.into()
            }
            (RequirementType::Equity, _) => I80F48::ONE,
        }
    }

    fn validate(&self) -> MarginfiResult {
        let asset_init_w = I80F48::from(self.asset_weight_init);
        let asset_maint_w = I80F48::from(self.asset_weight_maint);

        check!(
            asset_init_w >= I80F48::ZERO && asset_init_w <= I80F48::ONE,
            MarginfiError::InvalidConfig
        );
        check!(
            asset_maint_w <= (I80F48::ONE + I80F48::ONE),
            MarginfiError::InvalidConfig
        );
        check!(asset_maint_w >= asset_init_w, MarginfiError::InvalidConfig);

        let liab_init_w = I80F48::from(self.liability_weight_init);
        let liab_maint_w = I80F48::from(self.liability_weight_maint);

        check!(liab_init_w >= I80F48::ONE, MarginfiError::InvalidConfig);
        check!(
            liab_maint_w <= liab_init_w && liab_maint_w >= I80F48::ONE,
            MarginfiError::InvalidConfig
        );

        self.interest_rate_config.validate()?;

        if self.risk_tier == RiskTier::Isolated {
            check!(asset_init_w == I80F48::ZERO, MarginfiError::InvalidConfig);
            check!(asset_maint_w == I80F48::ZERO, MarginfiError::InvalidConfig);
        }

        Ok(())
    }

    #[inline]
    fn is_deposit_limit_active(&self) -> bool {
        self.deposit_limit != u64::MAX
    }

    #[inline]
    fn is_borrow_limit_active(&self) -> bool {
        self.borrow_limit != u64::MAX
    }

    fn is_pyth_push_migrated(&self) -> bool {
        (self.config_flags & PYTH_PUSH_MIGRATED) != 0
    }

    fn update_config_flag(&mut self, value: bool, flag: u8) {
        if value {
            self.config_flags |= flag;
        } else {
            self.config_flags &= !flag;
        }
    }

    /// * lst_mint, stake_pool, sol_pool - required only if configuring
    ///   `OracleSetup::StakedWithPythPush` on initial setup. If configuring a staked bank after
    ///   initial setup, can be omitted
    fn validate_oracle_setup(
        &self,
        ais: &[AccountInfo],
        lst_mint: Option<Pubkey>,
        stake_pool: Option<Pubkey>,
        sol_pool: Option<Pubkey>,
    ) -> MarginfiResult {
        OraclePriceFeedAdapter::validate_bank_config(self, ais, lst_mint, stake_pool, sol_pool)?;
        Ok(())
    }

    fn validate_oracle_age(&self) -> MarginfiResult {
        check!(
            self.oracle_max_age >= ORACLE_MIN_AGE,
            MarginfiError::InvalidOracleSetup
        );
        Ok(())
    }

    fn usd_init_limit_active(&self) -> bool {
        self.total_asset_value_init_limit != TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE
    }

    #[inline]
    fn get_oracle_max_age(&self) -> u64 {
        match (self.oracle_max_age, self.oracle_setup) {
            (0, OracleSetup::PythPushOracle) => MAX_PYTH_ORACLE_AGE,
            (n, _) => n as u64,
        }
    }

    fn get_pyth_push_oracle_feed_id(&self) -> Option<&FeedId> {
        if matches!(
            self.oracle_setup,
            OracleSetup::PythPushOracle | OracleSetup::StakedWithPythPush
        ) {
            let bytes: &[u8; 32] = self.oracle_keys[0].as_ref().try_into().unwrap();
            Some(bytes)
        } else {
            None
        }
    }
}

pub trait InterestRateConfigImpl {
    fn create_interest_rate_calculator(&self, group: &MarginfiGroup) -> InterestRateCalc;
    fn validate(&self) -> MarginfiResult;
    fn update(&mut self, ir_config: &InterestRateConfigOpt);
}

impl InterestRateConfigImpl for InterestRateConfig {
    fn create_interest_rate_calculator(&self, group: &MarginfiGroup) -> InterestRateCalc {
        let group_bank_config = &group.get_group_bank_config();
        debug!(
            "Creating interest rate calculator with protocol fees: {}",
            group_bank_config.program_fees
        );
        InterestRateCalc {
            optimal_utilization_rate: self.optimal_utilization_rate.into(),
            plateau_interest_rate: self.plateau_interest_rate.into(),
            max_interest_rate: self.max_interest_rate.into(),
            insurance_fixed_fee: self.insurance_fee_fixed_apr.into(),
            insurance_rate_fee: self.insurance_ir_fee.into(),
            protocol_fixed_fee: self.protocol_fixed_fee_apr.into(),
            protocol_rate_fee: self.protocol_ir_fee.into(),
            add_program_fees: group_bank_config.program_fees,
            program_fee_fixed: group.fee_state_cache.program_fee_fixed.into(),
            program_fee_rate: group.fee_state_cache.program_fee_rate.into(),
        }
    }

    fn validate(&self) -> MarginfiResult {
        let optimal_ur: I80F48 = self.optimal_utilization_rate.into();
        let plateau_ir: I80F48 = self.plateau_interest_rate.into();
        let max_ir: I80F48 = self.max_interest_rate.into();

        check!(
            optimal_ur > I80F48::ZERO && optimal_ur < I80F48::ONE,
            MarginfiError::InvalidConfig
        );
        check!(plateau_ir > I80F48::ZERO, MarginfiError::InvalidConfig);
        check!(max_ir > I80F48::ZERO, MarginfiError::InvalidConfig);
        check!(plateau_ir < max_ir, MarginfiError::InvalidConfig);

        Ok(())
    }

    fn update(&mut self, ir_config: &InterestRateConfigOpt) {
        set_if_some!(
            self.optimal_utilization_rate,
            ir_config.optimal_utilization_rate
        );
        set_if_some!(self.plateau_interest_rate, ir_config.plateau_interest_rate);
        set_if_some!(self.max_interest_rate, ir_config.max_interest_rate);
        set_if_some!(
            self.insurance_fee_fixed_apr,
            ir_config.insurance_fee_fixed_apr
        );
        set_if_some!(self.insurance_ir_fee, ir_config.insurance_ir_fee);
        set_if_some!(
            self.protocol_fixed_fee_apr,
            ir_config.protocol_fixed_fee_apr
        );
        set_if_some!(self.protocol_ir_fee, ir_config.protocol_ir_fee);
        set_if_some!(
            self.protocol_origination_fee,
            ir_config.protocol_origination_fee
        );
    }
}

#[derive(Debug, Clone)]
/// Short for calculator
pub struct InterestRateCalc {
    optimal_utilization_rate: I80F48,
    plateau_interest_rate: I80F48,
    max_interest_rate: I80F48,

    // Fees
    insurance_fixed_fee: I80F48,
    insurance_rate_fee: I80F48,
    /// AKA group fixed fee
    protocol_fixed_fee: I80F48,
    /// AKA group rate fee
    protocol_rate_fee: I80F48,

    program_fee_fixed: I80F48,
    program_fee_rate: I80F48,

    add_program_fees: bool,
}

impl InterestRateCalc {
    /// Return interest rate charged to borrowers and to depositors.
    /// Rate is denominated in APR (0-).
    ///
    /// Return ComputedInterestRates
    pub fn calc_interest_rate(&self, utilization_ratio: I80F48) -> Option<ComputedInterestRates> {
        let Fees {
            insurance_fee_rate,
            insurance_fee_fixed,
            group_fee_rate,
            group_fee_fixed,
            protocol_fee_rate,
            protocol_fee_fixed,
        } = self.get_fees();

        let fee_ir = insurance_fee_rate + group_fee_rate + protocol_fee_rate;
        let fee_fixed = insurance_fee_fixed + group_fee_fixed + protocol_fee_fixed;

        let base_rate_apr = self.interest_rate_curve(utilization_ratio)?;

        // Lending rate is adjusted for utilization ratio to symmetrize payments between borrowers and depositors.
        let lending_rate_apr = base_rate_apr.checked_mul(utilization_ratio)?;

        // Borrowing rate is adjusted for fees.
        // borrowing_rate = base_rate + base_rate * rate_fee + total_fixed_fee_apr
        let borrowing_rate_apr = base_rate_apr
            .checked_mul(I80F48::ONE.checked_add(fee_ir)?)?
            .checked_add(fee_fixed)?;

        let group_fee_apr = calc_fee_rate(base_rate_apr, group_fee_rate, group_fee_fixed)?;
        let insurance_fee_apr =
            calc_fee_rate(base_rate_apr, insurance_fee_rate, insurance_fee_fixed)?;
        let protocol_fee_apr = calc_fee_rate(base_rate_apr, protocol_fee_rate, protocol_fee_fixed)?;

        assert!(lending_rate_apr >= I80F48::ZERO);
        assert!(borrowing_rate_apr >= I80F48::ZERO);
        assert!(group_fee_apr >= I80F48::ZERO);
        assert!(insurance_fee_apr >= I80F48::ZERO);
        assert!(protocol_fee_apr >= I80F48::ZERO);

        // TODO: Add liquidation discount check
        Some(ComputedInterestRates {
            base_rate_apr,
            lending_rate_apr,
            borrowing_rate_apr,
            group_fee_apr,
            insurance_fee_apr,
            protocol_fee_apr,
        })
    }

    /// Piecewise linear interest rate function.
    /// The curves approaches the `plateau_interest_rate` as the utilization ratio approaches the `optimal_utilization_rate`,
    /// once the utilization ratio exceeds the `optimal_utilization_rate`, the curve approaches the `max_interest_rate`.
    ///
    /// To be clear we don't particularly appreciate the piecewise linear nature of this "curve", but it is what it is.
    #[inline]
    fn interest_rate_curve(&self, ur: I80F48) -> Option<I80F48> {
        let optimal_ur: I80F48 = self.optimal_utilization_rate;
        let plateau_ir: I80F48 = self.plateau_interest_rate;
        let max_ir: I80F48 = self.max_interest_rate;

        if ur <= optimal_ur {
            ur.checked_div(optimal_ur)?.checked_mul(plateau_ir)
        } else {
            (ur - optimal_ur)
                .checked_div(I80F48::ONE - optimal_ur)?
                .checked_mul(max_ir - plateau_ir)?
                .checked_add(plateau_ir)
        }
    }

    pub fn get_fees(&self) -> Fees {
        let (protocol_fee_rate, protocol_fee_fixed) = if self.add_program_fees {
            (self.program_fee_rate, self.program_fee_fixed)
        } else {
            (I80F48::ZERO, I80F48::ZERO)
        };

        Fees {
            insurance_fee_rate: self.insurance_rate_fee,
            insurance_fee_fixed: self.insurance_fixed_fee,
            group_fee_rate: self.protocol_rate_fee,
            group_fee_fixed: self.protocol_fixed_fee,
            protocol_fee_rate,
            protocol_fee_fixed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Fees {
    pub insurance_fee_rate: I80F48,
    pub insurance_fee_fixed: I80F48,
    pub group_fee_rate: I80F48,
    pub group_fee_fixed: I80F48,
    pub protocol_fee_rate: I80F48,
    pub protocol_fee_fixed: I80F48,
}

/// Calculates the fee rate for a given base rate and fees specified.
/// The returned rate is only the fee rate without the base rate.
///
/// Used for calculating the fees charged to the borrowers.
fn calc_fee_rate(base_rate: I80F48, rate_fees: I80F48, fixed_fees: I80F48) -> Option<I80F48> {
    if rate_fees.is_zero() {
        return Some(fixed_fees);
    }

    base_rate.checked_mul(rate_fees)?.checked_add(fixed_fees)
}

/// Calculates the accrued interest payment per period `time_delta` in a principal value `value` for interest rate (in APR) `arp`.
/// Result is the new principal value.
fn calc_accrued_interest_payment_per_period(
    apr: I80F48,
    time_delta: u64,
    value: I80F48,
) -> Option<I80F48> {
    let ir_per_period = apr
        .checked_mul(time_delta.into())?
        .checked_div(SECONDS_PER_YEAR)?;

    let new_value = value.checked_mul(I80F48::ONE.checked_add(ir_per_period)?)?;

    Some(new_value)
}

/// Calculates the interest payment for a given period `time_delta` in a principal value `value` for interest rate (in APR) `arp`.
/// Result is the interest payment.
fn calc_interest_payment_for_period(apr: I80F48, time_delta: u64, value: I80F48) -> Option<I80F48> {
    if apr.is_zero() {
        return Some(I80F48::ZERO);
    }

    let interest_payment = value
        .checked_mul(apr)?
        .checked_mul(time_delta.into())?
        .checked_div(SECONDS_PER_YEAR)?;

    Some(interest_payment)
}

/// We use a simple interest rate model that auto settles the accrued interest into the lending account balances.
/// The plan is to move to a compound interest model in the future.
///
/// Simple interest rate model:
/// - `P` - principal
/// - `i` - interest rate (per second)
/// - `t` - time (in seconds)
///
/// `P_t = P_0 * (1 + i) * t`
///
/// We use two interest rates, one for lending and one for borrowing.
///
/// Lending interest rate:
/// - `i_l` - lending interest rate
/// - `i` - base interest rate
/// - `ur` - utilization rate
///
/// `i_l` = `i` * `ur`
///
/// Borrowing interest rate:
/// - `i_b` - borrowing interest rate
/// - `i` - base interest rate
/// - `f_i` - interest rate fee
/// - `f_f` - fixed fee
///
/// `i_b = i * (1 + f_i) + f_f`
///
fn calc_interest_rate_accrual_state_changes(
    time_delta: u64,
    total_assets_amount: I80F48,
    total_liabilities_amount: I80F48,
    interest_rate_calc: &InterestRateCalc,
    asset_share_value: I80F48,
    liability_share_value: I80F48,
) -> Option<InterestRateStateChanges> {
    // If the cache is empty, we need to calculate the interest rates
    let utilization_rate = total_liabilities_amount.checked_div(total_assets_amount)?;
    debug!(
        "Utilization rate: {}, time delta {}s",
        utilization_rate, time_delta
    );
    let interest_rates = interest_rate_calc.calc_interest_rate(utilization_rate)?;

    debug!("{:#?}", interest_rates);

    let ComputedInterestRates {
        lending_rate_apr,
        borrowing_rate_apr,
        group_fee_apr,
        insurance_fee_apr,
        protocol_fee_apr,
        ..
    } = interest_rates;

    Some(InterestRateStateChanges {
        new_asset_share_value: calc_accrued_interest_payment_per_period(
            lending_rate_apr,
            time_delta,
            asset_share_value,
        )?,
        new_liability_share_value: calc_accrued_interest_payment_per_period(
            borrowing_rate_apr,
            time_delta,
            liability_share_value,
        )?,
        insurance_fees_collected: calc_interest_payment_for_period(
            insurance_fee_apr,
            time_delta,
            total_liabilities_amount,
        )?,
        group_fees_collected: calc_interest_payment_for_period(
            group_fee_apr,
            time_delta,
            total_liabilities_amount,
        )?,
        protocol_fees_collected: calc_interest_payment_for_period(
            protocol_fee_apr,
            time_delta,
            total_liabilities_amount,
        )?,
    })
}

pub struct InterestRateStateChanges {
    new_asset_share_value: I80F48,
    new_liability_share_value: I80F48,
    insurance_fees_collected: I80F48,
    group_fees_collected: I80F48,
    protocol_fees_collected: I80F48,
}

#[derive(Debug, Clone)]
pub enum BankVaultType {
    Liquidity,
    Insurance,
    Fee,
}

impl BankVaultType {
    pub fn get_seed(self) -> &'static [u8] {
        match self {
            BankVaultType::Liquidity => LIQUIDITY_VAULT_SEED.as_bytes(),
            BankVaultType::Insurance => INSURANCE_VAULT_SEED.as_bytes(),
            BankVaultType::Fee => FEE_VAULT_SEED.as_bytes(),
        }
    }

    pub fn get_authority_seed(self) -> &'static [u8] {
        match self {
            BankVaultType::Liquidity => LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            BankVaultType::Insurance => INSURANCE_VAULT_AUTHORITY_SEED.as_bytes(),
            BankVaultType::Fee => FEE_VAULT_AUTHORITY_SEED.as_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{
        assert_eq_with_tolerance,
        constants::{PROTOCOL_FEE_FIXED_DEFAULT, PROTOCOL_FEE_RATE_DEFAULT},
        state::{
            bank::{InterestRateConfigImpl, InterestRateStateChanges},
            bank_cache::ComputedInterestRates,
        },
    };

    use super::*;
    use fixed::types::I80F48;
    use fixed_macro::types::I80F48;
    use marginfi_type_crate::types::{Bank, BankConfig, InterestRateConfig};

    #[test]
    /// Tests that the interest payment for a 1 year period with 100% APR is 1.
    fn interest_payment_100apr_1year() {
        let apr = I80F48::ONE;
        let time_delta = 31_536_000; // 1 year
        let value = I80F48::ONE;

        assert_eq_with_tolerance!(
            calc_interest_payment_for_period(apr, time_delta, value).unwrap(),
            I80F48::ONE,
            I80F48!(0.001)
        );
    }

    /// Tests that the interest payment for a 1 year period with 50% APR is 0.5.
    #[test]
    fn interest_payment_50apr_1year() {
        let apr = I80F48::from_num(0.5);
        let time_delta = 31_536_000; // 1 year
        let value = I80F48::ONE;

        assert_eq_with_tolerance!(
            calc_interest_payment_for_period(apr, time_delta, value).unwrap(),
            I80F48::from_num(0.5),
            I80F48!(0.001)
        );
    }
    /// P: 1_000_000
    /// Apr: 12%
    /// Time: 1 second
    #[test]
    fn interest_payment_12apr_1second() {
        let apr = I80F48!(0.12);
        let time_delta = 1;
        let value = I80F48!(1_000_000);

        assert_eq_with_tolerance!(
            calc_interest_payment_for_period(apr, time_delta, value).unwrap(),
            I80F48!(0.0038),
            I80F48!(0.001)
        );
    }

    #[test]
    /// apr: 100%
    /// time: 1 year
    /// principal: 2
    /// expected: 4
    fn accrued_interest_apr100_year1() {
        assert_eq_with_tolerance!(
            calc_accrued_interest_payment_per_period(I80F48!(1), 31_536_000, I80F48!(2)).unwrap(),
            I80F48!(4),
            I80F48!(0.001)
        );
    }

    #[test]
    /// apr: 50%
    /// time: 1 year
    /// principal: 2
    /// expected: 3
    fn accrued_interest_apr50_year1() {
        assert_eq_with_tolerance!(
            calc_accrued_interest_payment_per_period(I80F48!(0.5), 31_536_000, I80F48!(2)).unwrap(),
            I80F48!(3),
            I80F48!(0.001)
        );
    }

    #[test]
    /// apr: 12%
    /// time: 1 second
    /// principal: 1_000_000
    /// expected: 1_038
    fn accrued_interest_apr12_year1() {
        assert_eq_with_tolerance!(
            calc_accrued_interest_payment_per_period(I80F48!(0.12), 1, I80F48!(1_000_000)).unwrap(),
            I80F48!(1_000_000.0038),
            I80F48!(0.001)
        );
    }

    #[test]
    /// ur: 0
    /// protocol_fixed_fee: 0.01
    fn ir_config_calc_interest_rate_pff_01() {
        let config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.6).into(),
            plateau_interest_rate: I80F48!(0.40).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            ..Default::default()
        };

        let ComputedInterestRates {
            base_rate_apr,
            lending_rate_apr: lending_apr,
            borrowing_rate_apr: borrow_apr,
            group_fee_apr: group_fees_apr,
            insurance_fee_apr: insurance_apr,
            protocol_fee_apr,
        } = config
            .create_interest_rate_calculator(&MarginfiGroup::default())
            .calc_interest_rate(I80F48!(0.6))
            .unwrap();

        assert_eq_with_tolerance!(base_rate_apr, I80F48!(0.4), I80F48!(0.001));
        assert_eq_with_tolerance!(lending_apr, I80F48!(0.24), I80F48!(0.001));
        assert_eq_with_tolerance!(borrow_apr, I80F48!(0.41), I80F48!(0.001));
        assert_eq_with_tolerance!(group_fees_apr, I80F48!(0.01), I80F48!(0.001));
        assert_eq_with_tolerance!(insurance_apr, I80F48!(0), I80F48!(0.001));
        assert_eq_with_tolerance!(protocol_fee_apr, I80F48!(0), I80F48!(0.001));
    }

    #[test]
    /// ur: 0.5
    /// protocol_fixed_fee: 0.01
    /// optimal_utilization_rate: 0.5
    /// plateau_interest_rate: 0.4
    fn ir_config_calc_interest_rate_pff_01_ur_05() {
        let config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.5).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let ComputedInterestRates {
            base_rate_apr,
            lending_rate_apr: lending_apr,
            borrowing_rate_apr: borrow_apr,
            group_fee_apr: group_fees_apr,
            insurance_fee_apr: insurance_apr,
            protocol_fee_apr: _,
        } = config
            .create_interest_rate_calculator(&MarginfiGroup::default())
            .calc_interest_rate(I80F48!(0.5))
            .unwrap();

        assert_eq_with_tolerance!(base_rate_apr, I80F48!(0.4), I80F48!(0.001));
        assert_eq_with_tolerance!(lending_apr, I80F48!(0.2), I80F48!(0.001));
        assert_eq_with_tolerance!(borrow_apr, I80F48!(0.45), I80F48!(0.001));
        assert_eq_with_tolerance!(group_fees_apr, I80F48!(0.01), I80F48!(0.001));
        assert_eq_with_tolerance!(insurance_apr, I80F48!(0.04), I80F48!(0.001));
    }

    #[test]
    fn calc_fee_rate_1() {
        let rate = I80F48!(0.4);
        let fee_ir = I80F48!(0.05);
        let fee_fixed = I80F48!(0.01);

        assert_eq!(
            calc_fee_rate(rate, fee_ir, fee_fixed).unwrap(),
            I80F48!(0.03)
        );
    }

    /// ur: 0.8
    /// protocol_fixed_fee: 0.01
    /// optimal_utilization_rate: 0.5
    /// plateau_interest_rate: 0.4
    /// max_interest_rate: 3
    /// insurance_ir_fee: 0.1
    #[test]
    fn ir_config_calc_interest_rate_pff_01_ur_08() {
        let config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let ComputedInterestRates {
            base_rate_apr,
            lending_rate_apr: lending_apr,
            borrowing_rate_apr: borrow_apr,
            group_fee_apr: group_fees_apr,
            insurance_fee_apr: insurance_apr,
            protocol_fee_apr: _,
        } = config
            .create_interest_rate_calculator(&MarginfiGroup::default())
            .calc_interest_rate(I80F48!(0.7))
            .unwrap();

        assert_eq_with_tolerance!(base_rate_apr, I80F48!(1.7), I80F48!(0.001));
        assert_eq_with_tolerance!(lending_apr, I80F48!(1.19), I80F48!(0.001));
        assert_eq_with_tolerance!(borrow_apr, I80F48!(1.88), I80F48!(0.001));
        assert_eq_with_tolerance!(group_fees_apr, I80F48!(0.01), I80F48!(0.001));
        assert_eq_with_tolerance!(insurance_apr, I80F48!(0.17), I80F48!(0.001));
    }

    #[test]
    fn ir_accrual_failing_fuzz_test_example() -> anyhow::Result<()> {
        let ir_config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut bank = Bank {
            asset_share_value: I80F48::ONE.into(),
            liability_share_value: I80F48::ONE.into(),
            total_liability_shares: I80F48!(207_112_621_602).into(),
            total_asset_shares: I80F48!(10_000_000_000_000).into(),
            last_update: current_timestamp,
            config: BankConfig {
                asset_weight_init: I80F48!(0.5).into(),
                asset_weight_maint: I80F48!(0.75).into(),
                liability_weight_init: I80F48!(1.5).into(),
                liability_weight_maint: I80F48!(1.25).into(),
                borrow_limit: u64::MAX,
                deposit_limit: u64::MAX,
                interest_rate_config: ir_config,
                ..Default::default()
            },
            ..Default::default()
        };

        let pre_net_assets = bank.get_asset_amount(bank.total_asset_shares.into())?
            - bank.get_liability_amount(bank.total_liability_shares.into())?;

        let mut clock = Clock::default();

        clock.unix_timestamp = current_timestamp + 3600;

        bank.accrue_interest(
            current_timestamp,
            &MarginfiGroup::default(),
            #[cfg(not(feature = "client"))]
            Pubkey::default(),
        )
        .unwrap();

        let post_collected_fees = I80F48::from(bank.collected_group_fees_outstanding)
            + I80F48::from(bank.collected_insurance_fees_outstanding);

        let post_net_assets = bank.get_asset_amount(bank.total_asset_shares.into())?
            + post_collected_fees
            - bank.get_liability_amount(bank.total_liability_shares.into())?;

        assert_eq_with_tolerance!(pre_net_assets, post_net_assets, I80F48!(1));

        Ok(())
    }

    #[test]
    fn interest_rate_accrual_test_0() -> anyhow::Result<()> {
        let ir_config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let ur = I80F48!(207_112_621_602) / I80F48!(10_000_000_000_000);
        let mut group = MarginfiGroup::default();
        group.group_flags = 1;
        group.fee_state_cache.program_fee_fixed = PROTOCOL_FEE_FIXED_DEFAULT.into();
        group.fee_state_cache.program_fee_rate = PROTOCOL_FEE_RATE_DEFAULT.into();

        let ComputedInterestRates {
            base_rate_apr,
            lending_rate_apr: lending_apr,
            borrowing_rate_apr: borrow_apr,
            group_fee_apr,
            insurance_fee_apr,
            protocol_fee_apr,
        } = ir_config
            .create_interest_rate_calculator(&group)
            .calc_interest_rate(ur)
            .expect("interest rate calculation failed");

        println!("ur: {}", ur);
        println!("base_apr: {}", base_rate_apr);
        println!("lending_apr: {}", lending_apr);
        println!("borrow_apr: {}", borrow_apr);
        println!("group_fee_apr: {}", group_fee_apr);
        println!("insurance_fee_apr: {}", insurance_fee_apr);

        assert_eq_with_tolerance!(
            borrow_apr,
            (lending_apr / ur) + group_fee_apr + insurance_fee_apr + protocol_fee_apr,
            I80F48!(0.001)
        );

        Ok(())
    }

    #[test]
    fn interest_rate_accrual_test_0_no_protocol_fees() -> anyhow::Result<()> {
        let ir_config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let ur = I80F48!(207_112_621_602) / I80F48!(10_000_000_000_000);

        let ComputedInterestRates {
            base_rate_apr,
            lending_rate_apr: lending_apr,
            borrowing_rate_apr: borrow_apr,
            group_fee_apr,
            insurance_fee_apr,
            protocol_fee_apr,
        } = ir_config
            .create_interest_rate_calculator(&MarginfiGroup::default())
            .calc_interest_rate(ur)
            .expect("interest rate calculation failed");

        println!("ur: {}", ur);
        println!("base_apr: {}", base_rate_apr);
        println!("lending_apr: {}", lending_apr);
        println!("borrow_apr: {}", borrow_apr);
        println!("group_fee_apr: {}", group_fee_apr);
        println!("insurance_fee_apr: {}", insurance_fee_apr);

        assert!(protocol_fee_apr.is_zero());

        assert_eq_with_tolerance!(
            borrow_apr,
            (lending_apr / ur) + group_fee_apr + insurance_fee_apr,
            I80F48!(0.001)
        );

        Ok(())
    }

    #[test]
    fn test_accruing_interest() -> anyhow::Result<()> {
        let ir_config = InterestRateConfig {
            optimal_utilization_rate: I80F48!(0.4).into(),
            plateau_interest_rate: I80F48!(0.4).into(),
            protocol_fixed_fee_apr: I80F48!(0.01).into(),
            max_interest_rate: I80F48!(3).into(),
            insurance_ir_fee: I80F48!(0.1).into(),
            ..Default::default()
        };

        let mut group = MarginfiGroup::default();
        group.group_flags = 1;
        group.fee_state_cache.program_fee_fixed = PROTOCOL_FEE_FIXED_DEFAULT.into();
        group.fee_state_cache.program_fee_rate = PROTOCOL_FEE_RATE_DEFAULT.into();

        let liab_share_value = I80F48!(1.0);
        let asset_share_value = I80F48!(1.0);

        let total_liability_shares = I80F48!(207_112_621_602);
        let total_asset_shares = I80F48!(10_000_000_000_000);

        let old_total_liability_amount = liab_share_value * total_liability_shares;
        let old_total_asset_amount = asset_share_value * total_asset_shares;

        let InterestRateStateChanges {
            new_asset_share_value,
            new_liability_share_value: new_liab_share_value,
            insurance_fees_collected: insurance_collected,
            group_fees_collected,
            protocol_fees_collected,
        } = calc_interest_rate_accrual_state_changes(
            3600,
            total_asset_shares,
            total_liability_shares,
            &ir_config.create_interest_rate_calculator(&group),
            asset_share_value,
            liab_share_value,
        )
        .unwrap();

        let new_total_liability_amount = total_liability_shares * new_liab_share_value;
        let new_total_asset_amount = total_asset_shares * new_asset_share_value;

        println!("new_asset_share_value: {}", new_asset_share_value);
        println!("new_liab_share_value: {}", new_liab_share_value);
        println!("group_fees_collected: {}", group_fees_collected);
        println!("insurance_collected: {}", insurance_collected);
        println!("protocol_fees_collected: {}", protocol_fees_collected);

        println!("new_total_liability_amount: {}", new_total_liability_amount);
        println!("new_total_asset_amount: {}", new_total_asset_amount);

        println!("old_total_liability_amount: {}", old_total_liability_amount);
        println!("old_total_asset_amount: {}", old_total_asset_amount);

        let total_fees_collected =
            group_fees_collected + insurance_collected + protocol_fees_collected;

        println!("total_fee_collected: {}", total_fees_collected);

        println!(
            "diff: {}",
            ((new_total_asset_amount - new_total_liability_amount) + total_fees_collected)
                - (old_total_asset_amount - old_total_liability_amount)
        );

        assert_eq_with_tolerance!(
            (new_total_asset_amount - new_total_liability_amount) + total_fees_collected,
            old_total_asset_amount - old_total_liability_amount,
            I80F48::ONE
        );

        Ok(())
    }
}
