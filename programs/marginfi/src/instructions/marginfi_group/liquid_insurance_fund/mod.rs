//! Module for liquid insurance pool instructions
//! Includes:
//! Create liquid insurance fund
//! Deposit into the bank's insurance vault
//! Request a withdrawal
//! Claim withdrawal
mod create_fund;
mod deposit;
mod withdraw_claim;
mod withdraw_request;

pub use create_fund::*;
pub use deposit::*;
pub use withdraw_claim::*;
pub use withdraw_request::*;
