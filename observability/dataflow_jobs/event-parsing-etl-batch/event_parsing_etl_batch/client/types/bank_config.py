from __future__ import annotations
from . import (
    interest_rate_config,
    wrapped_i80f48,
    bank_operational_state,
    oracle_setup,
)
import typing
from dataclasses import dataclass
from construct import Container
from solders.pubkey import Pubkey
from anchorpy.borsh_extension import BorshPubkey
import borsh_construct as borsh


class BankConfigJSON(typing.TypedDict):
    asset_weight_init: wrapped_i80f48.WrappedI80F48JSON
    asset_weight_maint: wrapped_i80f48.WrappedI80F48JSON
    liability_weight_init: wrapped_i80f48.WrappedI80F48JSON
    liability_weight_maint: wrapped_i80f48.WrappedI80F48JSON
    deposit_limit: int
    interest_rate_config: interest_rate_config.InterestRateConfigJSON
    operational_state: bank_operational_state.BankOperationalStateJSON
    oracle_setup: oracle_setup.OracleSetupJSON
    oracle_keys: list[str]
    borrow_limit: int
    padding: list[int]


@dataclass
class BankConfig:
    layout: typing.ClassVar = borsh.CStruct(
        "asset_weight_init" / wrapped_i80f48.WrappedI80F48.layout,
        "asset_weight_maint" / wrapped_i80f48.WrappedI80F48.layout,
        "liability_weight_init" / wrapped_i80f48.WrappedI80F48.layout,
        "liability_weight_maint" / wrapped_i80f48.WrappedI80F48.layout,
        "deposit_limit" / borsh.U64,
        "interest_rate_config" / interest_rate_config.InterestRateConfig.layout,
        "operational_state" / bank_operational_state.layout,
        "oracle_setup" / oracle_setup.layout,
        "oracle_keys" / BorshPubkey[5],
        "borrow_limit" / borsh.U64,
        "padding" / borsh.U64[7],
    )
    asset_weight_init: wrapped_i80f48.WrappedI80F48
    asset_weight_maint: wrapped_i80f48.WrappedI80F48
    liability_weight_init: wrapped_i80f48.WrappedI80F48
    liability_weight_maint: wrapped_i80f48.WrappedI80F48
    deposit_limit: int
    interest_rate_config: interest_rate_config.InterestRateConfig
    operational_state: bank_operational_state.BankOperationalStateKind
    oracle_setup: oracle_setup.OracleSetupKind
    oracle_keys: list[Pubkey]
    borrow_limit: int
    padding: list[int]

    @classmethod
    def from_decoded(cls, obj: Container) -> "BankConfig":
        return cls(
            asset_weight_init=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.asset_weight_init
            ),
            asset_weight_maint=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.asset_weight_maint
            ),
            liability_weight_init=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.liability_weight_init
            ),
            liability_weight_maint=wrapped_i80f48.WrappedI80F48.from_decoded(
                obj.liability_weight_maint
            ),
            deposit_limit=obj.deposit_limit,
            interest_rate_config=interest_rate_config.InterestRateConfig.from_decoded(
                obj.interest_rate_config
            ),
            operational_state=bank_operational_state.from_decoded(
                obj.operational_state
            ),
            oracle_setup=oracle_setup.from_decoded(obj.oracle_setup),
            oracle_keys=obj.oracle_keys,
            borrow_limit=obj.borrow_limit,
            padding=obj.padding,
        )

    def to_encodable(self) -> dict[str, typing.Any]:
        return {
            "asset_weight_init": self.asset_weight_init.to_encodable(),
            "asset_weight_maint": self.asset_weight_maint.to_encodable(),
            "liability_weight_init": self.liability_weight_init.to_encodable(),
            "liability_weight_maint": self.liability_weight_maint.to_encodable(),
            "deposit_limit": self.deposit_limit,
            "interest_rate_config": self.interest_rate_config.to_encodable(),
            "operational_state": self.operational_state.to_encodable(),
            "oracle_setup": self.oracle_setup.to_encodable(),
            "oracle_keys": self.oracle_keys,
            "borrow_limit": self.borrow_limit,
            "padding": self.padding,
        }

    def to_json(self) -> BankConfigJSON:
        return {
            "asset_weight_init": self.asset_weight_init.to_json(),
            "asset_weight_maint": self.asset_weight_maint.to_json(),
            "liability_weight_init": self.liability_weight_init.to_json(),
            "liability_weight_maint": self.liability_weight_maint.to_json(),
            "deposit_limit": self.deposit_limit,
            "interest_rate_config": self.interest_rate_config.to_json(),
            "operational_state": self.operational_state.to_json(),
            "oracle_setup": self.oracle_setup.to_json(),
            "oracle_keys": list(map(lambda item: str(item), self.oracle_keys)),
            "borrow_limit": self.borrow_limit,
            "padding": self.padding,
        }

    @classmethod
    def from_json(cls, obj: BankConfigJSON) -> "BankConfig":
        return cls(
            asset_weight_init=wrapped_i80f48.WrappedI80F48.from_json(
                obj["asset_weight_init"]
            ),
            asset_weight_maint=wrapped_i80f48.WrappedI80F48.from_json(
                obj["asset_weight_maint"]
            ),
            liability_weight_init=wrapped_i80f48.WrappedI80F48.from_json(
                obj["liability_weight_init"]
            ),
            liability_weight_maint=wrapped_i80f48.WrappedI80F48.from_json(
                obj["liability_weight_maint"]
            ),
            deposit_limit=obj["deposit_limit"],
            interest_rate_config=interest_rate_config.InterestRateConfig.from_json(
                obj["interest_rate_config"]
            ),
            operational_state=bank_operational_state.from_json(
                obj["operational_state"]
            ),
            oracle_setup=oracle_setup.from_json(obj["oracle_setup"]),
            oracle_keys=list(
                map(lambda item: Pubkey.from_string(item), obj["oracle_keys"])
            ),
            borrow_limit=obj["borrow_limit"],
            padding=obj["padding"],
        )
