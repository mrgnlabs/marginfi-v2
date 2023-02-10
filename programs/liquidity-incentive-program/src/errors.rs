use anchor_lang::prelude::*;

#[error_code]
pub enum LIPError {
    #[msg("Campaign is not active")]
    CampaignNotActive,
    #[msg("Deposit amount is to large")]
    DepositAmountTooLarge,
    #[msg("Deposit hasn't matured yet")]
    DepositNotMature,
}
