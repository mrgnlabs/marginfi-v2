from __future__ import annotations
import typing
from solders.pubkey import Pubkey
from solders.instruction import Instruction, AccountMeta
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class LendingPoolConfigureBankArgs(typing.TypedDict):
    bank_config_opt: types.bank_config_opt.BankConfigOpt


layout = borsh.CStruct("bank_config_opt" / types.bank_config_opt.BankConfigOpt.layout)


class LendingPoolConfigureBankAccounts(typing.TypedDict):
    marginfi_group: Pubkey
    admin: Pubkey
    bank: Pubkey


def lending_pool_configure_bank(
    args: LendingPoolConfigureBankArgs,
    accounts: LendingPoolConfigureBankAccounts,
    program_id: Pubkey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> Instruction:
    keys: list[AccountMeta] = [
        AccountMeta(
            pubkey=accounts["marginfi_group"], is_signer=False, is_writable=False
        ),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
        AccountMeta(pubkey=accounts["bank"], is_signer=False, is_writable=True),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b"y\xad\x9c(]\x948\xed"
    encoded_args = layout.build(
        {
            "bank_config_opt": args["bank_config_opt"].to_encodable(),
        }
    )
    data = identifier + encoded_args
    return Instruction(program_id, data, keys)
