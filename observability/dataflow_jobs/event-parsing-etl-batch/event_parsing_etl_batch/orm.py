from dataclasses import dataclass
from typing import Union
from anchorpy import Event, NamedInstruction
from solders.pubkey import Pubkey

from event_parsing_etl_batch.transaction_log_parser import InstructionWithLogs

# IDL event names
LENDING_ACCOUNT_DEPOSIT_EVENT = 'LendingAccountDepositEvent'
LENDING_ACCOUNT_WITHDRAW_EVENT = 'LendingAccountWithdrawEvent'
LENDING_ACCOUNT_BORROW_EVENT = 'LendingAccountBorrowEvent'
LENDING_ACCOUNT_REPAY_EVENT = 'LendingAccountRepayEvent'
MARGINFI_ACCOUNT_CREATE_EVENT = 'MarginfiAccountCreateEvent'
LENDING_POOL_BANK_ADD_EVENT = 'LendingPoolBankAddEvent'
LENDING_POOL_BANK_ACCRUE_INTEREST_EVENT = 'LendingPoolBankAccrueInterestEvent'


@dataclass
class LiquidityChangeRecord:
    NAME = "LiquidityChange"
    SCHEMA = ",".join(
        [
            "id:STRING",
            "timestamp:TIMESTAMP",
            "signature:STRING",
            "indexing_address:STRING",
            "marginfi_group:STRING",
            "marginfi_account:STRING",
            "authority:STRING",
            "operation:STRING",
            "amount:BIGNUMERIC",
            "balance_closed:BOOLEAN"
        ]
    )

    marginfi_group: Pubkey
    marginfi_account: Pubkey
    authority: Pubkey
    operation: str
    amount: int
    balance_closed: bool

    @staticmethod
    def from_event(event: Event) -> "LiquidityChangeRecord":
        balance_closed = None
        if event.name == LENDING_ACCOUNT_REPAY_EVENT or event.name == LENDING_ACCOUNT_WITHDRAW_EVENT:
            balance_closed = event.data.close_balance

        return LiquidityChangeRecord(operation=event.name,
                                     marginfi_account=event.data.header.marginfi_account,
                                     marginfi_group=event.data.header.marginfi_group,
                                     authority=event.data.header.signer,
                                     amount=event.data.amount,
                                     balance_closed=balance_closed)


def is_liquidity_change_event(event_name: str) -> bool:
    return event_name in [
        LENDING_ACCOUNT_DEPOSIT_EVENT,
        LENDING_ACCOUNT_WITHDRAW_EVENT,
        LENDING_ACCOUNT_BORROW_EVENT,
        LENDING_ACCOUNT_REPAY_EVENT,
    ]


@dataclass
class MarginfiAccountCreationRecord:
    NAME = "MarginfiAccountCreation"
    SCHEMA = ",".join(
        [
            "id:STRING",
            "timestamp:TIMESTAMP",
            "signature:STRING",
            "indexing_address:STRING",
            "marginfi_group:STRING",
            "marginfi_account:STRING",
            "authority:STRING",
        ]
    )

    marginfi_group: Pubkey
    marginfi_account: Pubkey
    authority: Pubkey

    @staticmethod
    def from_event(event: Event) -> "MarginfiAccountCreationRecord":
        return MarginfiAccountCreationRecord(marginfi_account=event.data.header.marginfi_account,
                                             marginfi_group=event.data.header.marginfi_group,
                                             authority=event.data.header.signer)


@dataclass
class LendingPoolBankAddRecord:
    NAME = "LendingPoolBankAdd"
    SCHEMA = ",".join(
        [
            "id:STRING",
            "timestamp:TIMESTAMP",
            "signature:STRING",
            "indexing_address:STRING",
            "marginfi_group:STRING",
            "bank:STRING",
            "mint:STRING",
            "authority:STRING",
        ]
    )

    marginfi_group: Pubkey
    bank: Pubkey
    mint: Pubkey
    authority: Pubkey

    @staticmethod
    def from_event(event: Event) -> "LendingPoolBankAddRecord":
        return LendingPoolBankAddRecord(marginfi_group=event.data.header.marginfi_group,
                                        authority=event.data.header.signer,
                                        bank=event.data.bank,
                                        mint=event.data.mint)


@dataclass
class LendingPoolBankAccrueInterestRecord:
    NAME = "LendingPoolBankAccrueInterest"
    SCHEMA = ",".join(
        [
            "id:STRING",
            "timestamp:TIMESTAMP",
            "signature:STRING",
            "indexing_address:STRING",
            "marginfi_group:STRING",
            "authority:STRING",
            "bank:STRING",
            "mint:STRING",
            "delta:BIGDECIMAL",
            "fees_collected:BIGDECIMAL",
            "insurance_collected:BIGDECIMAL",
        ]
    )

    marginfi_group: Pubkey
    authority: Pubkey
    bank: Pubkey
    mint: Pubkey
    delta: int
    fees_collected: float
    insurance_collected: float

    @staticmethod
    def from_event(event: Event, instruction: InstructionWithLogs,
                   instruction_args: NamedInstruction) -> "LendingPoolBankAccrueInterestRecord":
        if instruction_args.name != LendingPoolBankAccrueInterestRecord.NAME:
            print(f"error: event should be logged in ix type {LendingPoolBankAccrueInterestRecord.NAME}")

        bank_account_index = 1

        return LendingPoolBankAccrueInterestRecord(marginfi_group=event.data.header.marginfi_group,
                                                   authority=event.data.header.signer,
                                                   bank=instruction.message.accounts[bank_account_index],
                                                   mint=event.data.mint,
                                                   delta=event.data.delta,
                                                   fees_collected=event.data.feesCollected,
                                                   insurance_collected=event.data.insuranceCollected)


Record = Union[
    LiquidityChangeRecord, MarginfiAccountCreationRecord, LendingPoolBankAddRecord, LendingPoolBankAccrueInterestRecord]
