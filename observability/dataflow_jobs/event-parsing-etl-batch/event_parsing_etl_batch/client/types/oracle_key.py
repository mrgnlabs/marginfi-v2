from __future__ import annotations
import typing
from dataclasses import dataclass
from solders.pubkey import Pubkey
from anchorpy.borsh_extension import EnumForCodegen, BorshPubkey
import borsh_construct as borsh

PythJSONValue = tuple[str]
PythValue = tuple[Pubkey]


class PythJSON(typing.TypedDict):
    value: PythJSONValue
    kind: typing.Literal["Pyth"]


@dataclass
class Pyth:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Pyth"
    value: PythValue

    def to_json(self) -> PythJSON:
        return PythJSON(
            kind="Pyth",
            value=(str(self.value[0]),),
        )

    def to_encodable(self) -> dict:
        return {
            "Pyth": {
                "item_0": self.value[0],
            },
        }


OracleKeyKind = typing.Union[Pyth]
OracleKeyJSON = typing.Union[PythJSON]


def from_decoded(obj: dict) -> OracleKeyKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Pyth" in obj:
        val = obj["Pyth"]
        return Pyth((val["item_0"],))
    raise ValueError("Invalid enum object")


def from_json(obj: OracleKeyJSON) -> OracleKeyKind:
    if obj["kind"] == "Pyth":
        pyth_json_value = typing.cast(PythJSONValue, obj["value"])
        return Pyth((Pubkey.from_string(pyth_json_value[0]),))
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen("Pyth" / borsh.CStruct("item_0" / BorshPubkey))
