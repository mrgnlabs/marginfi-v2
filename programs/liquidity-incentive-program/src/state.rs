use anchor_lang::prelude::*;

#[account]
#[derive(Debug)]
pub struct Campaign {
    pub admin: Pubkey,
    pub lockup_period: u64,
    pub active: bool,
    pub max_deposits: u64,
    pub outstanding_deposits: u64,
    pub max_rewards: u64,
    pub marginfi_bank_pk: Pubkey,
}

#[account]
pub struct Deposit {
    pub owner: Pubkey,
    pub amount: u64,
    pub start_time: i64,
    pub campaign: Pubkey,
}
