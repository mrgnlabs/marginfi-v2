use crate::{check_eq, errors::MarginfiError, prelude::MarginfiResult};
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use marginfi_type_crate::{
    constants::{ORDER_ACTIVE_TAGS, ORDER_TAG_PADDING},
    types::{
        BalanceSide, ExecuteOrderBalanceRecord, ExecuteOrderRecord, MarginfiAccount, Order,
        OrderTrigger, OrderTriggerType, WrappedI80F48, MAX_LENDING_ACCOUNT_BALANCES,
    },
};

pub trait OrderImpl {
    fn initialize(
        &mut self,
        marginfi_account: Pubkey,
        trigger: OrderTrigger,
        tags: [u16; ORDER_ACTIVE_TAGS],
        bump: u8,
    ) -> MarginfiResult;
}

impl OrderImpl for Order {
    fn initialize(
        &mut self,
        marginfi_account: Pubkey,
        trigger: OrderTrigger,
        tags: [u16; ORDER_ACTIVE_TAGS],
        bump: u8,
    ) -> MarginfiResult {
        self.marginfi_account = marginfi_account;
        match trigger {
            OrderTrigger::StopLoss { threshold } => {
                self.trigger = OrderTriggerType::StopLoss;
                self.stop_loss = threshold;
                self.take_profit = WrappedI80F48::default();
                // Threshold must be > 0
                let val: I80F48 = self.stop_loss.into();
                if val <= I80F48::ZERO {
                    return Err(error!(MarginfiError::OrderTriggerValueNonPositive));
                }
            }
            OrderTrigger::TakeProfit { threshold } => {
                self.trigger = OrderTriggerType::TakeProfit;
                self.take_profit = threshold;
                self.stop_loss = WrappedI80F48::default();
                // Threshold must be > 0
                let val: I80F48 = self.take_profit.into();
                if val <= I80F48::ZERO {
                    return Err(error!(MarginfiError::OrderTriggerValueNonPositive));
                }
            }
            OrderTrigger::Both {
                stop_loss,
                take_profit,
            } => {
                self.trigger = OrderTriggerType::Both;
                self.stop_loss = stop_loss;
                self.take_profit = take_profit;
                // Both thresholds must be > 0
                let sl: I80F48 = self.stop_loss.into();
                let tp: I80F48 = self.take_profit.into();
                if sl <= I80F48::ZERO || tp <= I80F48::ZERO {
                    return Err(error!(MarginfiError::OrderTriggerValueNonPositive));
                }
            }
        }

        self.tags = tags;
        self._tags_padding = [0; ORDER_TAG_PADDING];
        self.bump = bump;
        self._reserved0 = [0; 5];
        self._reserved1 = [0; 2];

        Ok(())
    }
}

pub trait ExecuteOrderRecordImpl {
    fn initialize(
        &mut self,
        order: Pubkey,
        executor: Pubkey,
        marginfi_account: &MarginfiAccount,
        order_tags: &[u16],
    ) -> MarginfiResult;

    fn verify_unchanged(
        &self,
        marginfi_account: &MarginfiAccount,
        order_tags: &[u16],
    ) -> MarginfiResult;
}

impl ExecuteOrderRecordImpl for ExecuteOrderRecord {
    fn initialize(
        &mut self,
        order: Pubkey,
        executor: Pubkey,
        marginfi_account: &MarginfiAccount,
        order_tags: &[u16],
    ) -> MarginfiResult {
        self.order = order;
        self.executor = executor;
        self.balance_states = [ExecuteOrderBalanceRecord::default(); MAX_LENDING_ACCOUNT_BALANCES];

        for (i, balance) in marginfi_account.lending_account.balances.iter().enumerate() {
            if !balance.is_active() {
                continue;
            }

            // Skip balances that belong to this order, they can be changed by the liquidator
            if balance.tag != 0 && order_tags.iter().any(|t| *t == balance.tag) {
                continue;
            }

            let record = &mut self.balance_states[i];
            record.bank = balance.bank_pk;
            record.is_active = 1;
            let side = balance
                .get_side()
                .ok_or_else(|| error!(MarginfiError::IllegalBalanceState))?;
            record.is_asset = matches!(side, BalanceSide::Assets) as u8;
            record.shares = match side {
                BalanceSide::Assets => balance.asset_shares,
                BalanceSide::Liabilities => balance.liability_shares,
            }
        }

        Ok(())
    }

    fn verify_unchanged(
        &self,
        marginfi_account: &MarginfiAccount,
        order_tags: &[u16],
    ) -> MarginfiResult {
        for (record, balance) in self
            .balance_states
            .iter()
            .zip(marginfi_account.lending_account.balances.iter())
        {
            let balance_is_active = balance.is_active();
            let balance_in_order = balance.tag != 0 && order_tags.iter().any(|t| *t == balance.tag);
            let record_is_active = record.is_active == 1;

            if record_is_active {
                if !balance_is_active {
                    return Err(error!(MarginfiError::IllegalBalanceState));
                }

                check_eq!(
                    record.bank,
                    balance.bank_pk,
                    MarginfiError::IllegalBalanceState
                );

                let side = balance
                    .get_side()
                    .ok_or_else(|| error!(MarginfiError::IllegalBalanceState))?;

                let expected_is_asset = matches!(side, BalanceSide::Assets) as u8;

                check_eq!(
                    record.is_asset,
                    expected_is_asset,
                    MarginfiError::IllegalBalanceState
                );

                let expected_shares = match side {
                    BalanceSide::Assets => balance.asset_shares,
                    BalanceSide::Liabilities => balance.liability_shares,
                };

                check_eq!(
                    record.shares,
                    expected_shares,
                    MarginfiError::IllegalBalanceState
                );
            } else if balance_is_active && !balance_in_order {
                // Record marked inactive but an unexpected non-order balance is now active.
                return Err(error!(MarginfiError::IllegalBalanceState));
            }
        }

        Ok(())
    }
}
