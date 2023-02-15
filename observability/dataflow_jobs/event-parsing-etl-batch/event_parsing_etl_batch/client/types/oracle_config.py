from __future__ import annotations
from . import (
    oracle_setup,
)
import typing
from dataclasses import dataclass
from construct import Container
from solders.pubkey import Pubkey
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh


class OracleConfigJSON(typing.TypedDict):
    setup: oracle_setup.OracleSetupJSON
    keys: list[str]


@dataclass
class OracleConfig:
    layout: typing.ClassVar = borsh.CStruct(
        "setup" / oracle_setup.layout, "keys" / BorshPubkey[5]
    )
    setup: oracle_setup.OracleSetupKind
    keys: list[Pubkey]

    @classmethod
    def from_decoded(cls, obj: Container) -> "OracleConfig":
        return cls(setup=oracle_setup.from_decoded(obj.setup), keys=obj.keys)

    def to_encodable(self) -> dict[str, typing.Any]:
        return {"setup": self.setup.to_encodable(), "keys": self.keys}

    def to_json(self) -> OracleConfigJSON:
        return {
            "setup": self.setup.to_json(),
            "keys": list(map(lambda item: str(item), self.keys)),
        }

    @classmethod
    def from_json(cls, obj: OracleConfigJSON) -> "OracleConfig":
        return cls(
            setup=oracle_setup.from_json(obj["setup"]),
            keys=list(map(lambda item: Pubkey.from_string(item), obj["keys"])),
        )
