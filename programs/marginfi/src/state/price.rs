use crate::{
    check, check_eq,
    constants::{
        CONF_INTERVAL_MULTIPLE, EXP_10_I80F48, MAX_CONF_INTERVAL, MIN_PYTH_PUSH_VERIFICATION_LEVEL,
        NATIVE_STAKE_ID, PYTH_ID, SPL_SINGLE_POOL_ID, STD_DEV_MULTIPLE, SWITCHBOARD_PULL_ID,
    },
    debug, live, math_error,
    prelude::*,
};
use anchor_lang::prelude::borsh;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{borsh1::try_from_slice_unchecked, stake::state::StakeStateV2};
use anchor_spl::token::Mint;
use bytemuck::{Pod, Zeroable};
use enum_dispatch::enum_dispatch;
use fixed::types::I80F48;
use pyth_solana_receiver_sdk::price_update::{self, FeedId, PriceUpdateV2};
use pyth_solana_receiver_sdk::PYTH_PUSH_ORACLE_ID;
use std::{cell::Ref, cmp::min};
use switchboard_on_demand::{
    CurrentResult, Discriminator, PullFeedAccountData, SPL_TOKEN_PROGRAM_ID,
};

use super::marginfi_group::BankConfig;

#[repr(u8)]
#[derive(Copy, Clone, Debug, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub enum OracleSetup {
    None,
    PythLegacy,
    SwitchboardV2,
    PythPushOracle,
    SwitchboardPull,
    StakedWithPythPush,
}
unsafe impl Zeroable for OracleSetup {}
unsafe impl Pod for OracleSetup {}

impl OracleSetup {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::PythLegacy),    // Deprecated
            2 => Some(Self::SwitchboardV2), // Deprecated
            3 => Some(Self::PythPushOracle),
            4 => Some(Self::SwitchboardPull),
            5 => Some(Self::StakedWithPythPush),
            _ => None,
        }
    }
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
    PythPushOracle(PythPushOraclePriceFeed),
    SwitchboardPull(SwitchboardPullPriceFeed),
}

#[cfg_attr(feature = "client", derive(Clone, Debug))]
pub struct SwitchboardV2PriceFeed {
    _ema_price: Box<Price>,
    _price: Box<Price>,
}

impl PriceAdapter for SwitchboardV2PriceFeed {
    fn get_price_of_type(
        &self,
        _price_type: OraclePriceType,
        _bias: Option<PriceBias>,
    ) -> MarginfiResult<I80F48> {
        panic!("swb v2 is deprecated");
    }
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
                panic!("pyth legacy is deprecated");
            }
            OracleSetup::SwitchboardV2 => {
                panic!("swb v2 is deprecated");
            }
            OracleSetup::PythPushOracle => {
                check!(ais.len() == 1, MarginfiError::WrongNumberOfOracleAccounts);

                let account_info = &ais[0];

                if live!() {
                    check_eq!(
                        *account_info.owner,
                        pyth_solana_receiver_sdk::id(),
                        MarginfiError::PythPushWrongAccountOwner
                    );
                } else {
                    // On localnet, allow the mock program ID -OR- the real one
                    let owner_ok = account_info.owner.eq(&PYTH_ID)
                        || account_info.owner.eq(&pyth_solana_receiver_sdk::id());
                    check!(owner_ok, MarginfiError::PythPushWrongAccountOwner);
                }

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
                check!(ais.len() == 1, MarginfiError::WrongNumberOfOracleAccounts);
                if ais[0].key != &bank_config.oracle_keys[0] {
                    msg!(
                        "Expected oracle key: {:?}, got: {:?}",
                        bank_config.oracle_keys[0],
                        ais[0].key
                    );
                    return Err(error!(MarginfiError::WrongOracleAccountKeys));
                }

                Ok(OraclePriceFeedAdapter::SwitchboardPull(
                    SwitchboardPullPriceFeed::load_checked(&ais[0], clock.unix_timestamp, max_age)?,
                ))
            }
            OracleSetup::StakedWithPythPush => {
                check!(ais.len() == 3, MarginfiError::WrongNumberOfOracleAccounts);

                if ais[1].key != &bank_config.oracle_keys[1]
                    || ais[2].key != &bank_config.oracle_keys[2]
                {
                    msg!(
                        "Expected oracle keys: [1] {:?}, [2] {:?}, got: [1] {:?}, [2] {:?}",
                        bank_config.oracle_keys[1],
                        bank_config.oracle_keys[2],
                        ais[1].key,
                        ais[2].key
                    );
                    return Err(error!(MarginfiError::WrongOracleAccountKeys));
                }

                let lst_mint = Account::<'info, Mint>::try_from(&ais[1]).unwrap();
                let lst_supply = lst_mint.supply;
                let stake_state = try_from_slice_unchecked::<StakeStateV2>(&ais[2].data.borrow())?;
                let (_, stake) = match stake_state {
                    StakeStateV2::Stake(meta, stake, _) => (meta, stake),
                    _ => panic!("unsupported stake state"), // TODO emit more specific error
                };
                let sol_pool_balance = stake.delegation.stake;
                // Note: When the pool is fresh, it has 1 SOL in it (an initial and non-refundable
                // balance that will stay in the pool forever). We don't want to include that
                // balance when reading the quantity of SOL that has been staked from actual
                // depositors (i.e. the amount that can actually be redeemed again).
                let lamports_per_sol: u64 = 1_000_000_000;
                let sol_pool_adjusted_balance = sol_pool_balance
                    .checked_sub(lamports_per_sol)
                    .ok_or_else(math_error!())?;
                // Note: exchange rate is `sol_pool_balance / lst_supply`, but we will do the
                // division last to avoid precision loss. Division does not need to be
                // decimal-adjusted because both SOL and stake positions use 9 decimals

                let account_info = &ais[0];

                if live!() {
                    check_eq!(
                        account_info.owner,
                        &pyth_solana_receiver_sdk::id(),
                        MarginfiError::StakedPythPushWrongAccountOwner
                    );
                } else {
                    // On localnet, allow the mock program ID OR the real one (for regression tests against
                    // actual mainnet accounts).
                    // * Note: Typically price updates are owned by `pyth_solana_receiver_sdk` and the oracle
                    // feed account itself is owned by PYTH ID. On localnet, the mock program may own both for
                    // simplicity.
                    let owner_ok = account_info.owner.eq(&PYTH_ID)
                        || account_info.owner.eq(&pyth_solana_receiver_sdk::id());
                    check!(owner_ok, MarginfiError::StakedPythPushWrongAccountOwner);
                }

                let price_feed_id = bank_config.get_pyth_push_oracle_feed_id().unwrap();
                let mut feed = PythPushOraclePriceFeed::load_checked(
                    account_info,
                    price_feed_id,
                    clock,
                    max_age,
                )?;
                let adjusted_price = (feed.price.price as i128)
                    .checked_mul(sol_pool_adjusted_balance as i128)
                    .ok_or_else(math_error!())?
                    .checked_div(lst_supply as i128)
                    .ok_or_else(math_error!())?;
                feed.price.price = adjusted_price.try_into().unwrap();

                let adjusted_ema_price = (feed.ema_price.price as i128)
                    .checked_mul(sol_pool_adjusted_balance as i128)
                    .ok_or_else(math_error!())?
                    .checked_div(lst_supply as i128)
                    .ok_or_else(math_error!())?;
                feed.ema_price.price = adjusted_ema_price.try_into().unwrap();

                let price = OraclePriceFeedAdapter::PythPushOracle(feed);
                Ok(price)
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
                panic!("pyth legacy is deprecated");
            }
            OracleSetup::SwitchboardV2 => {
                panic!("swb v2 is deprecated");
            }
            OracleSetup::PythPushOracle => {
                check!(
                    oracle_ais.len() == 1,
                    MarginfiError::WrongNumberOfOracleAccounts
                );

                PythPushOraclePriceFeed::check_ai_and_feed_id(
                    &oracle_ais[0],
                    bank_config.get_pyth_push_oracle_feed_id().unwrap(),
                )?;

                Ok(())
            }
            OracleSetup::SwitchboardPull => {
                check!(
                    oracle_ais.len() == 1,
                    MarginfiError::WrongNumberOfOracleAccounts
                );
                if oracle_ais[0].key != &bank_config.oracle_keys[0] {
                    msg!(
                        "Expected oracle key: {:?}, got: {:?}",
                        bank_config.oracle_keys[0],
                        oracle_ais[0].key
                    );
                    return Err(error!(MarginfiError::WrongOracleAccountKeys));
                }

                SwitchboardPullPriceFeed::check_ais(&oracle_ais[0])?;

                Ok(())
            }
            OracleSetup::StakedWithPythPush => {
                if lst_mint.is_some() && stake_pool.is_some() && sol_pool.is_some() {
                    check!(
                        oracle_ais.len() == 3,
                        MarginfiError::WrongNumberOfOracleAccounts
                    );

                    PythPushOraclePriceFeed::check_ai_and_feed_id(
                        &oracle_ais[0],
                        bank_config.get_pyth_push_oracle_feed_id().unwrap(),
                    )?;

                    let lst_mint = lst_mint.unwrap();
                    let stake_pool = stake_pool.unwrap();
                    let sol_pool = sol_pool.unwrap();

                    let program_id = &SPL_SINGLE_POOL_ID;
                    let stake_pool_bytes = &stake_pool.to_bytes();
                    // Validate the given stake_pool derives the same lst_mint, proving stake_pool is correct
                    let (exp_mint, _) =
                        Pubkey::find_program_address(&[b"mint", stake_pool_bytes], program_id);
                    check_eq!(exp_mint, lst_mint, MarginfiError::StakePoolValidationFailed);
                    // Validate the now-proven stake_pool derives the given sol_pool
                    let (exp_pool, _) =
                        Pubkey::find_program_address(&[b"stake", stake_pool_bytes], program_id);
                    check_eq!(exp_pool, sol_pool, MarginfiError::StakePoolValidationFailed);

                    // Sanity check the mint. Note: spl-single-pool uses a classic Token, never Token22
                    check!(
                        oracle_ais[1].owner == &SPL_TOKEN_PROGRAM_ID,
                        MarginfiError::StakePoolValidationFailed
                    );
                    check_eq!(
                        oracle_ais[1].key(),
                        lst_mint,
                        MarginfiError::StakePoolValidationFailed
                    );
                    // Sanity check the pool is a native stake pool. Note: the native staking program is
                    // written in vanilla Solana and has no Anchor discriminator.
                    check!(
                        oracle_ais[2].owner == &NATIVE_STAKE_ID,
                        MarginfiError::StakePoolValidationFailed
                    );
                    check_eq!(
                        oracle_ais[2].key(),
                        sol_pool,
                        MarginfiError::StakePoolValidationFailed
                    );

                    Ok(())
                } else {
                    // light validation (after initial setup, only the Pyth oracle needs to be validated)
                    check!(
                        oracle_ais.len() == 1,
                        MarginfiError::WrongNumberOfOracleAccounts
                    );
                    PythPushOraclePriceFeed::check_ai_and_feed_id(
                        &oracle_ais[0],
                        bank_config.get_pyth_push_oracle_feed_id().unwrap(),
                    )?;

                    Ok(())
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
            MarginfiError::SwitchboardWrongAccountOwner
        );

        let feed: PullFeedAccountData = parse_swb_ignore_alignment(ai_data)?;
        let lite_feed = LitePullFeedAccountData::from(&feed);
        // TODO restore when swb fixes alignment issue in crate.
        // let feed = PullFeedAccountData::parse(ai_data)
        //     .map_err(|_| MarginfiError::SwitchboardInvalidAccount)?;

        // Check staleness
        let last_updated = feed.last_update_timestamp;
        if current_timestamp.saturating_sub(last_updated) > max_age as i64 {
            return err!(MarginfiError::SwitchboardStalePrice);
        }

        Ok(Self {
            feed: Box::new(lite_feed),
        })
    }

    fn check_ais(ai: &AccountInfo) -> MarginfiResult {
        let ai_data = ai.data.borrow();

        check!(
            ai.owner.eq(&SWITCHBOARD_PULL_ID),
            MarginfiError::SwitchboardWrongAccountOwner
        );

        let _feed = parse_swb_ignore_alignment(ai_data)?;
        // TODO restore when swb fixes alignment issue in crate.
        // PullFeedAccountData::parse(ai_data)
        //     .map_err(|_| MarginfiError::SwitchboardInvalidAccount)?;

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

// TODO remove when swb fixes the alignment issue in their crate
// (TargetAlignmentGreaterAndInputNotAligned) when bytemuck::from_bytes executes on any local system
// (including bpf next-test) where the struct is "properly" aligned 16
/// The same as PullFeedAccountData::parse but completely ignores input alignment.
pub fn parse_swb_ignore_alignment(data: Ref<&mut [u8]>) -> MarginfiResult<PullFeedAccountData> {
    if data.len() < 8 {
        return err!(MarginfiError::SwitchboardInvalidAccount);
    }

    if data[..8] != PullFeedAccountData::DISCRIMINATOR {
        return err!(MarginfiError::SwitchboardInvalidAccount);
    }

    let feed = bytemuck::try_pod_read_unaligned::<PullFeedAccountData>(
        &data[8..8 + std::mem::size_of::<PullFeedAccountData>()],
    )
    .map_err(|_| MarginfiError::SwitchboardInvalidAccount)?;

    Ok(feed)
}

pub fn load_price_update_v2_checked(ai: &AccountInfo) -> MarginfiResult<PriceUpdateV2> {
    if live!() {
        check_eq!(
            *ai.owner,
            pyth_solana_receiver_sdk::id(),
            MarginfiError::PythPushWrongAccountOwner
        );
    } else {
        // On localnet, allow the mock program ID OR the real one (for regression tests against
        // actual mainnet accounts).
        // * Note: Typically price updates are owned by `pyth_solana_receiver_sdk` and the oracle
        // feed account itself is owned by PYTH ID. On localnet, the mock program may own both for
        // simplicity.
        let owner_ok = ai.owner.eq(&PYTH_ID) || ai.owner.eq(&pyth_solana_receiver_sdk::id());
        check!(owner_ok, MarginfiError::PythPushWrongAccountOwner);
    }

    let price_feed_data = ai.try_borrow_data()?;
    let discriminator = &price_feed_data[0..8];
    let expected_discrim = <PriceUpdateV2 as anchor_lang::Discriminator>::DISCRIMINATOR;

    check_eq!(
        discriminator,
        expected_discrim,
        MarginfiError::PythPushInvalidAccount
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
                let error: MarginfiError = e.into();
                error
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
                let error: MarginfiError = e.into();
                error
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

        check_eq!(
            &price_feed_account.price_message.feed_id,
            feed_id,
            MarginfiError::PythPushMismatchedFeedId
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::constants::EXP_10;
    use crate::utils::hex_to_bytes;

    use super::*;

    use anchor_lang::solana_program::account_info::AccountInfo;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn swb_pull_get_price() {
        // From mainnet: https://solana.fm/address/BSzfJs4d1tAkSDqkepnfzEVcx2WtDVnwwXa2giy9PLeP
        // Actual price $155.59404527
        // conf/Std_dev ~5%
        let bytes = hex_to_bytes("c41b6cc40ad7db286f5e7566ac000a9530e56b1db49585772719aeaaeeadb4d9bd8c2357b88e9e782e53d81000000000000000000000000000985f538057856308000000000000005cba953f3f15356b17703e554d3983801916531d7976aa424ad64348ec50e4224650d81000000000000000000000000000a0d5a780cc7f580800000000000000a20b742cedab55efd1faf60aef2cb872a092d24dfba8a48c8b953a5e90ac7bbf874ed81000000000000000000000000000c04958360093580800000000000000e7ef024ea756f8beec2eaa40234070da356754a8eeb2ac6a17c32d17c3e99f8ddc50d81000000000000000000000000000bc8739b45d215b0800000000000000e3e5130902c3e9c27917789769f1ae05de15cf504658beafeed2c598a949b3b7bf53d810000000000000000000000000007cec168c94d667080000000000000020e270b743473d87eff321663e267ba1c9a151f7969cef8147f625e9a2af7287ea54d81000000000000000000000000000dc65eccc174d6f0800000000000000ab605484238ac93f225c65f24d7705bb74b00cdb576555c3995e196691a4de5f484ed8100000000000000000000000000088f28dc9271d59080000000000000015196392573dc9043242716f629d4c0fb93bc0cff7a1a10ede24281b0e98fb7d5454d810000000000000000000000000000441a10ca4a268080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000048ac38271f28ab1b12e49439bddf54871094e4832a56c7a8ec57bd18d357980086807068432f186a147cf0b13a30067d386204ea9d6c8b04743ac2ef010b07524c935636f2523f6aeeb6dc7b7dab0e86a13ff2c794f7895fc78851d69fdb593bdccdb36600000000000000000000000000e40b540200000001000000534f4c2f55534400000000000000000000000000000000000000000000000000000000019e9eb66600000000fca3d11000000000000000000000000000000000000000000000000000000000000000000000000000dc65eccc174d6f0800000000000000006c9225e039550300000000000000000070d3c6ecddf76b080000000000000000d8244bc073aa060000000000000000000441a10ca4a268080000000000000000dc65eccc174d6f08000000000000000200000000000000ea54d810000000005454d81000000000ea54d81000000000fa0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
        let key = pubkey!("BSzfJs4d1tAkSDqkepnfzEVcx2WtDVnwwXa2giy9PLeP");
        let mut lamports = 1_000_000u64;
        let mut data = bytes.clone();

        let ai = AccountInfo {
            key: &key,
            lamports: Rc::new(RefCell::new(&mut lamports)),
            data: Rc::new(RefCell::new(&mut data[..])),
            owner: &SWITCHBOARD_PULL_ID,
            rent_epoch: 361,
            is_signer: false,
            is_writable: true,
            executable: false,
        };

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
