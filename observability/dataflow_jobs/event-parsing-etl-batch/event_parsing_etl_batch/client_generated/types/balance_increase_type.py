from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class AnyJSON(typing.TypedDict):
    kind: typing.Literal["Any"]


class RepayOnlyJSON(typing.TypedDict):
    kind: typing.Literal["RepayOnly"]


class DepositOnlyJSON(typing.TypedDict):
    kind: typing.Literal["DepositOnly"]


@dataclass
class Any:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Any"

    @classmethod
    def to_json(cls) -> AnyJSON:
        return AnyJSON(
            kind="Any",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Any": {},
        }


@dataclass
class RepayOnly:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "RepayOnly"

    @classmethod
    def to_json(cls) -> RepayOnlyJSON:
        return RepayOnlyJSON(
            kind="RepayOnly",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "RepayOnly": {},
        }


@dataclass
class DepositOnly:
    discriminator: typing.ClassVar = 2
    kind: typing.ClassVar = "DepositOnly"

    @classmethod
    def to_json(cls) -> DepositOnlyJSON:
        return DepositOnlyJSON(
            kind="DepositOnly",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "DepositOnly": {},
        }


BalanceIncreaseTypeKind = typing.Union[Any, RepayOnly, DepositOnly]
BalanceIncreaseTypeJSON = typing.Union[AnyJSON, RepayOnlyJSON, DepositOnlyJSON]


def from_decoded(obj: dict) -> BalanceIncreaseTypeKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Any" in obj:
        return Any()
    if "RepayOnly" in obj:
        return RepayOnly()
    if "DepositOnly" in obj:
        return DepositOnly()
    raise ValueError("Invalid enum object")


def from_json(obj: BalanceIncreaseTypeJSON) -> BalanceIncreaseTypeKind:
    if obj["kind"] == "Any":
        return Any()
    if obj["kind"] == "RepayOnly":
        return RepayOnly()
    if obj["kind"] == "DepositOnly":
        return DepositOnly()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen(
    "Any" / borsh.CStruct(),
    "RepayOnly" / borsh.CStruct(),
    "DepositOnly" / borsh.CStruct(),
)
