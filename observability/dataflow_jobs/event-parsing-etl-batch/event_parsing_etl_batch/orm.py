import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Union, Optional
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


def time_str(dt: Optional[datetime] = None) -> str:
    if dt is None:
        dt = datetime.now(timezone.utc)
    return dt.strftime("%Y-%m-%d %H:%M:%S %Z")


@dataclass
class RecordBase:
    SCHEMA = ",".join(
        [
            "id:STRING",
            "created_at:TIMESTAMP",
            "timestamp:TIMESTAMP",
            "signature:STRING",
            "indexing_address:STRING",
        ]
    )

    id: str
    created_at: str
    timestamp: str
    signature: str
    indexing_address: str


@dataclass
class LiquidityChangeRecord(RecordBase):
    NAME = "LiquidityChange"
    SCHEMA = RecordBase.SCHEMA + "," + ",".join(
        [
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
    def from_event(event: Event, instruction: InstructionWithLogs,
                   _instruction_args: NamedInstruction) -> "LiquidityChangeRecord":
        balance_closed = None
        if event.name == LENDING_ACCOUNT_REPAY_EVENT or event.name == LENDING_ACCOUNT_WITHDRAW_EVENT:
            balance_closed = event.data.close_balance

        return LiquidityChangeRecord(id=str(uuid.uuid4()),
                                     created_at=time_str(),
                                     timestamp=time_str(instruction.timestamp),
                                     signature=instruction.signature,
                                     indexing_address=str(instruction.message.program_id),
                                     operation=event.name,
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
class MarginfiAccountCreationRecord(RecordBase):
    NAME = "MarginfiAccountCreation"
    SCHEMA = RecordBase.SCHEMA + "," + ",".join(
        [
            "marginfi_group:STRING",
            "marginfi_account:STRING",
            "authority:STRING",
        ]
    )

    marginfi_group: Pubkey
    marginfi_account: Pubkey
    authority: Pubkey

    @staticmethod
    def from_event(event: Event, instruction: InstructionWithLogs,
                   _instruction_args: NamedInstruction) -> "MarginfiAccountCreationRecord":
        return MarginfiAccountCreationRecord(id=str(uuid.uuid4()),
                                             created_at=time_str(),
                                             timestamp=time_str(instruction.timestamp),
                                             signature=instruction.signature,
                                             indexing_address=str(instruction.message.program_id),
                                             marginfi_account=event.data.header.marginfi_account,
                                             marginfi_group=event.data.header.marginfi_group,
                                             authority=event.data.header.signer)


@dataclass
class LendingPoolBankAddRecord(RecordBase):
    NAME = "LendingPoolBankAdd"
    SCHEMA = RecordBase.SCHEMA + "," + ",".join(
        [
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
    def from_event(event: Event, instruction: InstructionWithLogs,
                   _instruction_args: NamedInstruction) -> "LendingPoolBankAddRecord":
        return LendingPoolBankAddRecord(id=str(uuid.uuid4()),
                                        created_at=time_str(),
                                        timestamp=time_str(instruction.timestamp),
                                        signature=instruction.signature,
                                        indexing_address=str(instruction.message.program_id),
                                        marginfi_group=event.data.header.marginfi_group,
                                        authority=event.data.header.signer,
                                        bank=event.data.bank,
                                        mint=event.data.mint)


@dataclass
class LendingPoolBankAccrueInterestRecord(RecordBase):
    NAME = "LendingPoolBankAccrueInterest"
    SCHEMA = RecordBase.SCHEMA + "," + ",".join(
        [
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

        return LendingPoolBankAccrueInterestRecord(id=str(uuid.uuid4()),
                                                   created_at=time_str(),
                                                   timestamp=time_str(instruction.timestamp),
                                                   signature=instruction.signature,
                                                   indexing_address=str(instruction.message.program_id),
                                                   marginfi_group=event.data.header.marginfi_group,
                                                   authority=event.data.header.signer,
                                                   bank=instruction.message.accounts[bank_account_index],
                                                   mint=event.data.mint,
                                                   delta=event.data.delta,
                                                   fees_collected=event.data.feesCollected,
                                                   insurance_collected=event.data.insuranceCollected)


Record = Union[
    LiquidityChangeRecord, MarginfiAccountCreationRecord, LendingPoolBankAddRecord, LendingPoolBankAccrueInterestRecord]
