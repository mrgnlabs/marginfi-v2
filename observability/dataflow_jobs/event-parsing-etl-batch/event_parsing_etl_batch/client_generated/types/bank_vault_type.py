from __future__ import annotations
import typing
from dataclasses import dataclass
from anchorpy.borsh_extension import EnumForCodegen
import borsh_construct as borsh


class LiquidityJSON(typing.TypedDict):
    kind: typing.Literal["Liquidity"]


class InsuranceJSON(typing.TypedDict):
    kind: typing.Literal["Insurance"]


class FeeJSON(typing.TypedDict):
    kind: typing.Literal["Fee"]


@dataclass
class Liquidity:
    discriminator: typing.ClassVar = 0
    kind: typing.ClassVar = "Liquidity"

    @classmethod
    def to_json(cls) -> LiquidityJSON:
        return LiquidityJSON(
            kind="Liquidity",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Liquidity": {},
        }


@dataclass
class Insurance:
    discriminator: typing.ClassVar = 1
    kind: typing.ClassVar = "Insurance"

    @classmethod
    def to_json(cls) -> InsuranceJSON:
        return InsuranceJSON(
            kind="Insurance",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Insurance": {},
        }


@dataclass
class Fee:
    discriminator: typing.ClassVar = 2
    kind: typing.ClassVar = "Fee"

    @classmethod
    def to_json(cls) -> FeeJSON:
        return FeeJSON(
            kind="Fee",
        )

    @classmethod
    def to_encodable(cls) -> dict:
        return {
            "Fee": {},
        }


BankVaultTypeKind = typing.Union[Liquidity, Insurance, Fee]
BankVaultTypeJSON = typing.Union[LiquidityJSON, InsuranceJSON, FeeJSON]


def from_decoded(obj: dict) -> BankVaultTypeKind:
    if not isinstance(obj, dict):
        raise ValueError("Invalid enum object")
    if "Liquidity" in obj:
        return Liquidity()
    if "Insurance" in obj:
        return Insurance()
    if "Fee" in obj:
        return Fee()
    raise ValueError("Invalid enum object")


def from_json(obj: BankVaultTypeJSON) -> BankVaultTypeKind:
    if obj["kind"] == "Liquidity":
        return Liquidity()
    if obj["kind"] == "Insurance":
        return Insurance()
    if obj["kind"] == "Fee":
        return Fee()
    kind = obj["kind"]
    raise ValueError(f"Unrecognized enum kind: {kind}")


layout = EnumForCodegen(
    "Liquidity" / borsh.CStruct(),
    "Insurance" / borsh.CStruct(),
    "Fee" / borsh.CStruct(),
)
