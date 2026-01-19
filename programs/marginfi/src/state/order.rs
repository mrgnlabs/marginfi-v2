use crate::{check, check_eq, errors::MarginfiError, prelude::MarginfiResult};
use anchor_lang::prelude::*;
use fixed::types::I80F48;
use marginfi_type_crate::{
    constants::{ORDER_ACTIVE_TAGS, ORDER_TAG_PADDING},
    types::{
        BalanceSide, ExecuteOrderBalanceRecord, ExecuteOrderRecord, MarginfiAccount, Order,
        OrderTrigger, OrderTriggerType, WrappedI80F48, MAX_EXECUTE_RECORD_BALANCES,
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
        closed_order_balances_count: usize,
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
        self.balance_states = [ExecuteOrderBalanceRecord::default(); MAX_EXECUTE_RECORD_BALANCES];

        let mut idx: usize = 0;
        let mut inactive_count: u8 = 0;

        for balance in marginfi_account.lending_account.balances.iter() {
            if !balance.is_active() {
                inactive_count += 1;
                continue;
            }

            if idx >= self.balance_states.len() {
                return Err(error!(MarginfiError::IllegalBalanceState));
            }

            // Skip balances that belong to this order, they can be changed by the keeper
            if balance.tag != 0 && order_tags.iter().any(|t| *t == balance.tag) {
                continue;
            }

            let ExecuteOrderBalanceRecord {
                bank,
                tag,
                is_asset,
                shares,
                ..
            } = &mut self.balance_states[idx];

            let side = balance
                .get_side()
                .ok_or_else(|| error!(MarginfiError::IllegalBalanceState))?;

            *bank = balance.bank_pk;
            *tag = balance.tag;
            *is_asset = matches!(side, BalanceSide::Assets) as u8;
            *shares = match side {
                BalanceSide::Assets => balance.asset_shares,
                BalanceSide::Liabilities => balance.liability_shares,
            };

            idx += 1;
        }

        self.inactive_balance_count = inactive_count;
        self.active_balance_count = idx.try_into().unwrap();

        Ok(())
    }

    fn verify_unchanged(
        &self,
        marginfi_account: &MarginfiAccount,
        closed_order_balances_count: usize,
    ) -> MarginfiResult {
        let inactive_balance_count = marginfi_account
            .lending_account
            .balances
            .iter()
            .filter(|balance| !balance.is_active())
            .count();

        for record in self.balance_states[..self.active_balance_count as usize].iter() {
            let index = marginfi_account
                .lending_account
                .balances
                .binary_search_by(|balance| record.bank.cmp(&balance.bank_pk))
                .map_err(|_| MarginfiError::IllegalBalanceState)?;

            let balance = &marginfi_account.lending_account.balances[index];

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
        }

        // This implies that the inactive balances were also not touched.
        // This check is not strictly necessary since deposits are not allowed
        // during execution and the above has checked that the open balances are
        // still open and the same, but is left here as a sanity check.
        check!(
            self.inactive_balance_count as usize + closed_order_balances_count
                == inactive_balance_count,
            MarginfiError::IllegalBalanceState
        );

        Ok(())
    }
}
