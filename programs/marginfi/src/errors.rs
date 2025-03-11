use anchor_lang::prelude::*;

#[error_code]
pub enum MarginfiError {
    #[msg("Internal Marginfi logic error")] // 6000
    InternalLogicError,
    #[msg("Invalid bank index")] // 6001
    BankNotFound,
    #[msg("Lending account balance not found")] // 6002
    LendingAccountBalanceNotFound,
    #[msg("Bank deposit capacity exceeded")] // 6003
    BankAssetCapacityExceeded,
    #[msg("Invalid transfer")] // 6004
    InvalidTransfer,
    #[msg("Missing Oracle, Bank, LST mint, or Sol Pool")] // 6005
    MissingPythOrBankAccount,
    #[msg("Missing Pyth account")] // 6006
    MissingPythAccount,
    #[msg("Missing Bank account")] // 6007
    MissingBankAccount,
    #[msg("Invalid Bank account")] // 6008
    InvalidBankAccount,
    #[msg("RiskEngine rejected due to either bad health or stale oracles")] // 6009
    RiskEngineInitRejected,
    #[msg("Lending account balance slots are full")] // 6010
    LendingAccountBalanceSlotsFull,
    #[msg("Bank already exists")] // 6011
    BankAlreadyExists,
    #[msg("Illegal liquidation")] // 6012
    IllegalLiquidation,
    #[msg("Account is not bankrupt")] // 6013
    AccountNotBankrupt,
    #[msg("Account balance is not bad debt")] // 6014
    BalanceNotBadDebt,
    #[msg("Invalid group config")] // 6015
    InvalidConfig,
    #[msg("Bank paused")] // 6016
    BankPaused,
    #[msg("Bank is ReduceOnly mode")] // 6017
    BankReduceOnly,
    #[msg("Bank is missing")] // 6018
    BankAccountNotFound,
    #[msg("Operation is deposit-only")] // 6019
    OperationDepositOnly,
    #[msg("Operation is withdraw-only")] // 6020
    OperationWithdrawOnly,
    #[msg("Operation is borrow-only")] // 6021
    OperationBorrowOnly,
    #[msg("Operation is repay-only")] // 6022
    OperationRepayOnly,
    #[msg("No asset found")] // 6023
    NoAssetFound,
    #[msg("No liability found")] // 6024
    NoLiabilityFound,
    #[msg("Invalid oracle setup")] // 6025
    InvalidOracleSetup,
    #[msg("Invalid bank utilization ratio")] // 6026
    IllegalUtilizationRatio,
    #[msg("Bank borrow cap exceeded")] // 6027
    BankLiabilityCapacityExceeded,
    #[msg("Invalid Price")] // 6028
    InvalidPrice,
    #[msg("Account can have only one liability when account is under isolated risk")] // 6029
    IsolatedAccountIllegalState,
    #[msg("Emissions already setup")] // 6030
    EmissionsAlreadySetup,
    #[msg("Oracle is not set")] // 6031
    OracleNotSetup,
    #[msg("Invalid switchboard decimal conversion")] // 6032
    InvalidSwitchboardDecimalConversion,
    #[msg("Cannot close balance because of outstanding emissions")] // 6033
    CannotCloseOutstandingEmissions,
    #[msg("Update emissions error")] //6034
    EmissionsUpdateError,
    #[msg("Account disabled")] // 6035
    AccountDisabled,
    #[msg("Account can't temporarily open 3 balances, please close a balance first")] // 6036
    AccountTempActiveBalanceLimitExceeded,
    #[msg("Illegal action during flashloan")] // 6037
    AccountInFlashloan,
    #[msg("Illegal flashloan")] // 6038
    IllegalFlashloan,
    #[msg("Illegal flag")] // 6039
    IllegalFlag,
    #[msg("Illegal balance state")] // 6040
    IllegalBalanceState,
    #[msg("Illegal account authority transfer")] // 6041
    IllegalAccountAuthorityTransfer,
    #[msg("Unauthorized")] // 6042
    Unauthorized,
    #[msg("Invalid account authority")] // 6043
    IllegalAction,
    #[msg("Token22 Banks require mint account as first remaining account")] // 6044
    T22MintRequired,
    #[msg("Invalid ATA for global fee account")] // 6045
    InvalidFeeAta,
    #[msg("Use add pool permissionless instead")] // 6046
    AddedStakedPoolManually,
    #[msg("Staked SOL accounts can only deposit staked assets and borrow SOL")] // 6047
    AssetTagMismatch,
    #[msg("Stake pool validation failed: check the stake pool, mint, or sol pool")] // 6048
    StakePoolValidationFailed,
    #[msg("Switchboard oracle: stale price")] // 6049
    SwitchboardStalePrice,
    #[msg("Pyth Push oracle: stale price")] // 6050
    PythPushStalePrice,
    #[msg("Oracle error: wrong number of accounts")] // 6051
    WrongNumberOfOracleAccounts,
    #[msg("Oracle error: wrong account keys")] // 6052
    WrongOracleAccountKeys,
    #[msg("Pyth Push oracle: wrong account owner")] // 6053
    PythPushWrongAccountOwner,
    #[msg("Staked Pyth Push oracle: wrong account owner")] // 6054
    StakedPythPushWrongAccountOwner,
    #[msg("Pyth Push oracle: mismatched feed id")] // 6055
    PythPushMismatchedFeedId,
    #[msg("Pyth Push oracle: insufficient verification level")] // 6056
    PythPushInsufficientVerificationLevel,
    #[msg("Pyth Push oracle: feed id must be 32 Bytes")] // 6057
    PythPushFeedIdMustBe32Bytes,
    #[msg("Pyth Push oracle: feed id contains non-hex characters")] // 6058
    PythPushFeedIdNonHexCharacter,
    #[msg("Switchboard oracle: wrong account owner")] // 6059
    SwitchboardWrongAccountOwner,
    #[msg("Pyth Push oracle: invalid account")] // 6060
    PythPushInvalidAccount,
    #[msg("Switchboard oracle: invalid account")] // 6061
    SwitchboardInvalidAccount,
    #[msg("Math error")] // 6062
    MathError,
    #[msg("Invalid emissions destination account")] // 6063
    InvalidEmissionsDestinationAccount,
    #[msg("Arena groups can only support two banks")] // 6064
    ArenaBankLimit,
    #[msg("Arena groups cannot return to non-arena status")] // 6065
    ArenaSettingCannotChange,
}

impl From<MarginfiError> for ProgramError {
    fn from(e: MarginfiError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<pyth_solana_receiver_sdk::error::GetPriceError> for MarginfiError {
    fn from(e: pyth_solana_receiver_sdk::error::GetPriceError) -> Self {
        match e {
            pyth_solana_receiver_sdk::error::GetPriceError::PriceTooOld => {
                MarginfiError::PythPushStalePrice
            }
            pyth_solana_receiver_sdk::error::GetPriceError::MismatchedFeedId => {
                MarginfiError::PythPushMismatchedFeedId
            }
            pyth_solana_receiver_sdk::error::GetPriceError::InsufficientVerificationLevel => {
                MarginfiError::PythPushInsufficientVerificationLevel
            }
            pyth_solana_receiver_sdk::error::GetPriceError::FeedIdMustBe32Bytes => {
                MarginfiError::PythPushFeedIdMustBe32Bytes
            }
            pyth_solana_receiver_sdk::error::GetPriceError::FeedIdNonHexCharacter => {
                MarginfiError::PythPushFeedIdNonHexCharacter
            }
        }
    }
}
impl From<u32> for MarginfiError {
    fn from(value: u32) -> Self {
        match value {
            6001 => MarginfiError::BankNotFound,
            6002 => MarginfiError::LendingAccountBalanceNotFound,
            6003 => MarginfiError::BankAssetCapacityExceeded,
            6004 => MarginfiError::InvalidTransfer,
            6005 => MarginfiError::MissingPythOrBankAccount,
            6006 => MarginfiError::MissingPythAccount,
            6007 => MarginfiError::MissingBankAccount,
            6008 => MarginfiError::InvalidBankAccount,
            6009 => MarginfiError::RiskEngineInitRejected,
            6010 => MarginfiError::LendingAccountBalanceSlotsFull,
            6011 => MarginfiError::BankAlreadyExists,
            6012 => MarginfiError::IllegalLiquidation,
            6013 => MarginfiError::AccountNotBankrupt,
            6014 => MarginfiError::BalanceNotBadDebt,
            6015 => MarginfiError::InvalidConfig,
            6016 => MarginfiError::BankPaused,
            6017 => MarginfiError::BankReduceOnly,
            6018 => MarginfiError::BankAccountNotFound,
            6019 => MarginfiError::OperationDepositOnly,
            6020 => MarginfiError::OperationWithdrawOnly,
            6021 => MarginfiError::OperationBorrowOnly,
            6022 => MarginfiError::OperationRepayOnly,
            6023 => MarginfiError::NoAssetFound,
            6024 => MarginfiError::NoLiabilityFound,
            6025 => MarginfiError::InvalidOracleSetup,
            6026 => MarginfiError::IllegalUtilizationRatio,
            6027 => MarginfiError::BankLiabilityCapacityExceeded,
            6028 => MarginfiError::InvalidPrice,
            6029 => MarginfiError::IsolatedAccountIllegalState,
            6030 => MarginfiError::EmissionsAlreadySetup,
            6031 => MarginfiError::OracleNotSetup,
            6032 => MarginfiError::InvalidSwitchboardDecimalConversion,
            6033 => MarginfiError::CannotCloseOutstandingEmissions,
            6034 => MarginfiError::EmissionsUpdateError,
            6035 => MarginfiError::AccountDisabled,
            6036 => MarginfiError::AccountTempActiveBalanceLimitExceeded,
            6037 => MarginfiError::AccountInFlashloan,
            6038 => MarginfiError::IllegalFlashloan,
            6039 => MarginfiError::IllegalFlag,
            6040 => MarginfiError::IllegalBalanceState,
            6041 => MarginfiError::IllegalAccountAuthorityTransfer,
            6042 => MarginfiError::Unauthorized,
            6043 => MarginfiError::IllegalAction,
            6044 => MarginfiError::T22MintRequired,
            6045 => MarginfiError::InvalidFeeAta,
            6046 => MarginfiError::AddedStakedPoolManually,
            6047 => MarginfiError::AssetTagMismatch,
            6048 => MarginfiError::StakePoolValidationFailed,
            6049 => MarginfiError::SwitchboardStalePrice,
            6050 => MarginfiError::PythPushStalePrice,
            6051 => MarginfiError::WrongNumberOfOracleAccounts,
            6052 => MarginfiError::WrongOracleAccountKeys,
            6053 => MarginfiError::PythPushWrongAccountOwner,
            6054 => MarginfiError::StakedPythPushWrongAccountOwner,
            6055 => MarginfiError::PythPushMismatchedFeedId,
            6056 => MarginfiError::PythPushInsufficientVerificationLevel,
            6057 => MarginfiError::PythPushFeedIdMustBe32Bytes,
            6058 => MarginfiError::PythPushFeedIdNonHexCharacter,
            6059 => MarginfiError::SwitchboardWrongAccountOwner,
            6060 => MarginfiError::PythPushInvalidAccount,
            6061 => MarginfiError::SwitchboardInvalidAccount,
            6062 => MarginfiError::MathError,
            6063 => MarginfiError::InvalidEmissionsDestinationAccount,
            _ => MarginfiError::InternalLogicError,
        }
    }
}
