from __future__ import annotations
import typing
from solders.pubkey import Pubkey
from spl.token.constants import TOKEN_PROGRAM_ID
from solders.instruction import Instruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class LendingAccountDepositArgs(typing.TypedDict):
    amount: int


layout = borsh.CStruct("amount" / borsh.U64)


class LendingAccountDepositAccounts(typing.TypedDict):
    marginfi_group: Pubkey
    marginfi_account: Pubkey
    signer: Pubkey
    bank: Pubkey
    signer_token_account: Pubkey
    bank_liquidity_vault: Pubkey


def lending_account_deposit(
    args: LendingAccountDepositArgs,
    accounts: LendingAccountDepositAccounts,
    program_id: Pubkey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> Instruction:
    keys: list[AccountMeta] = [
        AccountMeta(
            pubkey=accounts["marginfi_group"], is_signer=False, is_writable=False
        ),
        AccountMeta(
            pubkey=accounts["marginfi_account"], is_signer=False, is_writable=True
        ),
        AccountMeta(pubkey=accounts["signer"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["bank"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["signer_token_account"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["bank_liquidity_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(pubkey=TOKEN_PROGRAM_ID, is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xab^\xebgR@\xd4\x8c"
    encoded_args = layout.build(
        {
            "amount": args["amount"],
        }
    )
    data = identifier + encoded_args
    return Instruction(program_id, data, keys)
