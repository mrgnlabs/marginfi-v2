from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
from solders.pubkey import Pubkey
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh


class GroupEventHeaderJSON(typing.TypedDict):
    version: str
    signer: typing.Optional[str]
    marginfi_group: str


@dataclass
class GroupEventHeader:
    layout: typing.ClassVar = borsh.CStruct(
        "version" / borsh.String,
        "signer" / borsh.Option(BorshPubkey),
        "marginfi_group" / BorshPubkey,
    )
    version: str
    signer: typing.Optional[Pubkey]
    marginfi_group: Pubkey

    @classmethod
    def from_decoded(cls, obj: Container) -> "GroupEventHeader":
        return cls(
            version=obj.version, signer=obj.signer, marginfi_group=obj.marginfi_group
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "version": self.version,
            "signer": self.signer,
            "marginfi_group": self.marginfi_group,
        }

    def to_json(self) -> GroupEventHeaderJSON:
        return {
            "version": self.version,
            "signer": (None if self.signer is None else str(self.signer)),
            "marginfi_group": str(self.marginfi_group),
        }

    @classmethod
    def from_json(cls, obj: GroupEventHeaderJSON) -> "GroupEventHeader":
        return cls(
            version=obj["version"],
            signer=(
                None if obj["signer"] is None else Pubkey.from_string(obj["signer"])
            ),
            marginfi_group=Pubkey.from_string(obj["marginfi_group"]),
        )
