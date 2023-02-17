from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class AnyJSON(typing.TypedDict):
    kind: typing.Literal["Any"]


class WithdrawOnlyJSON(typing.TypedDict):
    kind: typing.Literal["WithdrawOnly"]


class BorrowOnlyJSON(typing.TypedDict):
    kind: typing.Literal["BorrowOnly"]


class BypassBorrowLimitJSON(typing.TypedDict):
    kind: typing.Literal["BypassBorrowLimit"]


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
class WithdrawOnly:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "WithdrawOnly"

    @classmethod
    def to_json(cls) -> WithdrawOnlyJSON:
        return WithdrawOnlyJSON(
            kind="WithdrawOnly",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "WithdrawOnly": {},
        }


@dataclass
class BorrowOnly:
    discriminator: typing.ClassVar = 2
    kind: typing.ClassVar = "BorrowOnly"

    @classmethod
    def to_json(cls) -> BorrowOnlyJSON:
        return BorrowOnlyJSON(
            kind="BorrowOnly",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "BorrowOnly": {},
        }


@dataclass
class BypassBorrowLimit:
    discriminator: typing.ClassVar = 3
    kind: typing.ClassVar = "BypassBorrowLimit"

    @classmethod
    def to_json(cls) -> BypassBorrowLimitJSON:
        return BypassBorrowLimitJSON(
            kind="BypassBorrowLimit",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "BypassBorrowLimit": {},
        }


BalanceDecreaseTypeKind = typing.Union[Any, WithdrawOnly, BorrowOnly, BypassBorrowLimit]
BalanceDecreaseTypeJSON = typing.Union[
    AnyJSON, WithdrawOnlyJSON, BorrowOnlyJSON, BypassBorrowLimitJSON
]


def from_decoded(obj: dict) -> BalanceDecreaseTypeKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Any" in obj:
        return Any()
    if "WithdrawOnly" in obj:
        return WithdrawOnly()
    if "BorrowOnly" in obj:
        return BorrowOnly()
    if "BypassBorrowLimit" in obj:
        return BypassBorrowLimit()
    raise ValueError("Invalid enum object")


def from_json(obj: BalanceDecreaseTypeJSON) -> BalanceDecreaseTypeKind:
    if obj["kind"] == "Any":
        return Any()
    if obj["kind"] == "WithdrawOnly":
        return WithdrawOnly()
    if obj["kind"] == "BorrowOnly":
        return BorrowOnly()
    if obj["kind"] == "BypassBorrowLimit":
        return BypassBorrowLimit()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen(
    "Any" / borsh.CStruct(),
    "WithdrawOnly" / borsh.CStruct(),
    "BorrowOnly" / borsh.CStruct(),
    "BypassBorrowLimit" / borsh.CStruct(),
)
