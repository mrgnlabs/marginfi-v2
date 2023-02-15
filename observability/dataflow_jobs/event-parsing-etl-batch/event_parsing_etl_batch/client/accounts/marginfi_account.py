import typing
from dataclasses import dataclass
from solders.pubkey import Pubkey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Commitment
import borsh_construct as borsh
from anchorpy.coder.accounts import ACCOUNT_DISCRIMINATOR_SIZE
from anchorpy.error import AccountInvalidDiscriminator
from anchorpy.utils.rpc import get_multiple_accounts
from anchorpy.borsh_extension import BorshPubkey
from ..program_id import PROGRAM_ID
from .. import types


class MarginfiAccountJSON(typing.TypedDict):
    group: str
    authority: str
    lending_account: types.lending_account.LendingAccountJSON
    padding: list[int]


@dataclass
class MarginfiAccount:
    discriminator: typing.ClassVar = b"C\xb2\x82m~r\x1c*"
    layout: typing.ClassVar = borsh.CStruct(
        "group" / BorshPubkey,
        "authority" / BorshPubkey,
        "lending_account" / types.lending_account.LendingAccount.layout,
        "padding" / borsh.U64[64],
    )
    group: Pubkey
    authority: Pubkey
    lending_account: types.lending_account.LendingAccount
    padding: list[int]

    @classmethod
    async def fetch(
        cls,
        conn: AsyncClient,
        address: Pubkey,
        commitment: typing.Optional[Commitment] = None,
        program_id: Pubkey = PROGRAM_ID,
    ) -> typing.Optional["MarginfiAccount"]:
        resp = await conn.get_account_info(address, commitment=commitment)
        info = resp.value
        if info is None:
            return None
        if info.owner != program_id:
            raise ValueError("Account does not belong to this program")
        bytes_data = info.data
        return cls.decode(bytes_data)

    @classmethod
    async def fetch_multiple(
        cls,
        conn: AsyncClient,
        addresses: list[Pubkey],
        commitment: typing.Optional[Commitment] = None,
        program_id: Pubkey = PROGRAM_ID,
    ) -> typing.List[typing.Optional["MarginfiAccount"]]:
        infos = await get_multiple_accounts(conn, addresses, commitment=commitment)
        res: typing.List[typing.Optional["MarginfiAccount"]] = []
        for info in infos:
            if info is None:
                res.append(None)
                continue
            if info.account.owner != program_id:
                raise ValueError("Account does not belong to this program")
            res.append(cls.decode(info.account.data))
        return res

    @classmethod
    def decode(cls, data: bytes) -> "MarginfiAccount":
        if data[:ACCOUNT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise AccountInvalidDiscriminator(
                "The discriminator for this account is invalid"
            )
        dec = MarginfiAccount.layout.parse(data[ACCOUNT_DISCRIMINATOR_SIZE:])
        return cls(
            group=dec.group,
            authority=dec.authority,
            lending_account=types.lending_account.LendingAccount.from_decoded(
                dec.lending_account
            ),
            padding=dec.padding,
        )

    def to_json(self) -> MarginfiAccountJSON:
        return {
            "group": str(self.group),
            "authority": str(self.authority),
            "lending_account": self.lending_account.to_json(),
            "padding": self.padding,
        }

    @classmethod
    def from_json(cls, obj: MarginfiAccountJSON) -> "MarginfiAccount":
        return cls(
            group=Pubkey.from_string(obj["group"]),
            authority=Pubkey.from_string(obj["authority"]),
            lending_account=types.lending_account.LendingAccount.from_json(
                obj["lending_account"]
            ),
            padding=obj["padding"],
        )
