use anchor_lang::prelude::*;

#[error_code]
pub enum MarginfiError {
    #[msg("Math error")] // 6000
    MathError,
    #[msg("Invalid bank index")] // 6001
    BankNotFound,

    #[msg("Lending account balance not found")] // 6002
    LendingAccountBalanceNotFound,
    #[msg("Bank deposit capacity exceeded")] // 6003
    BankAssetCapacityExceeded,
    #[msg("Invalid transfer")] // 6004
    InvalidTransfer,
    #[msg("Missing Pyth or Bank account")] // 6005
    MissingPythOrBankAccount,
    #[msg("Missing Pyth account")] // 6006
    MissingPythAccount,
    #[msg("Invalid Pyth account")] // 6007
    InvalidOracleAccount,
    #[msg("Missing Bank account")] // 6008
    MissingBankAccount,
    #[msg("Invalid Bank account")] // 6009
    InvalidBankAccount,
    #[msg("Bad account health")] // 6010
    BadAccountHealth,
    #[msg("Lending account balance slots are full")] // 6011
    LendingAccountBalanceSlotsFull,
    #[msg("Bank already exists")] // 6012
    BankAlreadyExists,
    #[msg("Illegal liquidation")] // 6013
    IllegalLiquidation,
    #[msg("Account is not bankrupt")] // 6014
    AccountNotBankrupt,
    #[msg("Account balance is not bad debt")] // 6015
    BalanceNotBadDebt,
    #[msg("Invalid group config")] // 6016
    InvalidConfig,
    #[msg("Stale oracle data")] // 6017
    StaleOracle,
    #[msg("Bank paused")] // 6018
    BankPaused,
    #[msg("Bank is ReduceOnly mode")] // 6019
    BankReduceOnly,
    #[msg("Bank is missing")] // 6020
    BankAccoutNotFound,
    #[msg("Operation is deposit-only")] // 6021
    OperationDepositOnly,
    #[msg("Operation is withdraw-only")] // 6022
    OperationWithdrawOnly,
    #[msg("Operation is borrow-only")] // 6023
    OperationBorrowOnly,
    #[msg("Operation is repay-only")] // 6024
    OperationRepayOnly,
    #[msg("No asset found")] // 6025
    NoAssetFound,
    #[msg("No liability found")] // 6026
    NoLiabilityFound,
    #[msg("Invalid oracle setup")] // 6027
    InvalidOracleSetup,
    #[msg("Invalid bank utilization ratio")] // 6028
    IllegalUtilizationRatio,
    #[msg("Bank borrow cap exceeded")] // 6029
    BankLiabilityCapacityExceeded,
    #[msg("Invalid Price")] // 6030
    InvalidPrice,
    #[msg("Account can have only one liablity when account is under isolated risk")] // 6031
    IsolatedAccountIllegalState, // 6032
    #[msg("Emissions already setup")]
    EmissionsAlreadySetup,
    #[msg("Oracle is not set")] // 6033
    OracleNotSetup,
    #[msg("Invalid swithcboard decimal conversion")] // 6034
    InvalidSwitchboardDecimalConversion,
    #[msg("Cannot close balance because of outstanding emissions")] // 6035
    CannotCloseOutstandingEmissions,
    #[msg("Update emissions error")] //6036
    EmissionsUpdateError,
    #[msg("Account disabled")] // 6037
    AccountDisabled,
    #[msg("Account can't temporarily open 3 balances, please close a balance first")] // 6038
    AccountTempActiveBalanceLimitExceeded,
    #[msg("Illegal action during flashloan")] // 6039
    AccountInFlashloan,
    #[msg("Illegal flashloan")] // 6040
    IllegalFlashloan,
    #[msg("Illegal flag")] // 6041
    IllegalFlag,
    #[msg("Illegal balance state")] // 6043
    IllegalBalanceState,
}

impl From<MarginfiError> for ProgramError {
    fn from(e: MarginfiError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
