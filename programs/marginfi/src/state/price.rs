use anchor_lang::prelude::*;
use enum_dispatch::enum_dispatch;
use fixed::types::I80F48;
use pyth_sdk_solana::{load_price_feed_from_account_info, Price, PriceFeed};
use switchboard_v2::{AggregatorAccountData, SwitchboardDecimal};

use crate::{
    check,
    constants::{CONF_INTERVAL_MULTIPLE, EXP_10_I80F48, PYTH_ID},
    math_error,
    prelude::*,
};

use super::marginfi_group::{BankConfig, BankConfigOpt};

#[repr(u8)]
#[cfg_attr(any(feature = "test", feature = "client"), derive(PartialEq, Eq))]
#[derive(Copy, Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum OracleSetup {
    None,
    PythEma,
    SwitchboardV2,
}

#[enum_dispatch]
pub trait PriceAdapter {
    fn get_price(&self) -> MarginfiResult<I80F48>;
    fn get_confidence_interval(&self) -> MarginfiResult<I80F48>;
    /// Get a normalized price range for the given price feed.
    /// The range is the price +/- the CONF_INTERVAL_MULTIPLE * confidence interval.
    fn get_price_range(&self) -> MarginfiResult<(I80F48, I80F48)>;
}

#[enum_dispatch(PriceAdapter)]
pub enum PriceFeelAdapter {
    PythEma(PythEmaPriceFeed),
    SwitchboardV2(SwitchboardV2PriceFeed),
}

impl PriceFeelAdapter {
    pub fn try_from_bank_config<'a>(
        bank_config: &BankConfig,
        ais: &'a [AccountInfo<'a>],
        current_timestamp: i64,
        max_age: u64,
    ) -> MarginfiResult<Self> {
        match bank_config.oracle_setup {
            OracleSetup::None => Err(MarginfiError::OracleNotSetup.into()),
            OracleSetup::PythEma => {
                check!(ais.len() == 1, MarginfiError::InvalidOracleAccount);
                check!(
                    ais[0].key == &bank_config.oracle_keys[0],
                    MarginfiError::InvalidOracleAccount
                );

                Ok(PriceFeelAdapter::PythEma(PythEmaPriceFeed::new(
                    ais[0],
                    current_timestamp,
                    max_age,
                )?))
            }
            OracleSetup::SwitchboardV2 => {
                check!(ais.len() == 1, MarginfiError::InvalidOracleAccount);
                check!(
                    ais[0].key == &bank_config.oracle_keys[0],
                    MarginfiError::InvalidOracleAccount
                );

                Ok(PriceFeelAdapter::SwitchboardV2(
                    SwitchboardV2PriceFeed::new(&ais[0])?,
                ))
            }
        }
    }
    pub fn validate_bank_config(
        bank_config: &BankConfig,
        oracle_ais: &[AccountInfo],
    ) -> MarginfiResult {
        match bank_config.oracle_setup {
            OracleSetup::None => Err(MarginfiError::OracleNotSetup.into()),
            OracleSetup::PythEma => {
                check!(oracle_ais.len() == 1, MarginfiError::InvalidOracleAccount);
                check!(
                    oracle_ais[0].key == &bank_config.oracle_keys[0],
                    MarginfiError::InvalidOracleAccount
                );

                PythEmaPriceFeed::validate_ais(&oracle_ais[0])?;

                Ok(())
            }
            OracleSetup::SwitchboardV2 => {
                check!(oracle_ais.len() == 1, MarginfiError::InvalidOracleAccount);
                check!(
                    oracle_ais[0].key == &bank_config.oracle_keys[0],
                    MarginfiError::InvalidOracleAccount
                );

                SwitchboardV2PriceFeed::validate_ais(&oracle_ais[0])?;

                Ok(())
            }
        }
    }
}

pub struct PythEmaPriceFeed {
    price: Box<Price>,
}

impl PythEmaPriceFeed {
    pub fn new(ai: AccountInfo, current_time: i64, max_age: u64) -> MarginfiResult<Self> {
        let price_feed = load_pyth_price_feed(&ai)?;
        let price = price_feed
            .get_ema_price_no_older_than(current_time, max_age)
            .ok_or_else(|| MarginfiError::StaleOracle)?;

        Ok(Self {
            price: Box::new(price),
        })
    }

    fn validate_ais(ai: &AccountInfo) -> MarginfiResult {
        load_pyth_price_feed(ai)?;
        Ok(())
    }
}

impl PriceAdapter for PythEmaPriceFeed {
    fn get_price(&self) -> MarginfiResult<I80F48> {
        pyth_price_components_to_i80f48(I80F48::from_num(self.price.price), self.price.expo)
    }

    fn get_confidence_interval(&self) -> MarginfiResult<I80F48> {
        Ok(
            pyth_price_components_to_i80f48(I80F48::from_num(self.price.conf), self.price.expo)?
                .checked_mul(CONF_INTERVAL_MULTIPLE)
                .ok_or_else(math_error!())?,
        )
    }

    fn get_price_range(&self) -> MarginfiResult<(I80F48, I80F48)> {
        let base_price = self.get_price()?;
        let price_range = self.get_confidence_interval()?;

        let lowest_price = base_price
            .checked_sub(price_range)
            .ok_or_else(math_error!())?;
        let highest_price = base_price
            .checked_add(price_range)
            .ok_or_else(math_error!())?;

        Ok((lowest_price, highest_price))
    }
}

pub struct SwitchboardV2PriceFeed {
    aggregator_account: Box<AggregatorAccountData>,
}

impl SwitchboardV2PriceFeed {
    pub fn new(ai: &AccountInfo) -> MarginfiResult<Self> {
        Ok(Self {
            aggregator_account: Box::new(
                *AggregatorAccountData::new(ai).map_err(|_| MarginfiError::InvalidOracleAccount)?,
            ),
        })
    }

    fn validate_ais(ai: &AccountInfo) -> MarginfiResult {
        AggregatorAccountData::new(ai).map_err(|_| MarginfiError::InvalidOracleAccount)?;

        Ok(())
    }
}

impl PriceAdapter for SwitchboardV2PriceFeed {
    fn get_price(&self) -> MarginfiResult<I80F48> {
        let sw_decimal = self
            .aggregator_account
            .get_result()
            .map_err(|_| MarginfiError::InvalidPrice)?;
        Ok(swithcboard_decimal_to_i80f48(sw_decimal)
            .ok_or_else(|| MarginfiError::InvalidSwitchboardDecimalConversion)?)
    }

    fn get_confidence_interval(&self) -> MarginfiResult<I80F48> {
        let std_div = self.aggregator_account.latest_confirmed_round.std_deviation;
        let std_div = swithcboard_decimal_to_i80f48(std_div)
            .ok_or_else(|| MarginfiError::InvalidSwitchboardDecimalConversion)?;

        Ok(std_div
            .checked_mul(CONF_INTERVAL_MULTIPLE)
            .ok_or_else(math_error!())?)
    }

    fn get_price_range(&self) -> MarginfiResult<(I80F48, I80F48)> {
        let base_price = self.get_price()?;
        let price_range = self.get_confidence_interval()?;

        let lowest_price = base_price
            .checked_sub(price_range)
            .ok_or_else(math_error!())?;
        let highest_price = base_price
            .checked_add(price_range)
            .ok_or_else(math_error!())?;

        Ok((lowest_price, highest_price))
    }
}

#[inline(always)]
pub fn pyth_price_components_to_i80f48(price: I80F48, exponent: i32) -> MarginfiResult<I80F48> {
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
    let price_feed =
        load_price_feed_from_account_info(ai).map_err(|_| MarginfiError::InvalidOracleAccount)?;
    Ok(price_feed)
}

#[inline(always)]
fn swithcboard_decimal_to_i80f48(decimal: SwitchboardDecimal) -> Option<I80F48> {
    I80F48::checked_from_num(decimal.mantissa)?.checked_div(EXP_10_I80F48[decimal.scale as usize])
}
