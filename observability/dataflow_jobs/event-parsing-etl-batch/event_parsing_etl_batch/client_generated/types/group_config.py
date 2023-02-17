from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
from solders.pubkey import Pubkey
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh


class GroupConfigJSON(typing.TypedDict):
    admin: typing.Optional[str]


@dataclass
class GroupConfig:
    layout: typing.ClassVar = borsh.CStruct("admin" / borsh.Option(BorshPubkey))
    admin: typing.Optional[Pubkey]

    @classmethod
    def from_decoded(cls, obj: Container) -> "GroupConfig":
        return cls(admin=obj.admin)

    def to_encodable(self) -> dict[str, typing.Any]:
        return {"admin": self.admin}

    def to_json(self) -> GroupConfigJSON:
        return {"admin": (None if self.admin is None else str(self.admin))}

    @classmethod
    def from_json(cls, obj: GroupConfigJSON) -> "GroupConfig":
        return cls(
            admin=(None if obj["admin"] is None else Pubkey.from_string(obj["admin"]))
        )
