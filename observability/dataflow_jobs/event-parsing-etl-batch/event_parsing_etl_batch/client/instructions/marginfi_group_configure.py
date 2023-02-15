from __future__ import annotations
import typing
from solders.pubkey import Pubkey
from solders.instruction import Instruction, AccountMeta
import borsh_construct as borsh
from .. import types
from ..program_id import PROGRAM_ID


class MarginfiGroupConfigureArgs(typing.TypedDict):
    config: types.group_config.GroupConfig


layout = borsh.CStruct("config" / types.group_config.GroupConfig.layout)


class MarginfiGroupConfigureAccounts(typing.TypedDict):
    marginfi_group: Pubkey
    admin: Pubkey


def marginfi_group_configure(
    args: MarginfiGroupConfigureArgs,
    accounts: MarginfiGroupConfigureAccounts,
    program_id: Pubkey = PROGRAM_ID,
    remaining_accounts: typing.Optional[typing.List[AccountMeta]] = None,
) -> Instruction:
    keys: list[AccountMeta] = [
        AccountMeta(
            pubkey=accounts["marginfi_group"], is_signer=False, is_writable=True
        ),
        AccountMeta(pubkey=accounts["admin"], is_signer=True, is_writable=False),
    ]
    if remaining_accounts is not None:
        keys += remaining_accounts
    identifier = b">\xc7QN!\r\xec="
    encoded_args = layout.build(
        {
            "config": args["config"].to_encodable(),
        }
    )
    data = identifier + encoded_args
    return Instruction(program_id, data, keys)
