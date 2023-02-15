from __future__ import annotations
import typing
from solders.pubkey import Pubkey
from spl.token.constants import TOKEN_PROGRAM_ID
from solders.instruction import Instruction, AccountMeta
from ..program_id import PROGRAM_ID


class LendingPoolHandleBankruptcyAccounts(typing.TypedDict):
    marginfi_group: Pubkey
    admin: Pubkey
    bank: Pubkey
    marginfi_account: Pubkey
    liquidity_vault: Pubkey
    insurance_vault: Pubkey
    insurance_vault_authority: Pubkey


def lending_pool_handle_bankruptcy(
    accounts: LendingPoolHandleBankruptcyAccounts,
    program_id: Pubkey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> Instruction:
    keys: list[AccountMeta] = [
        AccountMeta(
            pubkey=accounts["marginfi_group"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["bank"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["marginfi_account"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["liquidity_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["insurance_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["insurance_vault_authority"],
            is_signer=False,
            is_writable=False,
        ),
        AccountMeta(pubkey=TOKEN_PROGRAM_ID, is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xa2\x0b8\x8bZ\x80F\xad"
    encoded_args = b""
    data = identifier + encoded_args
    return Instruction(program_id, data, keys)
