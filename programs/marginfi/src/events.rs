use anchor_lang::prelude::*;

// Event headers

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GroupEventHeader {
    pub version: String,
    pub signer: Option<Pubkey>,
    pub marginfi_group: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AccountEventHeader {
    pub version: String,
    pub signer: Pubkey,
    pub marginfi_account: Pubkey,
    pub marginfi_group: Pubkey,
}

// marginfi group events

#[event]
pub struct LendingPoolBankAccrueInterestEvent {
    pub header: GroupEventHeader,
    pub mint: Pubkey,
    pub delta: u64,
    pub fees_collected: f64,
    pub insurance_collected: f64,
}

#[event]
pub struct LendingPoolBankAddEvent {
    pub header: GroupEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
}

// marginfi account events

#[event]
pub struct MarginfiAccountCreateEvent {
    pub header: AccountEventHeader,
}

#[event]
pub struct LendingAccountDepositEvent {
    pub header: AccountEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

#[event]
pub struct LendingAccountRepayEvent {
    pub header: AccountEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub close_balance: bool,
}

#[event]
pub struct LendingAccountBorrowEvent {
    pub header: AccountEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

#[event]
pub struct LendingAccountWithdrawEvent {
    pub header: AccountEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub close_balance: bool,
}
