from __future__ import annotations
import typing
from solders.pubkey import Pubkey
from spl.token.constants import TOKEN_PROGRAM_ID
from solders.instruction import Instruction, AccountMeta
from ..program_id import PROGRAM_ID


class LendingPoolCollectBankFeesAccounts(typing.TypedDict):
    marginfi_group: Pubkey
    bank: Pubkey
    liquidity_vault_authority: Pubkey
    liquidity_vault: Pubkey
    insurance_vault: Pubkey
    fee_vault: Pubkey


def lending_pool_collect_bank_fees(
    accounts: LendingPoolCollectBankFeesAccounts,
    program_id: Pubkey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> Instruction:
    keys: list[AccountMeta] = [
        AccountMeta(
            pubkey=accounts["marginfi_group"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["bank"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["liquidity_vault_authority"],
            is_signer=False,
            is_writable=False,
        ),
        AccountMeta(
            pubkey=accounts["liquidity_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["insurance_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(pubkey=accounts["fee_vault"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=TOKEN_PROGRAM_ID, is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xc9\x05\xd7t\xe6\\K\x96"
    encoded_args = b""
    data = identifier + encoded_args
    return Instruction(program_id, data, keys)
