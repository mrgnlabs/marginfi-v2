use anchor_lang::prelude::*;

#[event]
pub struct MarginfiAccountCreateEvent {
    pub signer: Pubkey,
    pub marginfi_account: Pubkey,
    pub marginfi_group: Pubkey,
}

#[event]
pub struct LendingAccountDepositEvent {
    pub amount: u64,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub signer: Pubkey,
    pub marginfi_account: Pubkey,
    pub marginfi_group: Pubkey,
}

#[event]
pub struct LendingAccountRepayEvent {
    pub amount: u64,
    pub close_balance: bool,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub signer: Pubkey,
    pub marginfi_account: Pubkey,
    pub marginfi_group: Pubkey,
}

#[event]
pub struct LendingAccountBorrowEvent {
    pub amount: u64,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub signer: Pubkey,
    pub marginfi_account: Pubkey,
    pub marginfi_group: Pubkey,
}

#[event]
pub struct LendingAccountWithdrawEvent {
    pub amount: u64,
    pub close_balance: bool,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub signer: Pubkey,
    pub marginfi_account: Pubkey,
    pub marginfi_group: Pubkey,
}
