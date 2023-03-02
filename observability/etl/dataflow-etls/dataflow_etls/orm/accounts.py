import json
import uuid
from dataclasses import dataclass
from typing import Union, Dict, Type, TYPE_CHECKING
from anchorpy.program.common import NamedInstruction as NamedAccountData

from dataflow_etls.utils import pascal_to_snake_case, time_str, wrapped_i80f48_to_float, enum_to_str

if TYPE_CHECKING:
    from dataflow_etls.account_parsing import AccountUpdateRaw

# IDL account names
MARGINFI_GROUP_ACCOUNT_NAME = 'MarginfiGroup'
MARGINFI_ACCOUNT_ACCOUNT_NAME = 'MarginfiAccount'
LENDING_POOL_BANK_ACCOUNT_NAME = 'Bank'


@dataclass
class AccountUpdateRecordBase:
    SCHEMA = ",".join(
        [
            "id:STRING",
            "created_at:TIMESTAMP",
            "idl_version:INTEGER",
            "timestamp:TIMESTAMP",
            "owner_program:STRING",
            "pubkey:STRING",
        ]
    )

    id: str
    created_at: str
    idl_version: int
    timestamp: str
    owner_program: str
    pubkey: str

    def __init__(self, _parsed_data: NamedAccountData, account_update: "AccountUpdateRaw", idl_version: int):
        self.id = str(uuid.uuid4())
        self.created_at = time_str()
        self.timestamp = time_str(account_update['timestamp'])
        self.idl_version = idl_version
        self.owner_program = str(account_update['owner'])
        self.pubkey = str(account_update['pubkey'])

    @classmethod
    def get_tag(cls, snake_case: bool = False) -> str:
        if snake_case:
            return pascal_to_snake_case(cls.__name__)
        else:
            return cls.__name__


# Event headers


@dataclass
class MarginfiGroupUpdateRecord(AccountUpdateRecordBase):
    SCHEMA = AccountUpdateRecordBase.SCHEMA + "," + ",".join(
        [
            "admin:STRING",
        ]
    )

    admin: str

    def __init__(self, parsed_data: NamedAccountData, account_update: "AccountUpdateRaw", idl_version: int):
        super().__init__(parsed_data, account_update, idl_version)

        self.admin = str(parsed_data.data.admin)


@dataclass
class MarginfiAccountUpdateRecord(AccountUpdateRecordBase):
    SCHEMA = AccountUpdateRecordBase.SCHEMA + "," + ",".join(
        [
            "group:STRING",
            "authority:STRING",
            "active_balances:STRING",
        ]
    )

    group: str
    authority: str
    active_balances: str

    def __init__(self, parsed_data: NamedAccountData, account_update: "AccountUpdateRaw", idl_version: int):
        super().__init__(parsed_data, account_update, idl_version)

        self.group = str(parsed_data.data.group)
        self.authority = str(parsed_data.data.authority)
        self.active_balances = json.dumps([{"bank": str(balance.bank_pk),
                                            "asset_shares": wrapped_i80f48_to_float(balance.asset_shares),
                                            "liability_shares": wrapped_i80f48_to_float(balance.liability_shares)
                                            } for balance in parsed_data.data.lending_account.balances if
                                           balance.active])


@dataclass
class LendingPoolBankUpdateRecord(AccountUpdateRecordBase):
    SCHEMA = AccountUpdateRecordBase.SCHEMA + "," + ",".join(
        [
            "mint:STRING",
            "mint_decimals:INTEGER",
            "group:STRING",
            "asset_share_value:BIGNUMERIC",
            "liability_share_value:BIGNUMERIC",
            "liquidity_vault:STRING",
            "liquidity_vault_bump:INTEGER",
            "liquidity_vault_authority_bump:INTEGER",
            "insurance_vault:STRING",
            "insurance_vault_bump:INTEGER",
            "insurance_vault_authority_bump:INTEGER",
            "fee_vault:STRING",
            "fee_vault_bump:INTEGER",
            "fee_vault_authority_bump:INTEGER",
            "collected_insurance_fees_outstanding:BIGNUMERIC",
            "collected_group_fees_outstanding:BIGNUMERIC",
            "total_liability_shares:BIGNUMERIC",
            "total_asset_shares:BIGNUMERIC",
            "last_update:BIGNUMERIC",
            "config_asset_weight_init:BIGNUMERIC",
            "config_asset_weight_maint:BIGNUMERIC",
            "config_liability_weight_init:BIGNUMERIC",
            "config_liability_weight_maint:BIGNUMERIC",
            "config_deposit_limit:BIGNUMERIC",
            "config_borrow_limit:BIGNUMERIC",
            "config_interest_rate_config_optimal_utilization_rate:BIGNUMERIC",
            "config_interest_rate_config_plateau_interest_rate:BIGNUMERIC",
            "config_interest_rate_config_max_interest_rate:BIGNUMERIC",
            "config_interest_rate_config_insurance_fee_fixed_apr:BIGNUMERIC",
            "config_interest_rate_config_insurance_ir_fee:BIGNUMERIC",
            "config_interest_rate_config_protocol_fixed_fee_apr:BIGNUMERIC",
            "config_interest_rate_config_protocol_ir_fee:BIGNUMERIC",
            "config_operational_state:STRING",
            "config_oracle_setup:STRING",
            "config_oracle_keys:STRING",
            "config_risk_tier:STRING",
        ]
    )

    mint: str
    mint_decimals: int
    group: str
    asset_share_value: float
    liability_share_value: float
    liquidity_vault: str
    liquidity_vault_bump: int
    liquidity_vault_authority_bump: int
    insurance_vault: str
    insurance_vault_bump: int
    insurance_vault_authority_bump: int
    fee_vault: str
    fee_vault_bump: int
    fee_vault_authority_bump: int
    collected_insurance_fees_outstanding: float
    collected_group_fees_outstanding: float
    total_liability_shares: float
    total_asset_shares: float
    last_update: int
    config_asset_weight_init: float
    config_asset_weight_maint: float
    config_liability_weight_init: float
    config_liability_weight_maint: float
    config_deposit_limit: int
    config_borrow_limit: int
    config_interest_rate_config_optimal_utilization_rate: float
    config_interest_rate_config_plateau_interest_rate: float
    config_interest_rate_config_max_interest_rate: float
    config_interest_rate_config_insurance_fee_fixed_apr: float
    config_interest_rate_config_insurance_ir_fee: float
    config_interest_rate_config_protocol_fixed_fee_apr: float
    config_interest_rate_config_protocol_ir_fee: float
    config_operational_state: str
    config_oracle_setup: str
    config_oracle_keys: str
    config_risk_tier: str

    def __init__(self, parsed_data: NamedAccountData, account_update: "AccountUpdateRaw", idl_version: int):
        super().__init__(parsed_data, account_update, idl_version)

        self.mint = str(parsed_data.data.mint)
        self.mint_decimals = int(parsed_data.data.mint_decimals)
        self.group = str(parsed_data.data.group)
        self.asset_share_value = wrapped_i80f48_to_float(parsed_data.data.asset_share_value)
        self.liability_share_value = wrapped_i80f48_to_float(parsed_data.data.liability_share_value)
        self.liquidity_vault = str(parsed_data.data.liquidity_vault)
        self.liquidity_vault_bump = int(parsed_data.data.liquidity_vault_bump)
        self.liquidity_vault_authority_bump = int(parsed_data.data.liquidity_vault_authority_bump)
        self.insurance_vault = str(parsed_data.data.insurance_vault)
        self.insurance_vault_bump = int(parsed_data.data.insurance_vault_bump)
        self.insurance_vault_authority_bump = int(parsed_data.data.insurance_vault_authority_bump)
        self.fee_vault = str(parsed_data.data.fee_vault)
        self.fee_vault_bump = int(parsed_data.data.fee_vault_bump)
        self.fee_vault_authority_bump = int(parsed_data.data.fee_vault_authority_bump)
        self.collected_insurance_fees_outstanding = wrapped_i80f48_to_float(
            parsed_data.data.collected_insurance_fees_outstanding)
        self.collected_group_fees_outstanding = wrapped_i80f48_to_float(
            parsed_data.data.collected_group_fees_outstanding)
        self.total_liability_shares = wrapped_i80f48_to_float(parsed_data.data.total_liability_shares)
        self.total_asset_shares = wrapped_i80f48_to_float(parsed_data.data.total_asset_shares)
        self.last_update = int(parsed_data.data.last_update)

        self.config_asset_weight_init = wrapped_i80f48_to_float(parsed_data.data.config.asset_weight_init)
        self.config_asset_weight_maint = wrapped_i80f48_to_float(parsed_data.data.config.asset_weight_maint)
        self.config_liability_weight_init = wrapped_i80f48_to_float(parsed_data.data.config.liability_weight_init)
        self.config_liability_weight_maint = wrapped_i80f48_to_float(parsed_data.data.config.liability_weight_maint)
        self.config_deposit_limit = int(parsed_data.data.config.deposit_limit)
        self.config_borrow_limit = int(parsed_data.data.config.borrow_limit)
        self.config_operational_state = enum_to_str(parsed_data.data.config.operational_state)
        self.config_oracle_setup = enum_to_str(parsed_data.data.config.oracle_setup)
        self.config_oracle_keys = str([str(pk) for pk in parsed_data.data.config.oracle_keys])
        self.config_risk_tier = enum_to_str(parsed_data.data.config.risk_tier)

        self.config_interest_rate_config_optimal_utilization_rate = wrapped_i80f48_to_float(
            parsed_data.data.config.interest_rate_config.optimal_utilization_rate)
        self.config_interest_rate_config_plateau_interest_rate = wrapped_i80f48_to_float(
            parsed_data.data.config.interest_rate_config.plateau_interest_rate)
        self.config_interest_rate_config_max_interest_rate = wrapped_i80f48_to_float(
            parsed_data.data.config.interest_rate_config.max_interest_rate)
        self.config_interest_rate_config_insurance_fee_fixed_apr = wrapped_i80f48_to_float(
            parsed_data.data.config.interest_rate_config.insurance_fee_fixed_apr)
        self.config_interest_rate_config_insurance_ir_fee = wrapped_i80f48_to_float(
            parsed_data.data.config.interest_rate_config.insurance_ir_fee)
        self.config_interest_rate_config_protocol_fixed_fee_apr = wrapped_i80f48_to_float(
            parsed_data.data.config.interest_rate_config.protocol_fixed_fee_apr)
        self.config_interest_rate_config_protocol_ir_fee = wrapped_i80f48_to_float(
            parsed_data.data.config.interest_rate_config.protocol_ir_fee)


AccountUpdateRecordTypes = [MarginfiGroupUpdateRecord,
                            MarginfiAccountUpdateRecord,
                            LendingPoolBankUpdateRecord]

AccountUpdateRecord = Union[
    MarginfiGroupUpdateRecord,
    MarginfiAccountUpdateRecord,
    LendingPoolBankUpdateRecord
]

ACCOUNT_UPDATE_TO_RECORD_TYPE: Dict[str, Type[AccountUpdateRecord]] = {
    f"{MARGINFI_GROUP_ACCOUNT_NAME}": MarginfiGroupUpdateRecord,
    f"{MARGINFI_ACCOUNT_ACCOUNT_NAME}": MarginfiAccountUpdateRecord,
    f"{LENDING_POOL_BANK_ACCOUNT_NAME}": LendingPoolBankUpdateRecord,
}
