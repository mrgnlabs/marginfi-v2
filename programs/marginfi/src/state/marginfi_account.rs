use anchor_lang::prelude::*;
use fixed::types::I80F48;

#[account(zero_copy)]
pub struct MarginfiAccount {
    pub group: Pubkey,
    pub owner: Pubkey,
}

impl MarginfiAccount {
    /// Set the initial data for the marginfi account.
    pub fn initialize(&mut self, group: Pubkey, owner: Pubkey) {
        self.owner = owner;
        self.group = group;
    }
}

const MAX_LENDING_ACCOUNT_BALANCES: usize = 16;

#[zero_copy]
pub struct LendingAccount {
    pub balances: [Balance; MAX_LENDING_ACCOUNT_BALANCES],
}

#[zero_copy]
pub struct Balance {
    pub lending_pool_reserve_index: u8,
    pub deposit_shares: I80F48,
    pub liability_shares: I80F48,
}
