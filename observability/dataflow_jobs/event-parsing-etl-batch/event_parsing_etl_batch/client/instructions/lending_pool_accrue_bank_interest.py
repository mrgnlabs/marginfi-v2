from __future__ import annotations
import typing
from solders.pubkey import Pubkey
from solders.instruction import Instruction, AccountMeta
from ..program_id import PROGRAM_ID


class LendingPoolAccrueBankInterestAccounts(typing.TypedDict):
    marginfi_group: Pubkey
    bank: Pubkey


def lending_pool_accrue_bank_interest(
    accounts: LendingPoolAccrueBankInterestAccounts,
    program_id: Pubkey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> Instruction:
    keys: list[AccountMeta] = [
        AccountMeta(
            pubkey=accounts["marginfi_group"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["bank"], is_signer=False, is_writable=True),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"l\xc9\x1eW/Aa\xbc"
    encoded_args = b""
    data = identifier + encoded_args
    return Instruction(program_id, data, keys)
