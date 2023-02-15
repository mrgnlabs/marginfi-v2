from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class PausedJSON(typing.TypedDict):
    kind: typing.Literal["Paused"]


class OperationalJSON(typing.TypedDict):
    kind: typing.Literal["Operational"]


class ReduceOnlyJSON(typing.TypedDict):
    kind: typing.Literal["ReduceOnly"]


@dataclass
class Paused:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Paused"

    @classmethod
    def to_json(cls) -> PausedJSON:
        return PausedJSON(
            kind="Paused",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Paused": {},
        }


@dataclass
class Operational:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "Operational"

    @classmethod
    def to_json(cls) -> OperationalJSON:
        return OperationalJSON(
            kind="Operational",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Operational": {},
        }


@dataclass
class ReduceOnly:
    discriminator: typing.ClassVar = 2
    kind: typing.ClassVar = "ReduceOnly"

    @classmethod
    def to_json(cls) -> ReduceOnlyJSON:
        return ReduceOnlyJSON(
            kind="ReduceOnly",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "ReduceOnly": {},
        }


BankOperationalStateKind = typing.Union[Paused, Operational, ReduceOnly]
BankOperationalStateJSON = typing.Union[PausedJSON, OperationalJSON, ReduceOnlyJSON]


def from_decoded(obj: dict) -> BankOperationalStateKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Paused" in obj:
        return Paused()
    if "Operational" in obj:
        return Operational()
    if "ReduceOnly" in obj:
        return ReduceOnly()
    raise ValueError("Invalid enum object")


def from_json(obj: BankOperationalStateJSON) -> BankOperationalStateKind:
    if obj["kind"] == "Paused":
        return Paused()
    if obj["kind"] == "Operational":
        return Operational()
    if obj["kind"] == "ReduceOnly":
        return ReduceOnly()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen(
    "Paused" / borsh.CStruct(),
    "Operational" / borsh.CStruct(),
    "ReduceOnly" / borsh.CStruct(),
)
