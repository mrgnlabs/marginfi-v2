from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class AssetsJSON(typing.TypedDict):
    kind: typing.Literal["Assets"]


class LiabilitiesJSON(typing.TypedDict):
    kind: typing.Literal["Liabilities"]


@dataclass
class Assets:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Assets"

    @classmethod
    def to_json(cls) -> AssetsJSON:
        return AssetsJSON(
            kind="Assets",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Assets": {},
        }


@dataclass
class Liabilities:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "Liabilities"

    @classmethod
    def to_json(cls) -> LiabilitiesJSON:
        return LiabilitiesJSON(
            kind="Liabilities",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Liabilities": {},
        }


BalanceSideKind = typing.Union[Assets, Liabilities]
BalanceSideJSON = typing.Union[AssetsJSON, LiabilitiesJSON]


def from_decoded(obj: dict) -> BalanceSideKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Assets" in obj:
        return Assets()
    if "Liabilities" in obj:
        return Liabilities()
    raise ValueError("Invalid enum object")


def from_json(obj: BalanceSideJSON) -> BalanceSideKind:
    if obj["kind"] == "Assets":
        return Assets()
    if obj["kind"] == "Liabilities":
        return Liabilities()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen("Assets" / borsh.CStruct(), "Liabilities" / borsh.CStruct())
