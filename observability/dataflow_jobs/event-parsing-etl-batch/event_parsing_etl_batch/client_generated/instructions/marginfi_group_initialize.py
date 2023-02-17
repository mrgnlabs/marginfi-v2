from __future__ import annotations
import typing
from solders.pubkey import Pubkey
from solders.system_program import ID as SYS_PROGRAM_ID
from solders.instruction import Instruction, AccountMeta
from ..program_id import PROGRAM_ID


class MarginfiGroupInitializeAccounts(typing.TypedDict):
    marginfi_group: Pubkey
    admin: Pubkey


def marginfi_group_initialize(
    accounts: MarginfiGroupInitializeAccounts,
    program_id: Pubkey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> Instruction:
    keys: list[AccountMeta] = [
        AccountMeta(
            pubkey=accounts["marginfi_group"], is_signer=True, is_writable=True
        ),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=True),
        AccountMeta(pubkey=SYS_PROGRAM_ID, is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b'\xffCC\x1a^\x1f"\x14'
    encoded_args = b""
    data = identifier + encoded_args
    return Instruction(program_id, data, keys)
