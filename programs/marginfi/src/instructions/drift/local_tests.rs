#[cfg(test)]
mod tests {
    use bytemuck::Zeroable;
    use drift_mocks::{
        constants::{
            get_precision_increase, DRIFT_PRECISION_EXP, EXP_10, SPOT_CUMULATIVE_INTEREST_PRECISION,
        },
        state::{MinimalSpotMarket, MinimalUser, SpotBalanceType, SpotPosition},
    };

    /// Find the largest u64 raw value that won't overflow when adjusted.
    /// Formula: adjusted = raw * interest / precision
    /// Safe when: raw <= (u64::MAX * precision) / interest
    fn largest_safe_raw_for_u64_exact(market: &MinimalSpotMarket) -> u64 {
        let interest = market.cumulative_deposit_interest;
        if interest == 0 {
            return u64::MAX;
        }
        let max_product = (u64::MAX as u128)
            .checked_mul(SPOT_CUMULATIVE_INTEREST_PRECISION)
            .unwrap_or(u128::MAX);
        let safe = max_product / interest;
        if safe > u64::MAX as u128 {
            u64::MAX
        } else {
            safe as u64
        }
    }

    fn overflow_raw_for_u64_exact(market: &MinimalSpotMarket) -> u64 {
        largest_safe_raw_for_u64_exact(market).saturating_add(1)
    }

    /// Find the largest i64 raw value that won't overflow when adjusted.
    fn largest_safe_raw_for_i64_exact(market: &MinimalSpotMarket) -> i64 {
        let interest = market.cumulative_deposit_interest;
        if interest == 0 {
            return i64::MAX;
        }
        let max_product = (i64::MAX as u128)
            .checked_mul(SPOT_CUMULATIVE_INTEREST_PRECISION)
            .unwrap_or(u128::MAX);
        let safe = max_product / interest;
        if safe > i64::MAX as u128 {
            i64::MAX
        } else {
            safe as i64
        }
    }

    fn overflow_raw_for_i64_exact(market: &MinimalSpotMarket) -> i64 {
        largest_safe_raw_for_i64_exact(market).saturating_add(1)
    }

    /// Find the largest u64 token amount that won't overflow in scaled_balance_increment.
    /// Formula: scaled = amount * precision_increase / interest
    /// Safe when: amount <= u64::MAX * interest / precision_increase
    fn largest_safe_amount_for_scaled_increment(market: &MinimalSpotMarket) -> u64 {
        let precision_increase = get_precision_increase(market.decimals).unwrap();
        let interest = market.cumulative_deposit_interest;
        let safe = (u64::MAX as u128 * interest) / precision_increase;
        if safe > u64::MAX as u128 {
            u64::MAX
        } else {
            safe as u64
        }
    }

    fn overflow_amount_for_scaled_increment(market: &MinimalSpotMarket) -> u64 {
        largest_safe_amount_for_scaled_increment(market).saturating_add(1)
    }

    fn spot_market(decimals: u32, cumulative_deposit_interest: u128) -> MinimalSpotMarket {
        let mut market = MinimalSpotMarket::zeroed();
        market.decimals = decimals;
        market.cumulative_deposit_interest = cumulative_deposit_interest;
        market
    }

    fn user_with_deposit(market_index: u16, scaled_balance: u64) -> MinimalUser {
        let mut user = MinimalUser::zeroed();
        let position_index = if market_index == 0 { 0 } else { 1 };
        user.spot_positions[position_index] = SpotPosition {
            scaled_balance,
            open_bids: 0,
            open_asks: 0,
            cumulative_deposits: 0,
            market_index,
            balance_type: SpotBalanceType::Deposit,
            open_orders: 0,
            padding: [0; 4],
        };
        user
    }

    fn user_with_multiple_deposits(deposits: &[(u16, u64)]) -> MinimalUser {
        let mut user = MinimalUser::zeroed();
        for (i, (market_index, scaled_balance)) in deposits.iter().enumerate() {
            if i >= 8 {
                break;
            }
            user.spot_positions[i] = SpotPosition {
                scaled_balance: *scaled_balance,
                open_bids: 0,
                open_asks: 0,
                cumulative_deposits: 0,
                market_index: *market_index,
                balance_type: SpotBalanceType::Deposit,
                open_orders: 0,
                padding: [0; 4],
            };
        }
        user
    }

    #[test]
    fn precision_increase_for_common_decimals() {
        assert_eq!(get_precision_increase(6).unwrap(), EXP_10[13]); // USDC
        assert_eq!(get_precision_increase(9).unwrap(), EXP_10[10]); // SOL
        assert_eq!(get_precision_increase(8).unwrap(), EXP_10[11]); // BTC
        assert!(get_precision_increase(DRIFT_PRECISION_EXP + 1).is_err());
    }

    #[test]
    fn scaled_balance_increment() {
        // At 1.0x interest: scaled = 100_000_000 * 10^13 / 10^10 = 100_000_000_000
        let market_1x = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);
        assert_eq!(
            market_1x.get_scaled_balance_increment(100_000_000).unwrap(),
            100_000_000_000u64
        );

        // At 1.2x interest: scaled = 100_000_000 * 10^13 / (12 * 10^9) = 83_333_333_333
        let market_1_2x = spot_market(6, 12_000_000_000u128);
        assert_eq!(
            market_1_2x
                .get_scaled_balance_increment(100_000_000)
                .unwrap(),
            83_333_333_333u64
        );
    }

    #[test]
    fn scaled_balance_increment_floors_result() {
        let market = spot_market(6, 12_000_000_000u128);

        // 1 * 10^13 / (12 * 10^9) = 833.33... floors to 833
        let increment = market.get_scaled_balance_increment(1).unwrap();
        let decrement = market.get_scaled_balance_decrement(1).unwrap();

        assert_eq!(increment, 833u64);
        assert_eq!(decrement, increment + 1); // decrement rounds up
    }

    #[test]
    fn scaled_balance_decrement() {
        // At 1.0x: scaled = 100_000_000_000, rounded up: +1
        let market_1x = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);
        assert_eq!(
            market_1x.get_scaled_balance_decrement(100_000_000).unwrap(),
            100_000_000_001u64
        );

        // At 1.2x: scaled = 83_333_333_333, rounded up: +1
        let market_1_2x = spot_market(6, 12_000_000_000u128);
        assert_eq!(
            market_1_2x
                .get_scaled_balance_decrement(100_000_000)
                .unwrap(),
            83_333_333_334u64
        );
    }

    #[test]
    fn scaled_balance_decrement_zero_returns_zero() {
        let market = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);
        assert_eq!(market.get_scaled_balance_decrement(0).unwrap(), 0u64);
    }

    #[test]
    fn scaled_balance_decrement_always_greater_than_increment() {
        let market = spot_market(6, 12_000_000_000u128);

        for amount in [1u64, 100, 1_000_000, 100_000_000] {
            let increment = market.get_scaled_balance_increment(amount).unwrap();
            let decrement = market.get_scaled_balance_decrement(amount).unwrap();
            assert!(decrement >= increment + 1);
        }
    }

    #[test]
    fn withdraw_token_amount() {
        // At 1.0x: tokens = 100_000_000_000 * 10^10 / 10^13 = 100_000_000
        let market_1x = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);
        assert_eq!(
            market_1x
                .get_withdraw_token_amount(100_000_000_000)
                .unwrap(),
            100_000_000u64
        );

        // At 1.2x: tokens = 100_000_000_000 * 12_000_000_000 / 10^13 = 120_000_000
        let market_1_2x = spot_market(6, 12_000_000_000u128);
        assert_eq!(
            market_1_2x
                .get_withdraw_token_amount(100_000_000_000)
                .unwrap(),
            120_000_000u64
        );
    }

    #[test]
    fn withdraw_token_amount_zero_returns_zero() {
        let market = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);
        assert_eq!(market.get_withdraw_token_amount(0).unwrap(), 0u64);
    }

    #[test]
    fn round_trip_deposit_then_withdraw_at_same_interest() {
        let market = spot_market(6, 12_000_000_000u128);
        let deposit_amount = 100_000_000u64;

        let scaled = market.get_scaled_balance_increment(deposit_amount).unwrap();
        let withdrawn = market.get_withdraw_token_amount(scaled).unwrap();

        // Should lose at most 1 token unit due to rounding
        assert!(withdrawn <= deposit_amount);
        assert!(deposit_amount - withdrawn == 1);
    }

    #[test]
    fn round_trip_with_interest_accrual() {
        let deposit_market = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);
        let withdraw_market = spot_market(6, 12_000_000_000u128);
        let deposit_amount = 100_000_000u64;

        // Deposit at 1.0x, withdraw at 1.2x -> ~20% profit
        let scaled = deposit_market
            .get_scaled_balance_increment(deposit_amount)
            .unwrap();
        let withdrawn = withdraw_market.get_withdraw_token_amount(scaled).unwrap();

        assert!(withdrawn > deposit_amount);
        assert_eq!(withdrawn, 120_000_000u64);
    }

    #[test]
    fn immediate_withdraw_decrement_exceeds_increment_by_one() {
        let market = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);
        let amount = 100_000_000u64;

        let increment = market.get_scaled_balance_increment(amount).unwrap();
        let decrement = market.get_scaled_balance_decrement(amount).unwrap();

        assert_eq!(decrement, increment + 1);
    }

    #[test]
    fn adjust_oracle_price() {
        let price = 1_000_000i64;

        // At 1.0x: adjusted = price
        let market_1x = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);
        assert_eq!(market_1x.adjust_oracle_price(price).unwrap(), price);

        // At 1.2x: adjusted = 1_200_000
        let market_1_2x = spot_market(6, 12_000_000_000u128);
        assert_eq!(
            market_1_2x.adjust_oracle_price(price).unwrap(),
            1_200_000i64
        );
    }

    #[test]
    fn adjust_u64() {
        let market = spot_market(6, 15_000_000_000u128); // 1.5x
        assert_eq!(market.adjust_u64(10_000).unwrap(), 15_000u64);
    }

    #[test]
    fn adjust_i128_for_switchboard_price() {
        let market = spot_market(6, 12_000_000_000u128);
        let price: i128 = 1_000_000_000_000_000_000;
        assert_eq!(
            market.adjust_i128(price).unwrap(),
            1_200_000_000_000_000_000i128
        );
    }

    #[test]
    fn adjust_negative_values_fails() {
        let market = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);
        assert!(market.adjust_i64(-1).is_err());
        assert!(market.adjust_i128(-1).is_err());
    }

    #[test]
    fn market_staleness() {
        let mut market = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);
        market.last_interest_ts = 1000;

        assert!(market.is_stale(1001));
        assert!(market.is_stale(2000));
        assert!(!market.is_stale(1000));
        assert!(!market.is_stale(999));
    }

    #[test]
    fn user_scaled_balance_and_deposit_counting() {
        let user = user_with_deposit(0, 1000);
        assert_eq!(user.get_scaled_balance(0), 1000);

        let user = user_with_deposit(1, 2000);
        assert_eq!(user.get_scaled_balance(1), 2000);
        assert_eq!(user.count_active_deposits(), 1);

        let user = user_with_multiple_deposits(&[(0, 100), (1, 0), (2, 300)]);
        assert_eq!(user.count_active_deposits(), 2);
    }

    #[test]
    fn user_admin_deposit_validation() {
        // 1 main + 2 rewards deposits: valid
        let user = user_with_multiple_deposits(&[(0, 100), (1, 200), (2, 300)]);
        assert!(user.validate_not_bricked_by_admin_deposits().is_ok());

        // 4+ deposits: bricked
        let user = user_with_multiple_deposits(&[(0, 100), (1, 200), (2, 300), (3, 400)]);
        assert!(user.validate_not_bricked_by_admin_deposits().is_err());

        // Admin deposits in positions 2-7
        let mut user = user_with_deposit(1, 1000);
        user.spot_positions[2] = SpotPosition {
            scaled_balance: 500,
            open_bids: 0,
            open_asks: 0,
            cumulative_deposits: 0,
            market_index: 3,
            balance_type: SpotBalanceType::Deposit,
            open_orders: 0,
            padding: [0; 4],
        };
        assert!(user.has_admin_deposit(3).is_ok());
    }

    #[test]
    fn user_reward_account_validation() {
        let user = user_with_deposit(1, 1000);
        assert!(user.validate_reward_accounts(true, true).is_ok());

        let user = user_with_multiple_deposits(&[(0, 100), (1, 200)]);
        assert!(user.validate_reward_accounts(true, true).is_err());
        assert!(user.validate_reward_accounts(false, true).is_ok());

        let user = user_with_multiple_deposits(&[(0, 100), (1, 200), (2, 300)]);
        assert!(user.validate_reward_accounts(false, true).is_err());
        assert!(user.validate_reward_accounts(false, false).is_ok());
    }

    #[test]
    fn adjust_u64_overflow_at_exact_boundary() {
        // 200x interest to trigger overflow
        let market = spot_market(6, 2_000_000_000_000u128);

        let safe = largest_safe_raw_for_u64_exact(&market);
        assert!(
            market.adjust_u64(safe).is_ok(),
            "safe value {} should succeed",
            safe
        );

        let ovf = overflow_raw_for_u64_exact(&market);
        assert!(
            market.adjust_u64(ovf).is_err(),
            "overflow value {} should fail",
            ovf
        );

        // Verify meaningful boundary (not just MAX)
        assert!(safe < u64::MAX);
    }

    #[test]
    fn adjust_i64_overflow_at_exact_boundary() {
        let market = spot_market(6, 2_000_000_000_000u128);

        let safe = largest_safe_raw_for_i64_exact(&market);
        assert!(
            market.adjust_i64(safe).is_ok(),
            "safe value {} should succeed",
            safe
        );

        let ovf = overflow_raw_for_i64_exact(&market);
        assert!(
            market.adjust_i64(ovf).is_err(),
            "overflow value {} should fail",
            ovf
        );

        assert!(safe < i64::MAX);
    }

    #[test]
    fn scaled_balance_overflow_at_exact_boundary() {
        // Low interest (0.1x) creates high multiplier, triggers overflow
        let market = spot_market(6, 1_000_000_000u128);

        let safe = largest_safe_amount_for_scaled_increment(&market);
        assert!(
            market.get_scaled_balance_increment(safe).is_ok(),
            "safe amount {} should succeed",
            safe
        );

        let ovf = overflow_amount_for_scaled_increment(&market);
        assert!(
            market.get_scaled_balance_increment(ovf).is_err(),
            "overflow amount {} should fail",
            ovf
        );

        assert!(safe < u64::MAX);
    }

    #[test]
    fn adjust_i128_overflow_detection() {
        let market = spot_market(6, SPOT_CUMULATIVE_INTEREST_PRECISION);

        assert!(market.adjust_i128(-1).is_err());
        assert!(market.adjust_i128(i128::MIN).is_err());

        // Large positive values overflow during multiply
        let market_10x = spot_market(6, 100_000_000_000u128);
        assert!(market_10x.adjust_i128(i128::MAX / 5).is_err());

        // Normal Switchboard values work
        assert!(market.adjust_i128(1_000_000_000_000_000_000i128).is_ok());
    }

    #[test]
    fn sol_9_decimals_scaling() {
        let market = spot_market(9, SPOT_CUMULATIVE_INTEREST_PRECISION);
        let one_sol = 1_000_000_000u64;

        let scaled = market.get_scaled_balance_increment(one_sol).unwrap();
        assert_eq!(scaled, 1_000_000_000u64);

        let tokens = market.get_withdraw_token_amount(scaled).unwrap();
        assert_eq!(tokens, one_sol);
    }

    #[test]
    fn btc_8_decimals_scaling() {
        let market = spot_market(8, SPOT_CUMULATIVE_INTEREST_PRECISION);
        let one_btc = 100_000_000u64;

        let scaled = market.get_scaled_balance_increment(one_btc).unwrap();
        assert_eq!(scaled, 1_000_000_000u64);

        let tokens = market.get_withdraw_token_amount(scaled).unwrap();
        assert_eq!(tokens, one_btc);
    }

    #[test]
    fn integer_division_floors_correctly() {
        let market = spot_market(6, 12_000_000_000u128); // 1.2x

        assert_eq!(market.adjust_i64(5).unwrap(), 6); // 5 * 1.2 = 6 (exact)
        assert_eq!(market.adjust_i64(1).unwrap(), 1); // 1 * 1.2 = 1.2, floors to 1
        assert_eq!(market.adjust_i64(4).unwrap(), 4); // 4 * 1.2 = 4.8, floors to 4
        assert_eq!(market.get_scaled_balance_increment(1).unwrap(), 833); // 833.33... floors
    }
}
