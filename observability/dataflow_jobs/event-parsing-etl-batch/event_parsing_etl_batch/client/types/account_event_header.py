from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
from solders.pubkey import Pubkey
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh


class AccountEventHeaderJSON(typing.TypedDict):
    version: str
    signer: str
    marginfi_account: str
    marginfi_group: str


@dataclass
class AccountEventHeader:
    layout: typing.ClassVar = borsh.CStruct(
        "version" / borsh.String,
        "signer" / BorshPubkey,
        "marginfi_account" / BorshPubkey,
        "marginfi_group" / BorshPubkey,
    )
    version: str
    signer: Pubkey
    marginfi_account: Pubkey
    marginfi_group: Pubkey

    @classmethod
    def from_decoded(cls, obj: Container) -> "AccountEventHeader":
        return cls(
            version=obj.version,
            signer=obj.signer,
            marginfi_account=obj.marginfi_account,
            marginfi_group=obj.marginfi_group,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "version": self.version,
            "signer": self.signer,
            "marginfi_account": self.marginfi_account,
            "marginfi_group": self.marginfi_group,
        }

    def to_json(self) -> AccountEventHeaderJSON:
        return {
            "version": self.version,
            "signer": str(self.signer),
            "marginfi_account": str(self.marginfi_account),
            "marginfi_group": str(self.marginfi_group),
        }

    @classmethod
    def from_json(cls, obj: AccountEventHeaderJSON) -> "AccountEventHeader":
        return cls(
            version=obj["version"],
            signer=Pubkey.from_string(obj["signer"]),
            marginfi_account=Pubkey.from_string(obj["marginfi_account"]),
            marginfi_group=Pubkey.from_string(obj["marginfi_group"]),
        )
