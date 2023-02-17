from __future__ import annotations
from . import (
    balance,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class LendingAccountJSON(typing.TypedDict):
    balances: list[balance.BalanceJSON]
    padding: list[int]


@dataclass
class LendingAccount:
    layout: typing.ClassVar = borsh.CStruct(
        "balances" / balance.Balance.layout[16], "padding" / borsh.U64[8]
    )
    balances: list[balance.Balance]
    padding: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "LendingAccount":
        return cls(
            balances=list(
                map(lambda item: balance.Balance.from_decoded(item), obj.balances)
            ),
            padding=obj.padding,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "balances": list(map(lambda item: item.to_encodable(), self.balances)),
            "padding": self.padding,
        }

    def to_json(self) -> LendingAccountJSON:
        return {
            "balances": list(map(lambda item: item.to_json(), self.balances)),
            "padding": self.padding,
        }

    @classmethod
    def from_json(cls, obj: LendingAccountJSON) -> "LendingAccount":
        return cls(
            balances=list(
                map(lambda item: balance.Balance.from_json(item), obj["balances"])
            ),
            padding=obj["padding"],
        )
