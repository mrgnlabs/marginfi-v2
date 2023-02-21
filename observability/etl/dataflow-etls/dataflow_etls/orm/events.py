import re
import uuid
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Union, Optional
from anchorpy import Event, NamedInstruction
from solders.pubkey import Pubkey

from dataflow_etls.transaction_log_parser import InstructionWithLogs

# IDL event names
LENDING_ACCOUNT_DEPOSIT_EVENT = 'LendingAccountDepositEvent'
LENDING_ACCOUNT_WITHDRAW_EVENT = 'LendingAccountWithdrawEvent'
LENDING_ACCOUNT_BORROW_EVENT = 'LendingAccountBorrowEvent'
LENDING_ACCOUNT_REPAY_EVENT = 'LendingAccountRepayEvent'
MARGINFI_ACCOUNT_CREATE_EVENT = 'MarginfiAccountCreateEvent'
LENDING_POOL_BANK_ADD_EVENT = 'LendingPoolBankAddEvent'
LENDING_POOL_BANK_ACCRUE_INTEREST_EVENT = 'LendingPoolBankAccrueInterestEvent'
LENDING_POOL_HANDLE_BANKRUPTCY_EVENT = 'LendingPoolHandleBankruptcyEvent'


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
            "idl_version:INTEGER",
            "is_cpi:BOOLEAN",
            "timestamp:TIMESTAMP",
            "signature:STRING",
            "indexing_address:STRING",
        ]
    )

    id: str
    idl_version: int
    is_cpi: bool
    # call_stack: List[str]
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
            "marginfi_account_authority:STRING",
            "authority:STRING",
            "operation:STRING",
            "amount:BIGNUMERIC",
            "balance_closed:BOOLEAN"
        ]
    )

    EVENT_TYPES = [LENDING_ACCOUNT_REPAY_EVENT, LENDING_ACCOUNT_DEPOSIT_EVENT,
                   LENDING_ACCOUNT_BORROW_EVENT, LENDING_ACCOUNT_WITHDRAW_EVENT]
    INSTRUCTION_TYPES = [pascal_to_snake_case(string.removesuffix("Event")) for string in EVENT_TYPES]

    marginfi_group: Pubkey
    marginfi_account: Pubkey
    marginfi_account_authority: Pubkey
    authority: Pubkey
    operation: str
    amount: int
    balance_closed: Optional[bool]

    @staticmethod
    def from_event(event: Event, instruction: InstructionWithLogs,
                   _instruction_args: NamedInstruction) -> "LiquidityChangeRecord":
        balance_closed = None
        if event.name == LENDING_ACCOUNT_REPAY_EVENT or event.name == LENDING_ACCOUNT_WITHDRAW_EVENT:
            balance_closed = event.data.close_balance

        return LiquidityChangeRecord(id=str(uuid.uuid4()),
                                     created_at=time_str(),
                                     timestamp=time_str(instruction.timestamp),
                                     idl_version=instruction.idl_version,
                                     is_cpi=instruction.is_cpi,
                                     # call_stack=[str(pk) for pk in instruction.call_stack],
                                     signature=instruction.signature,
                                     indexing_address=str(instruction.message.program_id),
                                     operation=event.name,
                                     marginfi_account=event.data.header.marginfi_account,
                                     marginfi_account_authority=event.data.header.marginfi_account_authority,
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
            "marginfi_account_authority:STRING",
            "authority:STRING",
        ]
    )

    marginfi_group: Pubkey
    marginfi_account: Pubkey
    marginfi_account_authority: Pubkey
    authority: Pubkey

    @staticmethod
    def from_event(event: Event, instruction: InstructionWithLogs,
                   _instruction_args: NamedInstruction) -> "MarginfiAccountCreationRecord":
        return MarginfiAccountCreationRecord(id=str(uuid.uuid4()),
                                             created_at=time_str(),
                                             timestamp=time_str(instruction.timestamp),
                                             idl_version=instruction.idl_version,
                                             is_cpi=instruction.is_cpi,
                                             # call_stack=[str(pk) for pk in instruction.call_stack],
                                             signature=instruction.signature,
                                             indexing_address=str(instruction.message.program_id),
                                             marginfi_account=event.data.header.marginfi_account,
                                             marginfi_account_authority=event.data.header.marginfi_account_authority,
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
                                        idl_version=instruction.idl_version,
                                        is_cpi=instruction.is_cpi,
                                        # call_stack=[str(pk) for pk in instruction.call_stack],
                                        signature=instruction.signature,
                                        indexing_address=str(instruction.message.program_id),
                                        marginfi_group=event.data.header.marginfi_group,
                                        authority=event.data.header.signer,
                                        bank=event.data.bank,
                                        mint=event.data.mint)


@dataclass
class LendingPoolHandleBankruptcyRecord(RecordBase):
    NAME = "LendingPoolHandleBankruptcy"
    SCHEMA = RecordBase.SCHEMA + "," + ",".join(
        [
            "marginfi_group:STRING",
            "marginfi_account:STRING",
            "marginfi_account_authority:STRING",
            "bank:STRING",
            "mint:STRING",
            "authority:STRING",
            "bad_debt:BIGNUMERIC",
            "covered_amount:BIGNUMERIC",
            "socialized_amount:BIGNUMERIC",
        ]
    )

    marginfi_group: Pubkey
    marginfi_account: Pubkey
    marginfi_account_authority: Pubkey
    authority: Pubkey
    bank: Pubkey
    mint: Pubkey
    bad_debt: float
    covered_amount: float
    socialized_amount: float

    @staticmethod
    def from_event(event: Event, instruction: InstructionWithLogs,
                   _instruction_args: NamedInstruction) -> "LendingPoolHandleBankruptcyRecord":
        return LendingPoolHandleBankruptcyRecord(id=str(uuid.uuid4()),
                                                 created_at=time_str(),
                                                 timestamp=time_str(instruction.timestamp),
                                                 idl_version=instruction.idl_version,
                                                 is_cpi=instruction.is_cpi,
                                                 # call_stack=[str(pk) for pk in instruction.call_stack],
                                                 signature=instruction.signature,
                                                 indexing_address=str(instruction.message.program_id),
                                                 marginfi_group=event.data.header.marginfi_group,
                                                 marginfi_account=event.data.header.marginfi_account,
                                                 marginfi_account_authority=event.data.header.marginfi_account_authority,
                                                 authority=event.data.header.signer,
                                                 bank=event.data.bank,
                                                 mint=event.data.mint,
                                                 bad_debt=event.data.bad_debt,
                                                 covered_amount=event.data.covered_amount,
                                                 socialized_amount=event.data.socialized_amount)


@dataclass
class LendingPoolBankAccrueInterestRecord(RecordBase):
    NAME = "LendingPoolBankAccrueInterest"
    SCHEMA = RecordBase.SCHEMA + "," + ",".join(
        [
            "marginfi_group:STRING",
            "authority:STRING",
            "bank:STRING",
            "mint:STRING",
            "delta:BIGNUMERIC",
            "fees_collected:BIGNUMERIC",
            "insurance_collected:BIGNUMERIC",
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
                   _instruction_args: NamedInstruction) -> "LendingPoolBankAccrueInterestRecord":
        return LendingPoolBankAccrueInterestRecord(id=str(uuid.uuid4()),
                                                   created_at=time_str(),
                                                   timestamp=time_str(instruction.timestamp),
                                                   idl_version=instruction.idl_version,
                                                   is_cpi=instruction.is_cpi,
                                                   # call_stack=[str(pk) for pk in instruction.call_stack],
                                                   signature=instruction.signature,
                                                   indexing_address=str(instruction.message.program_id),
                                                   marginfi_group=event.data.header.marginfi_group,
                                                   authority=event.data.header.signer,
                                                   bank=event.data.bank,
                                                   mint=event.data.mint,
                                                   delta=event.data.delta,
                                                   fees_collected=event.data.fees_collected,
                                                   insurance_collected=event.data.insurance_collected)


Record = Union[
    LiquidityChangeRecord, MarginfiAccountCreationRecord, LendingPoolBankAddRecord, LendingPoolBankAccrueInterestRecord, LendingPoolHandleBankruptcyRecord]
