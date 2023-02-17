from __future__ import annotations
from . import (
    wrapped_i80f48,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class InterestRateConfigJSON(typing.TypedDict):
    optimal_utilization_rate: wrapped_i80f48.WrappedI80F48JSON
    plateau_interest_rate: wrapped_i80f48.WrappedI80F48JSON
    max_interest_rate: wrapped_i80f48.WrappedI80F48JSON
    insurance_fee_fixed_apr: wrapped_i80f48.WrappedI80F48JSON
    insurance_ir_fee: wrapped_i80f48.WrappedI80F48JSON
    protocol_fixed_fee_apr: wrapped_i80f48.WrappedI80F48JSON
    protocol_ir_fee: wrapped_i80f48.WrappedI80F48JSON
    padding: list[int]


@dataclass
class InterestRateConfig:
    layout: typing.ClassVar = borsh.CStruct(
        "optimal_utilization_rate" / wrapped_i80f48.WrappedI80F48.layout,
        "plateau_interest_rate" / wrapped_i80f48.WrappedI80F48.layout,
        "max_interest_rate" / wrapped_i80f48.WrappedI80F48.layout,
        "insurance_fee_fixed_apr" / wrapped_i80f48.WrappedI80F48.layout,
        "insurance_ir_fee" / wrapped_i80f48.WrappedI80F48.layout,
        "protocol_fixed_fee_apr" / wrapped_i80f48.WrappedI80F48.layout,
        "protocol_ir_fee" / wrapped_i80f48.WrappedI80F48.layout,
        "padding" / borsh.U128[8],
    )
    optimal_utilization_rate: wrapped_i80f48.WrappedI80F48
    plateau_interest_rate: wrapped_i80f48.WrappedI80F48
    max_interest_rate: wrapped_i80f48.WrappedI80F48
    insurance_fee_fixed_apr: wrapped_i80f48.WrappedI80F48
    insurance_ir_fee: wrapped_i80f48.WrappedI80F48
    protocol_fixed_fee_apr: wrapped_i80f48.WrappedI80F48
    protocol_ir_fee: wrapped_i80f48.WrappedI80F48
    padding: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "InterestRateConfig":
        return cls(
            optimal_utilization_rate=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.optimal_utilization_rate
            ),
            plateau_interest_rate=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.plateau_interest_rate
            ),
            max_interest_rate=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.max_interest_rate
            ),
            insurance_fee_fixed_apr=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.insurance_fee_fixed_apr
            ),
            insurance_ir_fee=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.insurance_ir_fee
            ),
            protocol_fixed_fee_apr=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.protocol_fixed_fee_apr
            ),
            protocol_ir_fee=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.protocol_ir_fee
            ),
            padding=obj.padding,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "optimal_utilization_rate": self.optimal_utilization_rate.to_encodable(),
            "plateau_interest_rate": self.plateau_interest_rate.to_encodable(),
            "max_interest_rate": self.max_interest_rate.to_encodable(),
            "insurance_fee_fixed_apr": self.insurance_fee_fixed_apr.to_encodable(),
            "insurance_ir_fee": self.insurance_ir_fee.to_encodable(),
            "protocol_fixed_fee_apr": self.protocol_fixed_fee_apr.to_encodable(),
            "protocol_ir_fee": self.protocol_ir_fee.to_encodable(),
            "padding": self.padding,
        }

    def to_json(self) -> InterestRateConfigJSON:
        return {
            "optimal_utilization_rate": self.optimal_utilization_rate.to_json(),
            "plateau_interest_rate": self.plateau_interest_rate.to_json(),
            "max_interest_rate": self.max_interest_rate.to_json(),
            "insurance_fee_fixed_apr": self.insurance_fee_fixed_apr.to_json(),
            "insurance_ir_fee": self.insurance_ir_fee.to_json(),
            "protocol_fixed_fee_apr": self.protocol_fixed_fee_apr.to_json(),
            "protocol_ir_fee": self.protocol_ir_fee.to_json(),
            "padding": self.padding,
        }

    @classmethod
    def from_json(cls, obj: InterestRateConfigJSON) -> "InterestRateConfig":
        return cls(
            optimal_utilization_rate=wrapped_i80f48.WrappedI80F48.from_json(
                obj["optimal_utilization_rate"]
            ),
            plateau_interest_rate=wrapped_i80f48.WrappedI80F48.from_json(
                obj["plateau_interest_rate"]
            ),
            max_interest_rate=wrapped_i80f48.WrappedI80F48.from_json(
                obj["max_interest_rate"]
            ),
            insurance_fee_fixed_apr=wrapped_i80f48.WrappedI80F48.from_json(
                obj["insurance_fee_fixed_apr"]
            ),
            insurance_ir_fee=wrapped_i80f48.WrappedI80F48.from_json(
                obj["insurance_ir_fee"]
            ),
            protocol_fixed_fee_apr=wrapped_i80f48.WrappedI80F48.from_json(
                obj["protocol_fixed_fee_apr"]
            ),
            protocol_ir_fee=wrapped_i80f48.WrappedI80F48.from_json(
                obj["protocol_ir_fee"]
            ),
            padding=obj["padding"],
        )
