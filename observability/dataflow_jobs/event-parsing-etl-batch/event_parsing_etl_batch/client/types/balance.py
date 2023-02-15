from __future__ import annotations
from . import (
    wrapped_i80f48,
)
import typing
from dataclasses import dataclass
from construct import Container
from solders.pubkey import Pubkey
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh


class BalanceJSON(typing.TypedDict):
    active: bool
    bank_pk: str
    asset_shares: wrapped_i80f48.WrappedI80F48JSON
    liability_shares: wrapped_i80f48.WrappedI80F48JSON
    padding: list[int]


@dataclass
class Balance:
    layout: typing.ClassVar = borsh.CStruct(
        "active" / borsh.Bool,
        "bank_pk" / BorshPubkey,
        "asset_shares" / wrapped_i80f48.WrappedI80F48.layout,
        "liability_shares" / wrapped_i80f48.WrappedI80F48.layout,
        "padding" / borsh.U64[4],
    )
    active: bool
    bank_pk: Pubkey
    asset_shares: wrapped_i80f48.WrappedI80F48
    liability_shares: wrapped_i80f48.WrappedI80F48
    padding: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "Balance":
        return cls(
            active=obj.active,
            bank_pk=obj.bank_pk,
            asset_shares=wrapped_i80f48.WrappedI80F48.from_decoded(obj.asset_shares),
            liability_shares=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.liability_shares
            ),
            padding=obj.padding,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "active": self.active,
            "bank_pk": self.bank_pk,
            "asset_shares": self.asset_shares.to_encodable(),
            "liability_shares": self.liability_shares.to_encodable(),
            "padding": self.padding,
        }

    def to_json(self) -> BalanceJSON:
        return {
            "active": self.active,
            "bank_pk": str(self.bank_pk),
            "asset_shares": self.asset_shares.to_json(),
            "liability_shares": self.liability_shares.to_json(),
            "padding": self.padding,
        }

    @classmethod
    def from_json(cls, obj: BalanceJSON) -> "Balance":
        return cls(
            active=obj["active"],
            bank_pk=Pubkey.from_string(obj["bank_pk"]),
            asset_shares=wrapped_i80f48.WrappedI80F48.from_json(obj["asset_shares"]),
            liability_shares=wrapped_i80f48.WrappedI80F48.from_json(
                obj["liability_shares"]
            ),
            padding=obj["padding"],
        )
