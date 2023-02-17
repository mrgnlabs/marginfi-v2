from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class InitialJSON(typing.TypedDict):
    kind: typing.Literal["Initial"]


class MaintenanceJSON(typing.TypedDict):
    kind: typing.Literal["Maintenance"]


@dataclass
class Initial:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Initial"

    @classmethod
    def to_json(cls) -> InitialJSON:
        return InitialJSON(
            kind="Initial",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Initial": {},
        }


@dataclass
class Maintenance:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "Maintenance"

    @classmethod
    def to_json(cls) -> MaintenanceJSON:
        return MaintenanceJSON(
            kind="Maintenance",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Maintenance": {},
        }


WeightTypeKind = typing.Union[Initial, Maintenance]
WeightTypeJSON = typing.Union[InitialJSON, MaintenanceJSON]


def from_decoded(obj: dict) -> WeightTypeKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Initial" in obj:
        return Initial()
    if "Maintenance" in obj:
        return Maintenance()
    raise ValueError("Invalid enum object")


def from_json(obj: WeightTypeJSON) -> WeightTypeKind:
    if obj["kind"] == "Initial":
        return Initial()
    if obj["kind"] == "Maintenance":
        return Maintenance()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen("Initial" / borsh.CStruct(), "Maintenance" / borsh.CStruct())
