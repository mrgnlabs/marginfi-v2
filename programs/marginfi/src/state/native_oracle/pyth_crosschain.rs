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

#[derive(Clone, Copy, Debug)]
pub struct PythnetPriceFeedControl {
    pub price_feed_id: [u8; 32],
    pub min_verificaiton_level: VerificationLevel,
}

impl PythnetPriceFeedControl {
    /// Check that the feed ID matches the expected feed ID
    pub fn check_feed_id_matches(&self, feed_id: &[u8; 32]) -> MarginfiResult<()> {
        check!(
            self.price_feed_id.eq(feed_id),
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
            "Update is older than current price update: proposed {} vs current {}",
            publish_time,
            self.publish_time,
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

#[cfg(test)]
mod test {
    use std::error::Error;

    use fixed_macro::types::I80F48;
    use solana_sdk::pubkey::Pubkey;

    use crate::constants::EXP_10;

    use super::*;
    const FEED_ID: [u8; 32] = [3; 32];

    #[test]
    fn feed_ctl() {
        let ctl_full_vl = PythnetPriceFeedControl {
            price_feed_id: FEED_ID,
            min_verificaiton_level: VerificationLevel::Full,
        };

        assert!(ctl_full_vl
            .check_sufficient_verificaiton_level(VerificationLevel::Full)
            .is_ok());
        assert!(ctl_full_vl
            .check_sufficient_verificaiton_level(VerificationLevel::Partial { num_signatures: 6 })
            .is_err());

        assert!(ctl_full_vl.check_feed_id_matches(&[0; 32]).is_err());
        assert!(ctl_full_vl.check_feed_id_matches(&[4; 32]).is_err());
        assert!(ctl_full_vl.check_feed_id_matches(&FEED_ID).is_ok());

        let ctl_partial_vl = PythnetPriceFeedControl {
            price_feed_id: FEED_ID,
            min_verificaiton_level: VerificationLevel::Partial { num_signatures: 5 },
        };

        assert!(ctl_partial_vl
            .check_sufficient_verificaiton_level(VerificationLevel::Full)
            .is_ok());
        assert!(ctl_partial_vl
            .check_sufficient_verificaiton_level(VerificationLevel::Partial { num_signatures: 6 })
            .is_ok());
        assert!(ctl_partial_vl
            .check_sufficient_verificaiton_level(VerificationLevel::Partial { num_signatures: 3 })
            .is_err());
    }

    #[test]
    fn feed_updating() -> Result<(), Box<dyn Error>> {
        let pf = PythnetPriceFeed::new_no_staleness_check(
            PriceUpdateV2 {
                write_authority: Pubkey::default(),
                verification_level: VerificationLevel::Partial { num_signatures: 3 },
                price_message: PriceFeedMessage {
                    feed_id: [0; 32],
                    price: 10 * EXP_10[6] as i64,
                    conf: 10 * EXP_10[4] as u64,
                    exponent: -6,
                    publish_time: 1,
                    prev_publish_time: 0,
                    ema_price: 10 * EXP_10[6] as i64,
                    ema_conf: 10 * EXP_10[4] as u64,
                },
                posted_slot: 0,
            },
            VerificationLevel::Full,
        );

        assert!(pf.is_err());

        let pf = PythnetPriceFeed::new_no_staleness_check(
            PriceUpdateV2 {
                write_authority: Pubkey::default(),
                verification_level: VerificationLevel::Full,
                price_message: PriceFeedMessage {
                    feed_id: [0; 32],
                    price: 10 * EXP_10[6] as i64,
                    conf: 10 * EXP_10[4] as u64,
                    exponent: -6,
                    publish_time: 1,
                    prev_publish_time: 0,
                    ema_price: 10 * EXP_10[6] as i64,
                    ema_conf: 10 * EXP_10[4] as u64,
                },
                posted_slot: 0,
            },
            VerificationLevel::Full,
        );

        assert!(pf.is_ok());

        let mut pf = pf.unwrap();

        assert!(pf.check_staleness(10, 5).is_err());
        assert!(pf.check_staleness(5, 6).is_ok());

        let ctl = PythnetPriceFeedControl {
            price_feed_id: FEED_ID,
            min_verificaiton_level: VerificationLevel::Full,
        };

        assert!(
            pf.try_update(
                &PriceUpdateV2 {
                    write_authority: Pubkey::default(),
                    verification_level: VerificationLevel::Partial { num_signatures: 2 },
                    price_message: PriceFeedMessage {
                        feed_id: FEED_ID,
                        price: 10 * EXP_10[6] as i64,
                        conf: 10 * EXP_10[4] as u64,
                        exponent: -6,
                        publish_time: 1,
                        prev_publish_time: 0,
                        ema_price: 10 * EXP_10[6] as i64,
                        ema_conf: 10 * EXP_10[4] as u64,
                    },
                    posted_slot: 0,
                },
                ctl,
            )
            .is_err(),
            "Should fail because of insufficient VerificationLevel"
        );

        assert!(
            pf.try_update(
                &PriceUpdateV2 {
                    write_authority: Pubkey::default(),
                    verification_level: VerificationLevel::Full,
                    price_message: PriceFeedMessage {
                        feed_id: [0; 32],
                        price: 10 * EXP_10[6] as i64,
                        conf: 10 * EXP_10[4] as u64,
                        exponent: -6,
                        publish_time: 1,
                        prev_publish_time: 0,
                        ema_price: 10 * EXP_10[6] as i64,
                        ema_conf: 10 * EXP_10[4] as u64,
                    },
                    posted_slot: 0,
                },
                ctl,
            )
            .is_err(),
            "Should fail because of incorrect feed id"
        );

        assert!(
            pf.try_update(
                &PriceUpdateV2 {
                    write_authority: Pubkey::default(),
                    verification_level: VerificationLevel::Full,
                    price_message: PriceFeedMessage {
                        feed_id: FEED_ID,
                        price: 10 * EXP_10[6] as i64,
                        conf: 10 * EXP_10[4] as u64,
                        exponent: -6,
                        publish_time: 1,
                        prev_publish_time: 0,
                        ema_price: 10 * EXP_10[6] as i64,
                        ema_conf: 10 * EXP_10[4] as u64,
                    },
                    posted_slot: 0,
                },
                ctl,
            )
            .is_err(),
            "Should fail because of update age"
        );

        assert!(
            pf.try_update(
                &PriceUpdateV2 {
                    write_authority: Pubkey::default(),
                    verification_level: VerificationLevel::Full,
                    price_message: PriceFeedMessage {
                        feed_id: FEED_ID,
                        price: 10 * EXP_10[6] as i64,
                        conf: 10 * EXP_10[4] as u64,
                        exponent: -6,
                        publish_time: 2,
                        prev_publish_time: 0,
                        ema_price: 10 * EXP_10[6] as i64,
                        ema_conf: 10 * EXP_10[4] as u64,
                    },
                    posted_slot: 0,
                },
                ctl,
            )
            .is_ok(),
            "Should succedd"
        );

        Ok(())
    }

    #[test]
    fn price_adapter() -> Result<(), Box<dyn Error>> {
        let mut pf = PythnetPriceFeed::new_no_staleness_check(
            PriceUpdateV2 {
                write_authority: Pubkey::default(),
                verification_level: VerificationLevel::Full,
                price_message: PriceFeedMessage {
                    feed_id: FEED_ID,
                    price: 10 * EXP_10[6] as i64,
                    conf: 10 * EXP_10[4] as u64,
                    exponent: -6,
                    publish_time: 1,
                    prev_publish_time: 0,
                    ema_price: 5 * EXP_10[6] as i64,
                    ema_conf: 15 * EXP_10[4] as u64,
                },
                posted_slot: 0,
            },
            VerificationLevel::Full,
        )
        .unwrap();

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::RealTime, None)?,
            I80F48!(10)
        );

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::High))?,
            I80F48!(10.211999999999993)
        );

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::Low))?,
            I80F48!(9.788000000000007)
        );

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::TimeWeighted, None)?,
            I80F48!(5)
        );

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::High))?,
            I80F48!(5.250000000000004)
        );

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::Low))?,
            I80F48!(4.749999999999996)
        );

        let ctl = PythnetPriceFeedControl {
            price_feed_id: FEED_ID,
            min_verificaiton_level: VerificationLevel::Full,
        };

        pf.try_update(
            &PriceUpdateV2 {
                write_authority: Pubkey::default(),
                verification_level: VerificationLevel::Full,
                price_message: PriceFeedMessage {
                    feed_id: FEED_ID,
                    price: 20 * EXP_10[6] as i64,
                    conf: 10 * EXP_10[4] as u64,
                    exponent: -6,
                    publish_time: 2,
                    prev_publish_time: 0,
                    ema_price: 40 * EXP_10[6] as i64,
                    ema_conf: 10 * EXP_10[4] as u64,
                },
                posted_slot: 0,
            },
            ctl,
        )
        .unwrap();

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::RealTime, None)?,
            I80F48!(20)
        );

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::High))?,
            I80F48!(20.211999999999993)
        );

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::RealTime, Some(PriceBias::Low))?,
            I80F48!(19.788000000000007)
        );

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::TimeWeighted, None)?,
            I80F48!(40)
        );

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::High))?,
            I80F48!(40.211999999999993)
        );

        assert_eq!(
            pf.get_price_of_type(OraclePriceType::TimeWeighted, Some(PriceBias::Low))?,
            I80F48!(39.788000000000007)
        );

        Ok(())
    }
}
