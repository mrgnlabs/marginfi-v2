from dataclasses import dataclass
from typing import Union, Optional
from anchorpy import Event

from solders.pubkey import Pubkey

LENDING_ACCOUNT_DEPOSIT_EVENT = 'LendingAccountDepositEvent'
LENDING_ACCOUNT_WITHDRAW_EVENT = 'LendingAccountWithdrawEvent'
LENDING_ACCOUNT_BORROW_EVENT = 'LendingAccountBorrowEvent'
LENDING_ACCOUNT_REPAY_EVENT = 'LendingAccountRepayEvent'
MARGINFI_ACCOUNT_CREATE_EVENT = 'MarginfiAccountCreateEvent'


@dataclass
class LiquidityChangeRecord:
    NAME = "LiquidityChange"

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

    marginfi_group: Pubkey
    marginfi_account: Pubkey
    authority: Pubkey

    @staticmethod
    def from_event(event: Event) -> "MarginfiAccountCreationRecord":
        return MarginfiAccountCreationRecord(marginfi_account=event.data.header.marginfi_account,
                                             marginfi_group=event.data.header.marginfi_group,
                                             authority=event.data.header.signer)


Record = Union[LiquidityChangeRecord, MarginfiAccountCreationRecord]


def event_to_record(event: Event) -> Optional[Record]:
    if is_liquidity_change_event(event.name):
        return LiquidityChangeRecord.from_event(event)
    elif event.name == MARGINFI_ACCOUNT_CREATE_EVENT:
        return MarginfiAccountCreationRecord.from_event(event)
    else:
        print("discarding unsupported event:", event.name)
        return None
