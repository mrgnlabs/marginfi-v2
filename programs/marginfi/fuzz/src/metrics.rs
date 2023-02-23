use lazy_static::lazy_static;
use std::sync::atomic::AtomicU64;

lazy_static! {
    pub static ref LOG_COUNTER: AtomicU64 = AtomicU64::new(0);
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        #[cfg(feature = "capture_log")] {
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

    pub fn print(&self) {
        print!("\r");
        print!("{}", self.get_print_string());
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
