from __future__ import annotations
import typing
from solders.pubkey import Pubkey
from solders.system_program import ID as SYS_PROGRAM_ID
from solders.instruction import Instruction, AccountMeta
from ..program_id import PROGRAM_ID


class MarginfiAccountInitializeAccounts(typing.TypedDict):
    marginfi_group: Pubkey
    marginfi_account: Pubkey
    authority: Pubkey
    fee_payer: Pubkey


def marginfi_account_initialize(
    accounts: MarginfiAccountInitializeAccounts,
    program_id: Pubkey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> Instruction:
    keys: list[AccountMeta] = [
        AccountMeta(
            pubkey=accounts["marginfi_group"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["marginfi_account"], is_signer=True, is_writable=True
        ),
        AccountMeta(pubkey=accounts["authority"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["fee_payer"], is_signer=True, is_writable=True),
        AccountMeta(pubkey=SYS_PROGRAM_ID, is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"+N=\xff\x944\xf9\x9a"
    encoded_args = b""
    data = identifier + encoded_args
    return Instruction(program_id, data, keys)
