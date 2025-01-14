use std::{cell::Ref, cmp::min};

use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use enum_dispatch::enum_dispatch;
use fixed::types::I80F48;
use pyth_sdk_solana::{state::SolanaPriceAccount, Price, PriceFeed};
use pyth_solana_receiver_sdk::price_update::{self, FeedId, PriceUpdateV2};
use switchboard_on_demand::{CurrentResult, PullFeedAccountData, SPL_TOKEN_PROGRAM_ID};
use switchboard_solana::{
    AggregatorAccountData, AggregatorResolutionMode, SwitchboardDecimal, SWITCHBOARD_PROGRAM_ID,
};

pub use pyth_sdk_solana;

use crate::{
    check,
    constants::{
        CONF_INTERVAL_MULTIPLE, EXP_10, EXP_10_I80F48, MAX_CONF_INTERVAL,
        MIN_PYTH_PUSH_VERIFICATION_LEVEL, NATIVE_STAKE_ID, PYTH_ID, SPL_SINGLE_POOL_ID,
        STD_DEV_MULTIPLE, SWITCHBOARD_PULL_ID,
    },
    debug, live, math_error,
    prelude::*,
};

use super::bank::BankConfig;
use anchor_lang::prelude::borsh;
use pyth_solana_receiver_sdk::PYTH_PUSH_ORACLE_ID;

#[repr(u8)]
#[cfg_attr(any(feature = "test", feature = "client"), derive(PartialEq, Eq))]
#[derive(Copy, Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum OracleSetup {
    None,
    PythLegacy,
    SwitchboardV2,
    PythPushOracle,
    SwitchboardPull,
    StakedWithPythPush,
}

#[derive(Copy, Clone, Debug)]
pub enum PriceBias {
    Low,
    High,
}

#[derive(Copy, Clone, Debug)]
pub enum OraclePriceType {
    /// Time weighted price
    /// EMA for PythEma
    TimeWeighted,
    /// Real time price
    RealTime,
}

#[enum_dispatch]
pub trait PriceAdapter {
    fn get_price_of_type(
        &self,
        oracle_price_type: OraclePriceType,
        bias: Option<PriceBias>,
    ) -> MarginfiResult<I80F48>;
}

#[enum_dispatch(PriceAdapter)]
#[cfg_attr(feature = "client", derive(Clone))]
pub enum OraclePriceFeedAdapter {
    PythLegacy(PythLegacyPriceFeed),
    SwitchboardV2(SwitchboardV2PriceFeed),
    PythPushOracle(PythPushOraclePriceFeed),
    SwitchboardPull(SwitchboardPullPriceFeed),
}

impl OraclePriceFeedAdapter {
    pub fn try_from_bank_config<'info>(
        bank_config: &BankConfig,
        ais: &'info [AccountInfo<'info>],
        clock: &Clock,
    ) -> MarginfiResult<Self> {
        Self::try_from_bank_config_with_max_age(
            bank_config,
            ais,
            clock,
            bank_config.get_oracle_max_age(),
        )
    }

    pub fn try_from_bank_config_with_max_age<'info>(
        bank_config: &BankConfig,
        ais: &'info [AccountInfo<'info>],
        clock: &Clock,
        max_age: u64,
    ) -> MarginfiResult<Self> {
        match bank_config.oracle_setup {
            OracleSetup::None => Err(MarginfiError::OracleNotSetup.into()),
            OracleSetup::PythLegacy => {
                check!(ais.len() == 1, MarginfiError::InvalidOracleAccount);
                check!(
                    ais[0].key == &bank_config.oracle_keys[0],
                    MarginfiError::InvalidOracleAccount
                );

                let account_info = &ais[0];

                Ok(OraclePriceFeedAdapter::PythLegacy(
                    PythLegacyPriceFeed::load_checked(account_info, clock.unix_timestamp, max_age)?,
                ))
            }
            OracleSetup::SwitchboardV2 => {
                check!(ais.len() == 1, MarginfiError::InvalidOracleAccount);
                check!(
                    ais[0].key == &bank_config.oracle_keys[0],
                    MarginfiError::InvalidOracleAccount
                );

                Ok(OraclePriceFeedAdapter::SwitchboardV2(
                    SwitchboardV2PriceFeed::load_checked(&ais[0], clock.unix_timestamp, max_age)?,
                ))
            }
            OracleSetup::PythPushOracle => {
                check!(ais.len() == 1, MarginfiError::InvalidOracleAccount);

                let account_info = &ais[0];

                check!(
                    account_info.owner == &pyth_solana_receiver_sdk::id(),
                    MarginfiError::InvalidOracleAccount
                );

                let price_feed_id = bank_config.get_pyth_push_oracle_feed_id().unwrap();

                Ok(OraclePriceFeedAdapter::PythPushOracle(
                    PythPushOraclePriceFeed::load_checked(
                        account_info,
                        price_feed_id,
                        clock,
                        max_age,
                    )?,
                ))
            }
            OracleSetup::SwitchboardPull => {
                check!(ais.len() == 1, MarginfiError::InvalidOracleAccount);
                check!(
                    ais[0].key == &bank_config.oracle_keys[0],
                    MarginfiError::InvalidOracleAccount
                );

                Ok(OraclePriceFeedAdapter::SwitchboardPull(
                    SwitchboardPullPriceFeed::load_checked(&ais[0], clock.unix_timestamp, max_age)?,
                ))
            }
            OracleSetup::StakedWithPythPush => {
                check!(ais.len() == 3, MarginfiError::InvalidOracleAccount);

                check!(
                    ais[1].key == &bank_config.oracle_keys[1]
                        && ais[2].key == &bank_config.oracle_keys[2],
                    MarginfiError::InvalidOracleAccount
                );

                let lst_mint = Account::<'info, Mint>::try_from(&ais[1]).unwrap();
                let lst_supply = lst_mint.supply;
                let sol_pool_balance = ais[2].lamports();
                // Note: exchange rate is `sol_pool_balance / lst_supply`, but we will do the
                // division last to avoid precision loss. Division does not need to be
                // decimal-adjusted because both SOL and stake positions use 9 decimals

                // Note: mainnet/staging/devnet use "push" oracles, localnet uses legacy
                if cfg!(any(
                    feature = "mainnet-beta",
                    feature = "staging",
                    feature = "devnet"
                )) {
                    let account_info = &ais[0];

                    check!(
                        account_info.owner == &pyth_solana_receiver_sdk::id(),
                        MarginfiError::InvalidOracleAccount
                    );

                    let price_feed_id = bank_config.get_pyth_push_oracle_feed_id().unwrap();
                    let mut feed = PythPushOraclePriceFeed::load_checked(
                        account_info,
                        price_feed_id,
                        clock,
                        max_age,
                    )?;
                    let adjusted_price = (feed.price.price as i128)
                        .checked_mul(sol_pool_balance as i128)
                        .ok_or_else(math_error!())?
                        .checked_div(lst_supply as i128)
                        .ok_or_else(math_error!())?;
                    feed.price.price = adjusted_price.try_into().unwrap();

                    let adjusted_ema_price = (feed.ema_price.price as i128)
                        .checked_mul(sol_pool_balance as i128)
                        .ok_or_else(math_error!())?
                        .checked_div(lst_supply as i128)
                        .ok_or_else(math_error!())?;
                    feed.ema_price.price = adjusted_ema_price.try_into().unwrap();

                    let price = OraclePriceFeedAdapter::PythPushOracle(feed);
                    Ok(price)
                } else {
                    // Localnet only
                    check!(
                        ais[0].key == &bank_config.oracle_keys[0],
                        MarginfiError::InvalidOracleAccount
                    );

                    let account_info = &ais[0];
                    let mut feed = PythLegacyPriceFeed::load_checked(
                        account_info,
                        clock.unix_timestamp,
                        max_age,
                    )?;

                    let adjusted_price = (feed.price.price as i128)
                        .checked_mul(sol_pool_balance as i128)
                        .ok_or_else(math_error!())?
                        .checked_div(lst_supply as i128)
                        .ok_or_else(math_error!())?;
                    feed.price.price = adjusted_price.try_into().unwrap();

                    let adjusted_ema_price = (feed.ema_price.price as i128)
                        .checked_mul(sol_pool_balance as i128)
                        .ok_or_else(math_error!())?
                        .checked_div(lst_supply as i128)
                        .ok_or_else(math_error!())?;
                    feed.ema_price.price = adjusted_ema_price.try_into().unwrap();

                    let price = OraclePriceFeedAdapter::PythLegacy(feed);
                    Ok(price)
                }
            }
        }
    }

    /// * lst_mint, stake_pool, sol_pool - required only if configuring
    ///   `OracleSetup::StakedWithPythPush` initially. (subsequent validations of staked banks can
    ///   omit these)
    pub fn validate_bank_config(
        bank_config: &BankConfig,
        oracle_ais: &[AccountInfo],
        lst_mint: Option<Pubkey>,
        stake_pool: Option<Pubkey>,
        sol_pool: Option<Pubkey>,
    ) -> MarginfiResult {
        match bank_config.oracle_setup {
            OracleSetup::None => Err(MarginfiError::OracleNotSetup.into()),
            OracleSetup::PythLegacy => {
                check!(oracle_ais.len() == 1, MarginfiError::InvalidOracleAccount);
                check!(
                    oracle_ais[0].key == &bank_config.oracle_keys[0],
                    MarginfiError::InvalidOracleAccount
                );

                PythLegacyPriceFeed::check_ais(&oracle_ais[0])?;

                Ok(())
            }
            OracleSetup::SwitchboardV2 => {
                check!(oracle_ais.len() == 1, MarginfiError::InvalidOracleAccount);
                check!(
                    oracle_ais[0].key == &bank_config.oracle_keys[0],
                    MarginfiError::InvalidOracleAccount
                );

                SwitchboardV2PriceFeed::check_ais(&oracle_ais[0])?;

                Ok(())
            }
            OracleSetup::PythPushOracle => {
                check!(oracle_ais.len() == 1, MarginfiError::InvalidOracleAccount);

                PythPushOraclePriceFeed::check_ai_and_feed_id(
                    &oracle_ais[0],
                    bank_config.get_pyth_push_oracle_feed_id().unwrap(),
                )?;

                Ok(())
            }
            OracleSetup::SwitchboardPull => {
                check!(oracle_ais.len() == 1, MarginfiError::InvalidOracleAccount);
                check!(
                    oracle_ais[0].key == &bank_config.oracle_keys[0],
                    MarginfiError::InvalidOracleAccount
                );

                SwitchboardPullPriceFeed::check_ais(&oracle_ais[0])?;

                Ok(())
            }
            OracleSetup::StakedWithPythPush => {
                if lst_mint.is_some() && stake_pool.is_some() && sol_pool.is_some() {
                    check!(oracle_ais.len() == 3, MarginfiError::InvalidOracleAccount);

                    // Note: mainnet/staging/devnet use "push" oracles, localnet uses legacy
                    if live!() {
                        PythPushOraclePriceFeed::check_ai_and_feed_id(
                            &oracle_ais[0],
                            bank_config.get_pyth_push_oracle_feed_id().unwrap(),
                        )?;
                    } else {
                        // Localnet only
                        check!(
                            oracle_ais[0].key == &bank_config.oracle_keys[0],
                            MarginfiError::InvalidOracleAccount
                        );

                        PythLegacyPriceFeed::check_ais(&oracle_ais[0])?;
                    }

                    let lst_mint = lst_mint.unwrap();
                    let stake_pool = stake_pool.unwrap();
                    let sol_pool = sol_pool.unwrap();

                    let program_id = &SPL_SINGLE_POOL_ID;
                    let stake_pool_bytes = &stake_pool.to_bytes();
                    // Validate the given stake_pool derives the same lst_mint, proving stake_pool is correct
                    let (exp_mint, _) =
                        Pubkey::find_program_address(&[b"mint", stake_pool_bytes], program_id);
                    check!(
                        exp_mint == lst_mint,
                        MarginfiError::StakePoolValidationFailed
                    );
                    // Validate the now-proven stake_pool derives the given sol_pool
                    let (exp_pool, _) =
                        Pubkey::find_program_address(&[b"stake", stake_pool_bytes], program_id);
                    check!(
                        exp_pool == sol_pool.key(),
                        MarginfiError::StakePoolValidationFailed
                    );

                    // Sanity check the mint. Note: spl-single-pool uses a classic Token, never Token22
                    check!(
                        oracle_ais[1].owner == &SPL_TOKEN_PROGRAM_ID
                            && oracle_ais[1].key() == lst_mint,
                        MarginfiError::StakePoolValidationFailed
                    );
                    // Sanity check the pool is a native stake pool. Note: the native staking program is
                    // written in vanilla Solana and has no Anchor discriminator.
                    check!(
                        oracle_ais[2].owner == &NATIVE_STAKE_ID && oracle_ais[2].key() == sol_pool,
                        MarginfiError::StakePoolValidationFailed
                    );

                    Ok(())
                } else {
                    // light validation (after initial setup, only the Pyth oracle needs to be validated)
                    check!(oracle_ais.len() == 1, MarginfiError::InvalidOracleAccount);
                    // Note: mainnet/staging/devnet use push oracles, localnet uses legacy push
                    if live!() {
                        PythPushOraclePriceFeed::check_ai_and_feed_id(
                            &oracle_ais[0],
                            bank_config.get_pyth_push_oracle_feed_id().unwrap(),
                        )?;
                    } else {
                        // Localnet only
                        PythLegacyPriceFeed::check_ais(&oracle_ais[0])?;
                    }

                    Ok(())
                }
            }
        }
    }
}

#[cfg_attr(feature = "client", derive(Clone, Debug))]
pub struct PythLegacyPriceFeed {
    ema_price: Box<Price>,
    price: Box<Price>,
}

impl PythLegacyPriceFeed {
    pub fn load_checked(ai: &AccountInfo, current_time: i64, max_age: u64) -> MarginfiResult<Self> {
        let price_feed = load_pyth_price_feed(ai)?;

        // Note: mainnet/staging/devnet use oracle age, localnet ignores oracle age
        let ema_price = if live!() {
            price_feed
                .get_ema_price_no_older_than(current_time, max_age)
                .ok_or(MarginfiError::StaleOracle)?
        } else {
            price_feed.get_ema_price_unchecked()
        };

        let price = if live!() {
            price_feed
                .get_price_no_older_than(current_time, max_age)
                .ok_or(MarginfiError::StaleOracle)?
        } else {
            price_feed.get_price_unchecked()
        };

        Ok(Self {
            ema_price: Box::new(ema_price),
            price: Box::new(price),
        })
    }

    fn check_ais(ai: &AccountInfo) -> MarginfiResult {
        load_pyth_price_feed(ai)?;
        Ok(())
    }

    fn get_confidence_interval(&self, use_ema: bool) -> MarginfiResult<I80F48> {
        let price = if use_ema {
            &self.ema_price
        } else {
            &self.price
        };

        let conf_interval =
            pyth_price_components_to_i80f48(I80F48::from_num(price.conf), price.expo)?
                .checked_mul(CONF_INTERVAL_MULTIPLE)
                .ok_or_else(math_error!())?;

        // Cap confidence interval to 5% of price
        let price = pyth_price_components_to_i80f48(I80F48::from_num(price.price), price.expo)?;

        let max_conf_interval = price
            .checked_mul(MAX_CONF_INTERVAL)
            .ok_or_else(math_error!())?;

        assert!(
            max_conf_interval >= I80F48::ZERO,
            "Negative max confidence interval"
        );

        assert!(
            conf_interval >= I80F48::ZERO,
            "Negative confidence interval"
        );

        Ok(min(conf_interval, max_conf_interval))
    }

    #[inline(always)]
    fn get_ema_price(&self) -> MarginfiResult<I80F48> {
        pyth_price_components_to_i80f48(I80F48::from_num(self.ema_price.price), self.ema_price.expo)
    }

    #[inline(always)]
    fn get_unweighted_price(&self) -> MarginfiResult<I80F48> {
        pyth_price_components_to_i80f48(I80F48::from_num(self.price.price), self.price.expo)
    }
}

impl PriceAdapter for PythLegacyPriceFeed {
    fn get_price_of_type(
        &self,
        price_type: OraclePriceType,
        bias: Option<PriceBias>,
    ) -> MarginfiResult<I80F48> {
        let price = match price_type {
            OraclePriceType::TimeWeighted => self.get_ema_price()?,
            OraclePriceType::RealTime => self.get_unweighted_price()?,
        };

        match bias {
            None => Ok(price),
            Some(price_bias) => {
                let confidence_interval = self
                    .get_confidence_interval(matches!(price_type, OraclePriceType::TimeWeighted))?;

                match price_bias {
                    PriceBias::Low => Ok(price
                        .checked_sub(confidence_interval)
                        .ok_or_else(math_error!())?),
                    PriceBias::High => Ok(price
                        .checked_add(confidence_interval)
                        .ok_or_else(math_error!())?),
                }
            }
        }
    }
}

#[cfg_attr(feature = "client", derive(Clone, Debug))]
pub struct SwitchboardPullPriceFeed {
    pub feed: Box<LitePullFeedAccountData>,
}

impl SwitchboardPullPriceFeed {
    pub fn load_checked(
        ai: &AccountInfo,
        current_timestamp: i64,
        max_age: u64,
    ) -> MarginfiResult<Self> {
        let ai_data = ai.data.borrow();

        check!(
            ai.owner.eq(&SWITCHBOARD_PULL_ID),
            MarginfiError::InvalidOracleAccount
        );

        let feed =
            PullFeedAccountData::parse(ai_data).map_err(|_| MarginfiError::InvalidOracleAccount)?;

        // Check staleness
        let last_updated = feed.last_update_timestamp;
        if current_timestamp.saturating_sub(last_updated) > max_age as i64 {
            return err!(MarginfiError::StaleOracle);
        }

        Ok(Self {
            feed: Box::new(feed.into()),
        })
    }

    fn check_ais(ai: &AccountInfo) -> MarginfiResult {
        let ai_data = ai.data.borrow();

        check!(
            ai.owner.eq(&SWITCHBOARD_PULL_ID),
            MarginfiError::InvalidOracleAccount
        );

        PullFeedAccountData::parse(ai_data).map_err(|_| MarginfiError::InvalidOracleAccount)?;

        Ok(())
    }

    fn get_price(&self) -> MarginfiResult<I80F48> {
        let sw_result = self.feed.result;
        // Note: Pull oracles support mean (result.mean) or median (result.value)
        let price: I80F48 = I80F48::from_num(sw_result.value)
            .checked_div(EXP_10_I80F48[switchboard_on_demand::PRECISION as usize])
            .ok_or_else(math_error!())?;

        // WARNING: Adding a line like the following will cause the entire project to silently fail
        // to build, resulting in `Program not deployed` errors downstream when testing

        // msg!("recorded price: {:?}", price);

        Ok(price)
    }

    fn get_confidence_interval(&self) -> MarginfiResult<I80F48> {
        let std_div: I80F48 = I80F48::from_num(self.feed.result.std_dev);

        let conf_interval = std_div
            .checked_mul(STD_DEV_MULTIPLE)
            .ok_or_else(math_error!())?;

        let price = self.get_price()?;

        let max_conf_interval = price
            .checked_mul(MAX_CONF_INTERVAL)
            .ok_or_else(math_error!())?;

        assert!(
            max_conf_interval >= I80F48::ZERO,
            "Negative max confidence interval"
        );

        assert!(
            conf_interval >= I80F48::ZERO,
            "Negative confidence interval"
        );

        Ok(min(conf_interval, max_conf_interval))
    }
}

impl PriceAdapter for SwitchboardPullPriceFeed {
    fn get_price_of_type(
        &self,
        _price_type: OraclePriceType,
        bias: Option<PriceBias>,
    ) -> MarginfiResult<I80F48> {
        let price = self.get_price()?;

        match bias {
            Some(price_bias) => {
                let confidence_interval = self.get_confidence_interval()?;

                match price_bias {
                    PriceBias::Low => Ok(price
                        .checked_sub(confidence_interval)
                        .ok_or_else(math_error!())?),
                    PriceBias::High => Ok(price
                        .checked_add(confidence_interval)
                        .ok_or_else(math_error!())?),
                }
            }
            None => Ok(price),
        }
    }
}

#[cfg_attr(feature = "client", derive(Clone, Debug))]
pub struct SwitchboardV2PriceFeed {
    aggregator_account: Box<LiteAggregatorAccountData>,
}

impl SwitchboardV2PriceFeed {
    pub fn load_checked(
        ai: &AccountInfo,
        current_timestamp: i64,
        max_age: u64,
    ) -> MarginfiResult<Self> {
        let ai_data = ai.data.borrow();

        check!(
            ai.owner.eq(&SWITCHBOARD_PROGRAM_ID),
            MarginfiError::InvalidOracleAccount
        );

        let aggregator_account = AggregatorAccountData::new_from_bytes(&ai_data)
            .map_err(|_| MarginfiError::InvalidOracleAccount)?;

        aggregator_account
            .check_staleness(current_timestamp, max_age as i64)
            .map_err(|_| MarginfiError::StaleOracle)?;

        Ok(Self {
            aggregator_account: Box::new(aggregator_account.into()),
        })
    }

    fn check_ais(ai: &AccountInfo) -> MarginfiResult {
        let ai_data = ai.data.borrow();

        check!(
            ai.owner.eq(&SWITCHBOARD_PROGRAM_ID),
            MarginfiError::InvalidOracleAccount
        );

        AggregatorAccountData::new_from_bytes(&ai_data)
            .map_err(|_| MarginfiError::InvalidOracleAccount)?;

        Ok(())
    }

    fn get_price(&self) -> MarginfiResult<I80F48> {
        let sw_decimal = self
            .aggregator_account
            .get_result()
            .map_err(|_| MarginfiError::InvalidPrice)?;

        Ok(switchboard_decimal_to_i80f48(sw_decimal)
            .ok_or(MarginfiError::InvalidSwitchboardDecimalConversion)?)
    }

    fn get_confidence_interval(&self) -> MarginfiResult<I80F48> {
        let std_div = self.aggregator_account.latest_confirmed_round_std_deviation;
        let std_div = switchboard_decimal_to_i80f48(std_div)
            .ok_or(MarginfiError::InvalidSwitchboardDecimalConversion)?;

        let conf_interval = std_div
            .checked_mul(STD_DEV_MULTIPLE)
            .ok_or_else(math_error!())?;

        let price = self.get_price()?;

        let max_conf_interval = price
            .checked_mul(MAX_CONF_INTERVAL)
            .ok_or_else(math_error!())?;

        assert!(
            max_conf_interval >= I80F48::ZERO,
            "Negative max confidence interval"
        );

        assert!(
            conf_interval >= I80F48::ZERO,
            "Negative confidence interval"
        );

        Ok(min(conf_interval, max_conf_interval))
    }
}

impl PriceAdapter for SwitchboardV2PriceFeed {
    fn get_price_of_type(
        &self,
        _price_type: OraclePriceType,
        bias: Option<PriceBias>,
    ) -> MarginfiResult<I80F48> {
        let price = self.get_price()?;

        match bias {
            Some(price_bias) => {
                let confidence_interval = self.get_confidence_interval()?;

                match price_bias {
                    PriceBias::Low => Ok(price
                        .checked_sub(confidence_interval)
                        .ok_or_else(math_error!())?),
                    PriceBias::High => Ok(price
                        .checked_add(confidence_interval)
                        .ok_or_else(math_error!())?),
                }
            }
            None => Ok(price),
        }
    }
}

pub fn load_price_update_v2_checked(ai: &AccountInfo) -> MarginfiResult<PriceUpdateV2> {
    check!(
        ai.owner.eq(&pyth_solana_receiver_sdk::id()),
        MarginfiError::InvalidOracleAccount
    );

    let price_feed_data = ai.try_borrow_data()?;
    let discriminator = &price_feed_data[0..8];

    check!(
        discriminator == <PriceUpdateV2 as anchor_lang_29::Discriminator>::DISCRIMINATOR,
        MarginfiError::InvalidOracleAccount
    );

    Ok(PriceUpdateV2::deserialize(
        &mut &price_feed_data.as_ref()[8..],
    )?)
}

#[cfg_attr(feature = "client", derive(Clone, Debug))]
pub struct PythPushOraclePriceFeed {
    ema_price: Box<pyth_solana_receiver_sdk::price_update::Price>,
    price: Box<pyth_solana_receiver_sdk::price_update::Price>,
}

impl PythPushOraclePriceFeed {
    /// Pyth push oracles are update using crosschain messages from pythnet
    /// There can be multiple pyth push oracles for a given feed_id. Marginfi allows using any
    /// pyth push oracle with a sufficient verification level and price age.
    ///
    /// Meaning that when loading the pyth push oracle, we don't verify the account address
    /// directly, but rather we verify the feed_id in the oracle data.
    ///
    /// Security assumptions:
    /// - The pyth-push-oracle account is owned by the pyth-solana-receiver program, checked in `load_price_update_v2_checked`
    /// - The pyth-push-oracle account is a PriceUpdateV2 account, checked in `load_price_update_v2_checked`
    /// - The pyth-push-oracle account has a minimum verification level, checked in `get_price_no_older_than_with_custom_verification_level`
    /// - The pyth-push-oracle account has a valid feed_id, the pyth-solana-receiver program enforces that the feed_id matches the pythnet feed_id, checked in
    ///     - `get_price_no_older_than_with_custom_verification_level` checks against the feed_id stored in the bank_config
    ///     - pyth-push-oracle asserts the a valid price update has a matching feed_id with the existing pyth-push-oracle update https://github.com/pyth-network/pyth-crosschain/blob/94f1bd54612adc3e186eaf0bb0f1f705880f20a6/target_chains/solana/programs/pyth-push-oracle/src/lib.rs#L101
    ///     - pyth-solana-receiver set the feed_id directly from a pythnet verified price_update message https://github.com/pyth-network/pyth-crosschain/blob/94f1bd54612adc3e186eaf0bb0f1f705880f20a6/target_chains/solana/programs/pyth-solana-receiver/src/lib.rs#L437
    /// - The pyth-push-oracle account is not older than the max_age, checked in `get_price_no_older_than_with_custom_verification_level`
    pub fn load_checked(
        ai: &AccountInfo,
        feed_id: &FeedId,
        clock: &Clock,
        max_age: u64,
    ) -> MarginfiResult<Self> {
        let price_feed_account = load_price_update_v2_checked(ai)?;

        let price = price_feed_account
            .get_price_no_older_than_with_custom_verification_level(
                clock,
                max_age,
                feed_id,
                MIN_PYTH_PUSH_VERIFICATION_LEVEL,
            )
            .map_err(|e| {
                debug!("Pyth push oracle error: {:?}", e);

                match e {
                    pyth_solana_receiver_sdk::error::GetPriceError::PriceTooOld => {
                        MarginfiError::StaleOracle
                    }
                    _ => MarginfiError::InvalidOracleAccount,
                }
            })?;

        let ema_price = {
            let price_update::PriceFeedMessage {
                exponent,
                publish_time,
                ema_price,
                ema_conf,
                ..
            } = price_feed_account.price_message;

            pyth_solana_receiver_sdk::price_update::Price {
                price: ema_price,
                conf: ema_conf,
                exponent,
                publish_time,
            }
        };

        Ok(Self {
            price: Box::new(price),
            ema_price: Box::new(ema_price),
        })
    }

    #[cfg(feature = "client")]
    pub fn load_unchecked(ai: &AccountInfo) -> MarginfiResult<Self> {
        let price_feed_account = load_price_update_v2_checked(ai)?;

        let price = price_feed_account
            .get_price_unchecked(&price_feed_account.price_message.feed_id)
            .map_err(|e| {
                println!("Pyth push oracle error: {:?}", e);

                match e {
                    pyth_solana_receiver_sdk::error::GetPriceError::PriceTooOld => {
                        MarginfiError::StaleOracle
                    }
                    _ => MarginfiError::InvalidOracleAccount,
                }
            })?;

        let ema_price = {
            let price_update::PriceFeedMessage {
                exponent,
                publish_time,
                ema_price,
                ema_conf,
                ..
            } = price_feed_account.price_message;

            pyth_solana_receiver_sdk::price_update::Price {
                price: ema_price,
                conf: ema_conf,
                exponent,
                publish_time,
            }
        };

        Ok(Self {
            price: Box::new(price),
            ema_price: Box::new(ema_price),
        })
    }

    #[cfg(feature = "client")]
    pub fn peek_feed_id(ai: &AccountInfo) -> MarginfiResult<FeedId> {
        let price_feed_account = load_price_update_v2_checked(ai)?;

        Ok(price_feed_account.price_message.feed_id)
    }

    pub fn check_ai_and_feed_id(ai: &AccountInfo, feed_id: &FeedId) -> MarginfiResult {
        let price_feed_account = load_price_update_v2_checked(ai)?;

        check!(
            &price_feed_account.price_message.feed_id.eq(feed_id),
            MarginfiError::InvalidOracleAccount
        );

        Ok(())
    }

    fn get_confidence_interval(&self, use_ema: bool) -> MarginfiResult<I80F48> {
        let price = if use_ema {
            &self.ema_price
        } else {
            &self.price
        };

        let conf_interval =
            pyth_price_components_to_i80f48(I80F48::from_num(price.conf), price.exponent)?
                .checked_mul(CONF_INTERVAL_MULTIPLE)
                .ok_or_else(math_error!())?;

        // Cap confidence interval to 5% of price
        let price = pyth_price_components_to_i80f48(I80F48::from_num(price.price), price.exponent)?;

        let max_conf_interval = price
            .checked_mul(MAX_CONF_INTERVAL)
            .ok_or_else(math_error!())?;

        assert!(
            max_conf_interval >= I80F48::ZERO,
            "Negative max confidence interval"
        );

        assert!(
            conf_interval >= I80F48::ZERO,
            "Negative confidence interval"
        );

        Ok(min(conf_interval, max_conf_interval))
    }

    #[inline(always)]
    fn get_ema_price(&self) -> MarginfiResult<I80F48> {
        pyth_price_components_to_i80f48(
            I80F48::from_num(self.ema_price.price),
            self.ema_price.exponent,
        )
    }

    #[inline(always)]
    fn get_unweighted_price(&self) -> MarginfiResult<I80F48> {
        pyth_price_components_to_i80f48(I80F48::from_num(self.price.price), self.price.exponent)
    }

    /// Find PDA address of a pyth push oracle given a shard_id and feed_id
    ///
    /// Pyth sponsored feed id
    /// `constants::PYTH_PUSH_PYTH_SPONSORED_SHARD_ID = 0`
    ///
    /// Marginfi sponsored feed id
    /// `constants::PYTH_PUSH_MARGINFI_SPONSORED_SHARD_ID = 3301`
    pub fn find_oracle_address(shard_id: u16, feed_id: &FeedId) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[&shard_id.to_le_bytes(), feed_id], &PYTH_PUSH_ORACLE_ID)
    }
}

impl PriceAdapter for PythPushOraclePriceFeed {
    fn get_price_of_type(
        &self,
        price_type: OraclePriceType,
        bias: Option<PriceBias>,
    ) -> MarginfiResult<I80F48> {
        let price = match price_type {
            OraclePriceType::TimeWeighted => self.get_ema_price()?,
            OraclePriceType::RealTime => self.get_unweighted_price()?,
        };

        match bias {
            None => Ok(price),
            Some(price_bias) => {
                let confidence_interval = self
                    .get_confidence_interval(matches!(price_type, OraclePriceType::TimeWeighted))?;

                match price_bias {
                    PriceBias::Low => Ok(price
                        .checked_sub(confidence_interval)
                        .ok_or_else(math_error!())?),
                    PriceBias::High => Ok(price
                        .checked_add(confidence_interval)
                        .ok_or_else(math_error!())?),
                }
            }
        }
    }
}

/// A slimmed down version of the PullFeedAccountData struct copied from the
/// switchboard-on-demand/src/pull_feed.rs
#[cfg_attr(feature = "client", derive(Clone, Debug))]
pub struct LitePullFeedAccountData {
    pub result: CurrentResult,
    #[cfg(feature = "client")]
    pub feed_hash: [u8; 32],
    #[cfg(feature = "client")]
    pub last_update_timestamp: i64,
}

impl From<&PullFeedAccountData> for LitePullFeedAccountData {
    fn from(feed: &PullFeedAccountData) -> Self {
        Self {
            result: feed.result,
            #[cfg(feature = "client")]
            feed_hash: feed.feed_hash,
            #[cfg(feature = "client")]
            last_update_timestamp: feed.last_update_timestamp,
        }
    }
}

impl From<Ref<'_, PullFeedAccountData>> for LitePullFeedAccountData {
    fn from(feed: Ref<'_, PullFeedAccountData>) -> Self {
        Self {
            result: feed.result,
            #[cfg(feature = "client")]
            feed_hash: feed.feed_hash,
            #[cfg(feature = "client")]
            last_update_timestamp: feed.last_update_timestamp,
        }
    }
}

/// A slimmed down version of the AggregatorAccountData struct copied from the switchboard-v2/src/aggregator.rs
#[cfg_attr(feature = "client", derive(Clone, Debug))]
struct LiteAggregatorAccountData {
    /// Use sliding windoe or round based resolution
    /// NOTE: This changes result propagation in latest_round_result
    pub resolution_mode: AggregatorResolutionMode,
    /// Latest confirmed update request result that has been accepted as valid.
    pub latest_confirmed_round_result: SwitchboardDecimal,
    pub latest_confirmed_round_num_success: u32,
    pub latest_confirmed_round_std_deviation: SwitchboardDecimal,
    /// Minimum number of oracle responses required before a round is validated.
    pub min_oracle_results: u32,
}

impl From<&AggregatorAccountData> for LiteAggregatorAccountData {
    fn from(agg: &AggregatorAccountData) -> Self {
        Self {
            resolution_mode: agg.resolution_mode,
            latest_confirmed_round_result: agg.latest_confirmed_round.result,
            latest_confirmed_round_num_success: agg.latest_confirmed_round.num_success,
            latest_confirmed_round_std_deviation: agg.latest_confirmed_round.std_deviation,
            min_oracle_results: agg.min_oracle_results,
        }
    }
}

impl LiteAggregatorAccountData {
    /// If sufficient oracle responses, returns the latest on-chain result in SwitchboardDecimal format
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use switchboard_v2::AggregatorAccountData;
    /// use std::convert::TryInto;
    ///
    /// let feed_result = AggregatorAccountData::new(feed_account_info)?.get_result()?;
    /// let decimal: f64 = feed_result.try_into()?;
    /// ```

    pub fn get_result(&self) -> anchor_lang::Result<SwitchboardDecimal> {
        if self.resolution_mode == AggregatorResolutionMode::ModeSlidingResolution {
            return Ok(self.latest_confirmed_round_result);
        }
        let min_oracle_results = self.min_oracle_results;
        let latest_confirmed_round_num_success = self.latest_confirmed_round_num_success;
        if min_oracle_results > latest_confirmed_round_num_success {
            return Err(MarginfiError::InvalidOracleAccount.into());
        }
        Ok(self.latest_confirmed_round_result)
    }
}

#[inline(always)]
fn pyth_price_components_to_i80f48(price: I80F48, exponent: i32) -> MarginfiResult<I80F48> {
    let scaling_factor = EXP_10_I80F48[exponent.unsigned_abs() as usize];

    let price = if exponent == 0 {
        price
    } else if exponent < 0 {
        price
            .checked_div(scaling_factor)
            .ok_or_else(math_error!())?
    } else {
        price
            .checked_mul(scaling_factor)
            .ok_or_else(math_error!())?
    };

    Ok(price)
}

/// Load and validate a pyth price feed account.
fn load_pyth_price_feed(ai: &AccountInfo) -> MarginfiResult<PriceFeed> {
    check!(ai.owner.eq(&PYTH_ID), MarginfiError::InvalidOracleAccount);
    let price_feed = SolanaPriceAccount::account_info_to_feed(ai)
        .map_err(|_| MarginfiError::InvalidOracleAccount)?;
    Ok(price_feed)
}

#[inline(always)]
fn switchboard_decimal_to_i80f48(decimal: SwitchboardDecimal) -> Option<I80F48> {
    let decimal = fit_scale_switchboard_decimal(decimal, MAX_SCALE)?;

    I80F48::from_num(decimal.mantissa).checked_div(EXP_10_I80F48[decimal.scale as usize])
}

const MAX_SCALE: u32 = 20;

/// Scale a SwitchboardDecimal down to a given scale.
/// Return original SwitchboardDecimal if it is already at or below the given scale.
///
/// This may result in minimal loss of precision past the scale delta.
#[inline]
fn fit_scale_switchboard_decimal(
    decimal: SwitchboardDecimal,
    scale: u32,
) -> Option<SwitchboardDecimal> {
    if decimal.scale <= scale {
        return Some(decimal);
    }

    let scale_diff = decimal.scale - scale;
    let mantissa = decimal.mantissa.checked_div(EXP_10[scale_diff as usize])?;

    Some(SwitchboardDecimal { mantissa, scale })
}

#[cfg(test)]
mod tests {
    use fixed_macro::types::I80F48;
    use pretty_assertions::assert_eq;
    use rust_decimal::Decimal;

    use crate::utils::hex_to_bytes;

    use super::*;
    #[test]
    fn swb_decimal_test_18() {
        let decimal = SwitchboardDecimal {
            mantissa: 1000000000000000000,
            scale: 18,
        };
        let i80f48 = switchboard_decimal_to_i80f48(decimal).unwrap();
        assert_eq!(i80f48, I80F48::from_num(1));
    }

    #[test]
    /// Testing the standard deviation of the switchboard oracle on the SOLUSD mainnet feed
    fn swb_dec_test_28() {
        let dec = SwitchboardDecimal {
            mantissa: 13942937500000000000000000,
            scale: 28,
        };

        {
            let decimal: Decimal = dec.try_into().unwrap();
            println!("control check: {:?}", decimal);
        }

        let i80f48 = switchboard_decimal_to_i80f48(dec).unwrap();

        assert_eq!(i80f48, I80F48::from_num(0.00139429375));
    }

    #[test]
    fn pyth_conf_interval_cap() {
        // Define a price with a 10% confidence interval
        let high_confidence_price = Box::new(Price {
            price: 100i64 * EXP_10[6] as i64,
            conf: 10u64 * EXP_10[6] as u64,
            expo: -6,
            publish_time: 0,
        });

        // Define a price with a 1% confidence interval
        let low_confidence_price = Box::new(Price {
            price: 100i64 * EXP_10[6] as i64,
            conf: EXP_10[6] as u64,
            expo: -6,
            publish_time: 0,
        });

        // Initialize PythEmaPriceFeed with high confidence price as EMA
        let pyth_adapter = PythLegacyPriceFeed {
            ema_price: high_confidence_price,
            price: low_confidence_price,
        };

        // Test confidence interval when using EMA price (high confidence)
        let high_conf_interval = pyth_adapter.get_confidence_interval(true).unwrap();
        // The confidence interval should be capped at 5%
        assert_eq!(high_conf_interval, I80F48!(5.00000000000007));

        // Test confidence interval when not using EMA price (low confidence)
        let low_conf_interval = pyth_adapter.get_confidence_interval(false).unwrap();
        // The confidence interval should be the calculated value (2.12%)
        assert_eq!(low_conf_interval, I80F48!(2.12));
    }

    #[test]
    fn switchboard_conf_interval_cap() {
        // Define a price with a 10% confidence interval
        // Initialize SwitchboardV2PriceFeed with high confidence price
        let swb_adapter_high_confidence = SwitchboardV2PriceFeed {
            aggregator_account: Box::new(LiteAggregatorAccountData {
                resolution_mode: AggregatorResolutionMode::ModeSlidingResolution,
                latest_confirmed_round_result: SwitchboardDecimal::from_f64(100.0),
                latest_confirmed_round_num_success: 1,
                latest_confirmed_round_std_deviation: SwitchboardDecimal::from_f64(10.0),
                min_oracle_results: 1,
            }),
        };

        let swb_adapter_low_confidence = SwitchboardV2PriceFeed {
            aggregator_account: Box::new(LiteAggregatorAccountData {
                resolution_mode: AggregatorResolutionMode::ModeSlidingResolution,
                latest_confirmed_round_result: SwitchboardDecimal::from_f64(100.0),
                latest_confirmed_round_num_success: 1,
                latest_confirmed_round_std_deviation: SwitchboardDecimal::from_f64(1.0),
                min_oracle_results: 1,
            }),
        };

        // Test confidence interval
        let high_conf_interval = swb_adapter_high_confidence
            .get_confidence_interval()
            .unwrap();
        // The confidence interval should be capped at 5%
        assert_eq!(high_conf_interval, I80F48!(5.00000000000007));

        let low_conf_interval = swb_adapter_low_confidence
            .get_confidence_interval()
            .unwrap();

        // The confidence interval should be the calculated value (1.96%)

        assert_eq!(low_conf_interval, I80F48!(1.96));
    }

    #[test]
    fn pyth_and_pyth_push_cmp() {
        fn get_prices(
            price: i64,
            conf: u64,
        ) -> (Price, pyth_solana_receiver_sdk::price_update::Price) {
            let legacy_price = Price {
                price,
                conf,
                expo: -6,
                publish_time: 0,
            };

            let push_price = pyth_solana_receiver_sdk::price_update::Price {
                price,
                conf,
                exponent: -6,
                publish_time: 0,
            };

            assert_eq!(legacy_price.price, push_price.price);
            assert_eq!(legacy_price.conf, push_price.conf);
            assert_eq!(legacy_price.expo, push_price.exponent);
            assert_eq!(legacy_price.publish_time, push_price.publish_time);

            (legacy_price, push_price)
        }

        let (legacy_price, push_price) =
            get_prices(100i64 * EXP_10[6] as i64, 10u64 * EXP_10[6] as u64);

        let (legacy_ema, push_price_ema) =
            get_prices(99i64 * EXP_10[6] as i64, 4u64 * EXP_10[6] as u64);

        let pyth_legacy = PythLegacyPriceFeed {
            ema_price: Box::new(legacy_ema),
            price: Box::new(legacy_price),
        };

        let pyth_push = PythPushOraclePriceFeed {
            ema_price: Box::new(push_price_ema),
            price: Box::new(push_price),
        };

        assert_eq!(
            pyth_legacy.get_ema_price().unwrap(),
            pyth_push.get_ema_price().unwrap()
        );
        assert_eq!(
            pyth_legacy.get_unweighted_price().unwrap(),
            pyth_push.get_unweighted_price().unwrap()
        );

        assert_eq!(
            pyth_legacy.get_confidence_interval(true).unwrap(),
            pyth_push.get_confidence_interval(true).unwrap()
        );

        assert_eq!(
            pyth_legacy.get_confidence_interval(false).unwrap(),
            pyth_push.get_confidence_interval(false).unwrap()
        );

        assert_eq!(
            pyth_legacy
                .get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::Low))
                .unwrap(),
            pyth_push
                .get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::Low))
                .unwrap()
        );

        // Test high bias ema
        assert_eq!(
            pyth_legacy
                .get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::High))
                .unwrap(),
            pyth_push
                .get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::High))
                .unwrap()
        );

        // Test low bias ema
        assert_eq!(
            pyth_legacy
                .get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::Low))
                .unwrap(),
            pyth_push
                .get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::Low))
                .unwrap()
        );

        // Test no bias real time
        assert_eq!(
            pyth_legacy
                .get_price_of_type(OraclePriceType::RealTime, None)
                .unwrap(),
            pyth_push
                .get_price_of_type(OraclePriceType::RealTime, None)
                .unwrap()
        );

        // new pricees with very wide confidence
        let (legacy_price, push_price) =
            get_prices(100i64 * EXP_10[6] as i64, 100u64 * EXP_10[6] as u64);

        let (legacy_ema, push_price_ema) =
            get_prices(99i64 * EXP_10[6] as i64, 88u64 * EXP_10[6] as u64);

        let pyth_legacy = PythLegacyPriceFeed {
            ema_price: Box::new(legacy_ema),
            price: Box::new(legacy_price),
        };

        let pyth_push = PythPushOraclePriceFeed {
            ema_price: Box::new(push_price_ema),
            price: Box::new(push_price),
        };

        // Test high bias ema
        assert_eq!(
            pyth_legacy.get_ema_price().unwrap(),
            pyth_push.get_ema_price().unwrap()
        );
        assert_eq!(
            pyth_legacy.get_unweighted_price().unwrap(),
            pyth_push.get_unweighted_price().unwrap()
        );

        assert_eq!(
            pyth_legacy.get_confidence_interval(true).unwrap(),
            pyth_push.get_confidence_interval(true).unwrap()
        );

        assert_eq!(
            pyth_legacy.get_confidence_interval(false).unwrap(),
            pyth_push.get_confidence_interval(false).unwrap()
        );

        assert_eq!(
            pyth_legacy
                .get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::Low))
                .unwrap(),
            pyth_push
                .get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::Low))
                .unwrap()
        );

        // Test high bias ema
        assert_eq!(
            pyth_legacy
                .get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::High))
                .unwrap(),
            pyth_push
                .get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::High))
                .unwrap()
        );

        // Test low bias ema
        assert_eq!(
            pyth_legacy
                .get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::Low))
                .unwrap(),
            pyth_push
                .get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::Low))
                .unwrap()
        );

        // Test no bias real time
        assert_eq!(
            pyth_legacy
                .get_price_of_type(OraclePriceType::RealTime, None)
                .unwrap(),
            pyth_push
                .get_price_of_type(OraclePriceType::RealTime, None)
                .unwrap()
        );
    }

    use solana_sdk::account::Account;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Convert an account to info, useful if you only care about data for testing purposes.
    pub fn account_to_account_info<'a>(
        account: &'a mut Account,
        key: &'a Pubkey,
    ) -> AccountInfo<'a> {
        AccountInfo {
            key,
            lamports: Rc::new(RefCell::new(&mut account.lamports)),
            data: Rc::new(RefCell::new(&mut account.data[..])),
            owner: &account.owner,
            rent_epoch: account.rent_epoch,
            is_signer: false,
            is_writable: true,
            executable: account.executable,
        }
    }

    pub fn create_switch_pull_oracle_account_from_bytes(data: Vec<u8>) -> Account {
        Account {
            lamports: 1_000_000,
            data,
            owner: SWITCHBOARD_PULL_ID,
            executable: false,
            rent_epoch: 361,
        }
    }

    #[test]
    fn swb_pull_get_price() {
        // From mainnet: https://solana.fm/address/BSzfJs4d1tAkSDqkepnfzEVcx2WtDVnwwXa2giy9PLeP
        // Actual price $155.59404527
        // conf/Std_dev ~5%
        let bytes = hex_to_bytes("c41b6cc40ad7db286f5e7566ac000a9530e56b1db49585772719aeaaeeadb4d9bd8c2357b88e9e782e53d81000000000000000000000000000985f538057856308000000000000005cba953f3f15356b17703e554d3983801916531d7976aa424ad64348ec50e4224650d81000000000000000000000000000a0d5a780cc7f580800000000000000a20b742cedab55efd1faf60aef2cb872a092d24dfba8a48c8b953a5e90ac7bbf874ed81000000000000000000000000000c04958360093580800000000000000e7ef024ea756f8beec2eaa40234070da356754a8eeb2ac6a17c32d17c3e99f8ddc50d81000000000000000000000000000bc8739b45d215b0800000000000000e3e5130902c3e9c27917789769f1ae05de15cf504658beafeed2c598a949b3b7bf53d810000000000000000000000000007cec168c94d667080000000000000020e270b743473d87eff321663e267ba1c9a151f7969cef8147f625e9a2af7287ea54d81000000000000000000000000000dc65eccc174d6f0800000000000000ab605484238ac93f225c65f24d7705bb74b00cdb576555c3995e196691a4de5f484ed8100000000000000000000000000088f28dc9271d59080000000000000015196392573dc9043242716f629d4c0fb93bc0cff7a1a10ede24281b0e98fb7d5454d810000000000000000000000000000441a10ca4a268080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000048ac38271f28ab1b12e49439bddf54871094e4832a56c7a8ec57bd18d357980086807068432f186a147cf0b13a30067d386204ea9d6c8b04743ac2ef010b07524c935636f2523f6aeeb6dc7b7dab0e86a13ff2c794f7895fc78851d69fdb593bdccdb36600000000000000000000000000e40b540200000001000000534f4c2f55534400000000000000000000000000000000000000000000000000000000019e9eb66600000000fca3d11000000000000000000000000000000000000000000000000000000000000000000000000000dc65eccc174d6f0800000000000000006c9225e039550300000000000000000070d3c6ecddf76b080000000000000000d8244bc073aa060000000000000000000441a10ca4a268080000000000000000dc65eccc174d6f08000000000000000200000000000000ea54d810000000005454d81000000000ea54d81000000000fa0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
        let mut acc = create_switch_pull_oracle_account_from_bytes(bytes);
        let key = pubkey!("BSzfJs4d1tAkSDqkepnfzEVcx2WtDVnwwXa2giy9PLeP");
        let ai = account_to_account_info(&mut acc, &key);
        let ai_check = SwitchboardPullPriceFeed::check_ais(&ai);
        assert!(ai_check.is_ok());

        let current_timestamp = 42;
        let max_age = 100;
        let feed: SwitchboardPullPriceFeed =
            SwitchboardPullPriceFeed::load_checked(&ai, current_timestamp, max_age).unwrap();
        let price: I80F48 = feed.get_price().unwrap();
        let conf: I80F48 = feed.get_confidence_interval().unwrap();

        //println!("price: {:?}, conf: {:?}", price, conf);

        let target_price: I80F48 = I80F48::from_num(155); // Target price is $155
        let price_tolerance: I80F48 = target_price * I80F48::from_num(0.01);

        let target_conf: I80F48 = target_price * I80F48::from_num(0.05);
        let conf_tolerance: I80F48 = target_conf * I80F48::from_num(0.005);

        let min_price: I80F48 = target_price.checked_sub(price_tolerance).unwrap();
        let max_price: I80F48 = target_price.checked_add(price_tolerance).unwrap();
        assert!(price >= min_price && price <= max_price);

        let min_conf: I80F48 = target_conf.checked_sub(conf_tolerance).unwrap();
        let max_conf: I80F48 = target_conf.checked_add(conf_tolerance).unwrap();
        assert!(conf >= min_conf && conf <= max_conf);

        let price_bias_none: I80F48 = feed
            .get_price_of_type(OraclePriceType::RealTime, None)
            .unwrap();
        assert_eq!(price, price_bias_none);

        let price_bias_low: I80F48 = feed
            .get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::Low))
            .unwrap();
        let target_price_low: I80F48 = target_price.checked_sub(target_conf).unwrap();
        let min_price: I80F48 = target_price_low.checked_sub(price_tolerance).unwrap();
        let max_price: I80F48 = target_price_low.checked_add(price_tolerance).unwrap();
        assert!(price_bias_low >= min_price && price_bias_low <= max_price);

        let price_bias_high: I80F48 = feed
            .get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::High))
            .unwrap();
        let target_price_high: I80F48 = target_price.checked_add(target_conf).unwrap();
        let min_price: I80F48 = target_price_high.checked_sub(price_tolerance).unwrap();
        let max_price: I80F48 = target_price_high.checked_add(price_tolerance).unwrap();
        assert!(price_bias_high >= min_price && price_bias_high <= max_price);
    }
}
