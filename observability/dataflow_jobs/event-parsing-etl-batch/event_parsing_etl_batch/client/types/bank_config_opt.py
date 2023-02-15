from __future__ import annotations
from . import (
    wrapped_i80f48,
    oracle_config,
    bank_operational_state,
)
import typing
from dataclasses import dataclass
from construct import Container
import borsh_construct as borsh


class BankConfigOptJSON(typing.TypedDict):
    asset_weight_init: typing.Optional[wrapped_i80f48.WrappedI80F48JSON]
    asset_weight_maint: typing.Optional[wrapped_i80f48.WrappedI80F48JSON]
    liability_weight_init: typing.Optional[wrapped_i80f48.WrappedI80F48JSON]
    liability_weight_maint: typing.Optional[wrapped_i80f48.WrappedI80F48JSON]
    deposit_limit: typing.Optional[int]
    borrow_limit: typing.Optional[int]
    operational_state: typing.Optional[bank_operational_state.BankOperationalStateJSON]
    oracle: typing.Optional[oracle_config.OracleConfigJSON]


@dataclass
class BankConfigOpt:
    layout: typing.ClassVar = borsh.CStruct(
        "asset_weight_init" / borsh.Option(wrapped_i80f48.WrappedI80F48.layout),
        "asset_weight_maint" / borsh.Option(wrapped_i80f48.WrappedI80F48.layout),
        "liability_weight_init" / borsh.Option(wrapped_i80f48.WrappedI80F48.layout),
        "liability_weight_maint" / borsh.Option(wrapped_i80f48.WrappedI80F48.layout),
        "deposit_limit" / borsh.Option(borsh.U64),
        "borrow_limit" / borsh.Option(borsh.U64),
        "operational_state" / borsh.Option(bank_operational_state.layout),
        "oracle" / borsh.Option(oracle_config.OracleConfig.layout),
    )
    asset_weight_init: typing.Optional[wrapped_i80f48.WrappedI80F48]
    asset_weight_maint: typing.Optional[wrapped_i80f48.WrappedI80F48]
    liability_weight_init: typing.Optional[wrapped_i80f48.WrappedI80F48]
    liability_weight_maint: typing.Optional[wrapped_i80f48.WrappedI80F48]
    deposit_limit: typing.Optional[int]
    borrow_limit: typing.Optional[int]
    operational_state: typing.Optional[bank_operational_state.BankOperationalStateKind]
    oracle: typing.Optional[oracle_config.OracleConfig]

    @classmethod
    def from_decoded(cls, obj: Container) -> "BankConfigOpt":
        return cls(
            asset_weight_init=(
                None
                if obj.asset_weight_init is None
                else wrapped_i80f48.WrappedI80F48.from_decoded(obj.asset_weight_init)
            ),
            asset_weight_maint=(
                None
                if obj.asset_weight_maint is None
                else wrapped_i80f48.WrappedI80F48.from_decoded(obj.asset_weight_maint)
            ),
            liability_weight_init=(
                None
                if obj.liability_weight_init is None
                else wrapped_i80f48.WrappedI80F48.from_decoded(
                    obj.liability_weight_init
                )
            ),
            liability_weight_maint=(
                None
                if obj.liability_weight_maint is None
                else wrapped_i80f48.WrappedI80F48.from_decoded(
                    obj.liability_weight_maint
                )
            ),
            deposit_limit=obj.deposit_limit,
            borrow_limit=obj.borrow_limit,
            operational_state=(
                None
                if obj.operational_state is None
                else bank_operational_state.from_decoded(obj.operational_state)
            ),
            oracle=(
                None
                if obj.oracle is None
                else oracle_config.OracleConfig.from_decoded(obj.oracle)
            ),
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "asset_weight_init": (
                None
                if self.asset_weight_init is None
                else self.asset_weight_init.to_encodable()
            ),
            "asset_weight_maint": (
                None
                if self.asset_weight_maint is None
                else self.asset_weight_maint.to_encodable()
            ),
            "liability_weight_init": (
                None
                if self.liability_weight_init is None
                else self.liability_weight_init.to_encodable()
            ),
            "liability_weight_maint": (
                None
                if self.liability_weight_maint is None
                else self.liability_weight_maint.to_encodable()
            ),
            "deposit_limit": self.deposit_limit,
            "borrow_limit": self.borrow_limit,
            "operational_state": (
                None
                if self.operational_state is None
                else self.operational_state.to_encodable()
            ),
            "oracle": (None if self.oracle is None else self.oracle.to_encodable()),
        }

    def to_json(self) -> BankConfigOptJSON:
        return {
            "asset_weight_init": (
                None
                if self.asset_weight_init is None
                else self.asset_weight_init.to_json()
            ),
            "asset_weight_maint": (
                None
                if self.asset_weight_maint is None
                else self.asset_weight_maint.to_json()
            ),
            "liability_weight_init": (
                None
                if self.liability_weight_init is None
                else self.liability_weight_init.to_json()
            ),
            "liability_weight_maint": (
                None
                if self.liability_weight_maint is None
                else self.liability_weight_maint.to_json()
            ),
            "deposit_limit": self.deposit_limit,
            "borrow_limit": self.borrow_limit,
            "operational_state": (
                None
                if self.operational_state is None
                else self.operational_state.to_json()
            ),
            "oracle": (None if self.oracle is None else self.oracle.to_json()),
        }

    @classmethod
    def from_json(cls, obj: BankConfigOptJSON) -> "BankConfigOpt":
        return cls(
            asset_weight_init=(
                None
                if obj["asset_weight_init"] is None
                else wrapped_i80f48.WrappedI80F48.from_json(obj["asset_weight_init"])
            ),
            asset_weight_maint=(
                None
                if obj["asset_weight_maint"] is None
                else wrapped_i80f48.WrappedI80F48.from_json(obj["asset_weight_maint"])
            ),
            liability_weight_init=(
                None
                if obj["liability_weight_init"] is None
                else wrapped_i80f48.WrappedI80F48.from_json(
                    obj["liability_weight_init"]
                )
            ),
            liability_weight_maint=(
                None
                if obj["liability_weight_maint"] is None
                else wrapped_i80f48.WrappedI80F48.from_json(
                    obj["liability_weight_maint"]
                )
            ),
            deposit_limit=obj["deposit_limit"],
            borrow_limit=obj["borrow_limit"],
            operational_state=(
                None
                if obj["operational_state"] is None
                else bank_operational_state.from_json(obj["operational_state"])
            ),
            oracle=(
                None
                if obj["oracle"] is None
                else oracle_config.OracleConfig.from_json(obj["oracle"])
            ),
        )
