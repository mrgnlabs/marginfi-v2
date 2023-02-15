from __future__ import annotations
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class WrappedI80F48JSON(typing.TypedDict):
    value: int


@dataclass
class WrappedI80F48:
    layout: typing.ClassVar = borsh.CStruct("value" / borsh.I128)
    value: int

    @classmethod
    def from_decoded(cls, obj: Container) -> "WrappedI80F48":
        return cls(value=obj.value)

    def to_encodable(self) -> dict[str, typing.Any]:
        return {"value": self.value}

    def to_json(self) -> WrappedI80F48JSON:
        return {"value": self.value}

    @classmethod
    def from_json(cls, obj: WrappedI80F48JSON) -> "WrappedI80F48":
        return cls(value=obj["value"])
