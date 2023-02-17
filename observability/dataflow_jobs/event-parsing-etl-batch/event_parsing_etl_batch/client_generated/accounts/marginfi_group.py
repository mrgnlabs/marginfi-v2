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


class MarginfiGroupJSON(typing.TypedDict):
    admin: str
    padding0: list[int]
    padding1: list[int]


@dataclass
class MarginfiGroup:
    discriminator: typing.ClassVar = b"\xb6\x17\xad\xf0\x97\xce\xb6C"
    layout: typing.ClassVar = borsh.CStruct(
        "admin" / BorshPubkey, "padding0" / borsh.U128[32], "padding1" / borsh.U128[32]
    )
    admin: Pubkey
    padding0: list[int]
    padding1: list[int]

    @classmethod
    async def fetch(
        cls,
        conn: AsyncClient,
        address: Pubkey,
        commitment: typing.Optional[Commitment] = None,
        program_id: Pubkey = PROGRAM_ID,
    ) -> typing.Optional["MarginfiGroup"]:
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
    ) -> typing.List[typing.Optional["MarginfiGroup"]]:
        infos = await get_multiple_accounts(conn, addresses, commitment=commitment)
        res: typing.List[typing.Optional["MarginfiGroup"]] = []
        for info in infos:
            if info is None:
                res.append(None)
                continue
            if info.account.owner != program_id:
                raise ValueError("Account does not belong to this program")
            res.append(cls.decode(info.account.data))
        return res

    @classmethod
    def decode(cls, data: bytes) -> "MarginfiGroup":
        if data[:ACCOUNT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise AccountInvalidDiscriminator(
                "The discriminator for this account is invalid"
            )
        dec = MarginfiGroup.layout.parse(data[ACCOUNT_DISCRIMINATOR_SIZE:])
        return cls(
            admin=dec.admin,
            padding0=dec.padding0,
            padding1=dec.padding1,
        )

    def to_json(self) -> MarginfiGroupJSON:
        return {
            "admin": str(self.admin),
            "padding0": self.padding0,
            "padding1": self.padding1,
        }

    @classmethod
    def from_json(cls, obj: MarginfiGroupJSON) -> "MarginfiGroup":
        return cls(
            admin=Pubkey.from_string(obj["admin"]),
            padding0=obj["padding0"],
            padding1=obj["padding1"],
        )
