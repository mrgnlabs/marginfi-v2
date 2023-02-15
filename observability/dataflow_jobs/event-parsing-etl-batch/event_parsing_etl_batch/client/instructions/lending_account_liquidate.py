from __future__ import annotations
import typing
from solders.pubkey import Pubkey
from spl.token.constants import TOKEN_PROGRAM_ID
from solders.instruction import Instruction, AccountMeta
import borsh_construct as borsh
from ..program_id import PROGRAM_ID


class LendingAccountLiquidateArgs(typing.TypedDict):
    asset_amount: int


layout = borsh.CStruct("asset_amount" / borsh.U64)


class LendingAccountLiquidateAccounts(typing.TypedDict):
    marginfi_group: Pubkey
    asset_bank: Pubkey
    liab_bank: Pubkey
    liquidator_marginfi_account: Pubkey
    signer: Pubkey
    liquidatee_marginfi_account: Pubkey
    bank_liquidity_vault_authority: Pubkey
    bank_liquidity_vault: Pubkey
    bank_insurance_vault: Pubkey


def lending_account_liquidate(
    args: LendingAccountLiquidateArgs,
    accounts: LendingAccountLiquidateAccounts,
    program_id: Pubkey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> Instruction:
    keys: list[AccountMeta] = [
        AccountMeta(
            pubkey=accounts["marginfi_group"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["asset_bank"], is_signer=False, is_writable=True),
        AccountMeta(pubkey=accounts["liab_bank"], is_signer=False, is_writable=True),
        AccountMeta(
            pubkey=accounts["liquidator_marginfi_account"],
            is_signer=False,
            is_writable=True,
        ),
        AccountMeta(pubkey=accounts["signer"], is_signer=True, is_writable=False),
        AccountMeta(
            pubkey=accounts["liquidatee_marginfi_account"],
            is_signer=False,
            is_writable=True,
        ),
        AccountMeta(
            pubkey=accounts["bank_liquidity_vault_authority"],
            is_signer=False,
            is_writable=True,
        ),
        AccountMeta(
            pubkey=accounts["bank_liquidity_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(
            pubkey=accounts["bank_insurance_vault"], is_signer=False, is_writable=True
        ),
        AccountMeta(pubkey=TOKEN_PROGRAM_ID, is_signer=False, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"\xd6\xa9\x97\xd5\xfb\xa7V\xdb"
    encoded_args = layout.build(
        {
            "asset_amount": args["asset_amount"],
        }
    )
    data = identifier + encoded_args
    return Instruction(program_id, data, keys)
