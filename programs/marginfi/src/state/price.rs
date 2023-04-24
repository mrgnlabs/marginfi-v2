use anchor_lang::prelude::*;

use enum_dispatch::enum_dispatch;
use fixed::types::I80F48;
use pyth_sdk_solana::{load_price_feed_from_account_info, Price, PriceFeed};
use switchboard_v2::{
    AggregatorAccountData, AggregatorResolutionMode, SwitchboardDecimal, SWITCHBOARD_PROGRAM_ID,
};

use crate::{
    check,
    constants::{CONF_INTERVAL_MULTIPLE, EXP_10, EXP_10_I80F48, PYTH_ID},
    math_error,
    prelude::*,
};

use super::marginfi_group::BankConfig;

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
pub enum OraclePriceFeedAdapter {
    PythEma(PythEmaPriceFeed),
    SwitchboardV2(SwitchboardV2PriceFeed),
}

impl OraclePriceFeedAdapter {
    pub fn try_from_bank_config(
        bank_config: &BankConfig,
        ais: &[AccountInfo],
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

                let account_info = &ais[0];

                Ok(OraclePriceFeedAdapter::PythEma(PythEmaPriceFeed::new(
                    account_info,
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

                Ok(OraclePriceFeedAdapter::SwitchboardV2(
                    SwitchboardV2PriceFeed::new(&ais[0], current_timestamp, max_age)?,
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
    pub fn new(ai: &AccountInfo, current_time: i64, max_age: u64) -> MarginfiResult<Self> {
        let price_feed = load_pyth_price_feed(ai)?;
        let price = price_feed
            .get_ema_price_no_older_than(current_time, max_age)
            .ok_or(MarginfiError::StaleOracle)?;

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
    aggregator_account: Box<LiteAggregatorAccountData>,
}

impl SwitchboardV2PriceFeed {
    pub fn new(ai: &AccountInfo, current_timestamp: i64, max_age: u64) -> MarginfiResult<Self> {
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

    fn validate_ais(ai: &AccountInfo) -> MarginfiResult {
        let ai_data = ai.data.borrow();

        check!(
            ai.owner.eq(&SWITCHBOARD_PROGRAM_ID),
            MarginfiError::InvalidOracleAccount
        );

        AggregatorAccountData::new_from_bytes(&ai_data)
            .map_err(|_| MarginfiError::InvalidOracleAccount)?;

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
            .ok_or(MarginfiError::InvalidSwitchboardDecimalConversion)?)
    }

    fn get_confidence_interval(&self) -> MarginfiResult<I80F48> {
        let std_div = self.aggregator_account.latest_confirmed_round_std_deviation;
        let std_div = swithcboard_decimal_to_i80f48(std_div)
            .ok_or(MarginfiError::InvalidSwitchboardDecimalConversion)?;

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

/// A slimmed down version of the AggregatorAccountData struct copied from the switchboard-v2/src/aggregator.rs
struct LiteAggregatorAccountData {
    /// Use sliding windoe or round based resolution
    /// NOTE: This changes result propogation in latest_round_result
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
    let price_feed =
        load_price_feed_from_account_info(ai).map_err(|_| MarginfiError::InvalidOracleAccount)?;
    Ok(price_feed)
}

#[inline(always)]
fn swithcboard_decimal_to_i80f48(decimal: SwitchboardDecimal) -> Option<I80F48> {
    let decimal = fit_scale_switchboard_decimal(decimal, MAX_SCALE)?;

    I80F48::from_num(decimal.mantissa).checked_div(EXP_10_I80F48[decimal.scale as usize])
}

const MAX_SCALE: u32 = 20;

/// Scale a SwitchboardDecimal down to a given scale.
/// Return original SwitchboardDecimal if it is already at or below the given scale.
///
/// This may result in minimal loss of precision past the scale delta.
/// However in practice we've seen that SwitchboardDecimals are significanly overscaled.
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
    use rust_decimal::Decimal;

    use super::*;
    #[test]
    fn swb_decimal_test_18() {
        let decimal = SwitchboardDecimal {
            mantissa: 1000000000000000000,
            scale: 18,
        };
        let i80f48 = swithcboard_decimal_to_i80f48(decimal).unwrap();
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

        let i80f48 = swithcboard_decimal_to_i80f48(dec).unwrap();

        assert_eq!(i80f48, I80F48::from_num(0.00139429375));
    }
}
