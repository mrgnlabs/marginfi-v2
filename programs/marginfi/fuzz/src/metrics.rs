use anchor_lang::error::Error;
use itertools::Itertools;
use lazy_static::lazy_static;
use std::{collections::HashMap, sync::atomic::AtomicU64};

lazy_static! {
    pub static ref LOG_COUNTER: AtomicU64 = AtomicU64::new(0);
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        #[cfg(feature = "capture_log")] {
            use base64::Engine;
            let mut ct = $crate::metrics::LOG_COUNTER.load(std::sync::atomic::Ordering::Acquire);

            let header = format!("{} -", ct);
            let msg = format!($($arg)*);
            log::info!("{} {}", header, msg);

            ct += 1;
            $crate::metrics::LOG_COUNTER.store(ct, std::sync::atomic::Ordering::Release);
        }
    }
}

#[derive(Default, Debug)]
pub struct Metrics {
    deposit_s: u64,
    deposit_e: u64,
    withdraw_s: u64,
    withdraw_e: u64,
    borrow_s: u64,
    borrow_e: u64,
    repay_s: u64,
    repay_e: u64,
    liquidate_e: u64,
    liquidate_s: u64,
    handle_bankruptcy_s: u64,
    handle_bankruptcy_e: u64,
    pub price_update: u64,
    error_counts: HashMap<String, u64>,
}

#[derive(Debug)]
pub enum MetricAction {
    Deposit,
    Withdraw,
    Borrow,
    Repay,
    Liquidate,
    Bankruptcy,
}

impl Metrics {
    pub fn update_metric(&mut self, metric: MetricAction, success: bool) {
        log!("Result {:?} {}", metric, success);

        let metric = match (metric, success) {
            (MetricAction::Deposit, true) => &mut self.deposit_s,
            (MetricAction::Deposit, false) => &mut self.deposit_e,
            (MetricAction::Withdraw, true) => &mut self.withdraw_s,
            (MetricAction::Withdraw, false) => &mut self.withdraw_e,
            (MetricAction::Borrow, true) => &mut self.borrow_s,
            (MetricAction::Borrow, false) => &mut self.borrow_e,
            (MetricAction::Repay, true) => &mut self.repay_s,
            (MetricAction::Repay, false) => &mut self.repay_e,
            (MetricAction::Liquidate, true) => &mut self.liquidate_s,
            (MetricAction::Liquidate, false) => &mut self.liquidate_e,
            (MetricAction::Bankruptcy, true) => &mut self.handle_bankruptcy_s,
            (MetricAction::Bankruptcy, false) => &mut self.handle_bankruptcy_e,
        };

        *metric += 1;
    }

    pub fn update_error(&mut self, error: &Error) {
        let error_name = match error {
            Error::AnchorError(e) => {
                e.error_name.clone()
            }
            Error::ProgramError(e) => {
                e.program_error.to_string()
            }
        };

        let error_count = self.error_counts.entry(error_name).or_insert(0);
        *error_count += 1;
    }

    pub fn print(&self) {
        println!("{}", self.get_print_string());
        println!("{:?}", self.get_error_counts_string());
    }

    pub fn get_error_counts_string(&self) -> String {
        let top_5_by_count = self.error_counts.iter().sorted_by_key(|(_, count)| *count).rev().take(5).collect::<Vec<_>>();
        let top_5_by_count_str = top_5_by_count.iter().map(|(error, count)| {
            format!("{}: {}", error, count)
        }).collect::<Vec<_>>().join(", ");

        format!("Top 5 errors: {}", top_5_by_count_str)
    }

    pub fn get_print_string(&self) -> String {
        format!("Deposit\t{}\t{}\tWithd\t{}\t{}\tBorrow\t{}\t{}\tRepay\t{}\t{}\tLiq\t{}\t{}\tBank\t{}\t{}\tUpdate\t{}",
            self.deposit_s,
            self.deposit_e,    
            self.withdraw_s,
            self.withdraw_e,
            self.borrow_s,
            self.borrow_e,
            self.repay_s,
            self.repay_e,
            self.liquidate_s,
            self.liquidate_e,
            self.handle_bankruptcy_s,
            self.handle_bankruptcy_e,
            self.price_update)
    }

    pub fn log(&self) {
        log!("{}", self.get_print_string())
    }
}
