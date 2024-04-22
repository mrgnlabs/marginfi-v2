use std::cmp::min;

use fixed::types::I80F48;
use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2, VerificationLevel};
use pythnet_sdk::messages::PriceFeedMessage;

use crate::{
    check,
    constants::{CONF_INTERVAL_MULTIPLE, MAX_CONF_INTERVAL},
    math_error,
    state::price::{pyth_price_components_to_i80f48, OraclePriceType, PriceAdapter, PriceBias},
    MarginfiError, MarginfiResult,
};

#[derive(Debug, Clone)]
pub struct PythnetPriceFeedControl<'a> {
    pub price_feed_id: &'a [u8; 32],
    pub min_verificaiton_level: VerificationLevel,
}

impl PythnetPriceFeedControl<'_> {
    /// Check that the feed ID matches the expected feed ID
    pub fn check_feed_id_matches(&self, feed_id: &[u8; 32]) -> MarginfiResult<()> {
        check!(
            feed_id == self.price_feed_id,
            MarginfiError::IllegalOracleUpdate,
            "Price feed ID mismatch"
        );

        Ok(())
    }

    /// Check that the verification level is sufficient for the oracle update
    pub fn check_sufficient_verificaiton_level(
        &self,
        verification_level: VerificationLevel,
    ) -> MarginfiResult<()> {
        check!(
            verification_level.gte(self.min_verificaiton_level),
            MarginfiError::IllegalOracleUpdate,
            "Insufficient verification level {:?} for oracle update",
            verification_level
        );

        Ok(())
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(
    any(feature = "test", feature = "client"),
    derive(Debug, type_layout::TypeLayout)
)]
pub struct PythnetPriceFeed {
    pub price: i64,
    pub conf: u64,
    pub exponent: i32,
    pub publish_time: i64,
    pub ema_price: i64,
    pub ema_conf: u64,
    pub min_verificaiton_level: VerificationLevel,
}

impl PythnetPriceFeed {
    /// Create a new PythnetPriceFeed from a PriceUpdateV2
    /// Checks:
    /// - Verification level is sufficient
    ///
    /// Does not check for staleness, or feed ID matching
    ///
    /// Staleness is ignored because this is the first time we are setting up the feed,
    /// getting a live feed is not practical.
    ///
    /// Feed ID check is done in the `validate_oracle_setup` call.
    pub fn new_no_staleness_check(
        price_update: PriceUpdateV2,
        min_verificaiton_level: VerificationLevel,
    ) -> MarginfiResult<Self> {
        let PriceFeedMessage {
            price,
            conf,
            exponent,
            publish_time,
            ema_price,
            ema_conf,
            ..
        } = price_update.price_message;

        check!(
            price_update.verification_level.gte(min_verificaiton_level),
            MarginfiError::IllegalOracleUpdate,
            "Insufficient verification level"
        );

        Ok(Self {
            price,
            conf,
            exponent,
            publish_time,
            ema_price,
            ema_conf,
            min_verificaiton_level,
        })
    }

    /// Update the PythnetPriceFeed with a new PriceUpdateV2
    ///
    /// Checks:
    /// - Feed ID matches the expected feed ID
    /// - Verification level is sufficient
    /// - Oracle is not stale
    /// - Update is newer than the current price update
    pub fn try_update(
        &mut self,
        new_price_update: &PriceUpdateV2,
        ctl: PythnetPriceFeedControl,
    ) -> MarginfiResult<()> {
        let PriceFeedMessage {
            ref feed_id,
            price,
            conf,
            exponent,
            publish_time,
            ema_price,
            ema_conf,
            ..
        } = new_price_update.price_message;

        ctl.check_feed_id_matches(feed_id)?;
        ctl.check_sufficient_verificaiton_level(new_price_update.verification_level)?;

        check!(
            publish_time > self.publish_time,
            MarginfiError::IllegalOracleUpdate,
            "Update is older than current price update"
        );

        *self = Self {
            price,
            conf,
            exponent,
            publish_time,
            ema_price,
            ema_conf,
            min_verificaiton_level: self.min_verificaiton_level,
        };

        Ok(())
    }

    pub fn check_staleness(&self, current_time: i64, max_age: u64) -> MarginfiResult<()> {
        check!(
            self.publish_time.saturating_add(max_age as i64) >= current_time,
            MarginfiError::StaleOracle,
            "Oracle is stale"
        );

        Ok(())
    }

    #[inline(always)]
    fn get_ema_price(&self) -> MarginfiResult<I80F48> {
        pyth_price_components_to_i80f48(I80F48::from_num(self.ema_price), self.exponent)
    }

    #[inline(always)]
    fn get_unweighted_price(&self) -> MarginfiResult<I80F48> {
        pyth_price_components_to_i80f48(I80F48::from_num(self.price), self.exponent)
    }

    fn get_confidence_interval(&self, use_ema: bool) -> MarginfiResult<I80F48> {
        let (price, conf) = if use_ema {
            (self.ema_price, self.ema_conf)
        } else {
            (self.price, self.conf)
        };

        let conf_interval = pyth_price_components_to_i80f48(I80F48::from_num(conf), self.exponent)?
            .checked_mul(CONF_INTERVAL_MULTIPLE)
            .ok_or_else(math_error!())?;

        // Cap confidence interval to 5% of price
        let price = pyth_price_components_to_i80f48(I80F48::from_num(price), self.exponent)?;

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

impl PriceAdapter for PythnetPriceFeed {
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
