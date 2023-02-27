import uuid
from dataclasses import dataclass
from typing import Union, Dict, Type, TYPE_CHECKING
from anchorpy.program.common import NamedInstruction as NamedAccountData

from dataflow_etls.utils import pascal_to_snake_case, time_str

if TYPE_CHECKING:
    from dataflow_etls.account_parsing import AccountUpdateRaw

# IDL account names
MARGINFI_GROUP_ACCOUNT_NAME = 'MarginfiGroup'
MARGINFI_ACCOUNT_ACCOUNT_NAME = 'MarginfiGroup'
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
        ]
    )

    id: str
    created_at: str
    idl_version: int
    timestamp: str
    owner_program: str

    def __init__(self, _parsed_data: NamedAccountData, account_update: "AccountUpdateRaw", idl_version: int):
        self.id = str(uuid.uuid4())
        self.created_at = time_str()
        self.timestamp = time_str(account_update['timestamp'])
        self.idl_version = idl_version
        self.owner_program = str(account_update['owner'])

    @classmethod
    def get_tag(cls, snake_case: bool = False) -> str:
        if snake_case:
            return pascal_to_snake_case(cls.__name__)
        else:
            return cls.__name__


# Event headers


@dataclass
class MarginfiGroupUpdateRecord(AccountUpdateRecordBase):
    SCHEMA = AccountUpdateRecordBase.SCHEMA

    def __init__(self, parsed_data: NamedAccountData, account_update: "AccountUpdateRaw", idl_version: int):
        super().__init__(parsed_data, account_update, idl_version)


@dataclass
class MarginfiAccountUpdateRecord(AccountUpdateRecordBase):
    SCHEMA = AccountUpdateRecordBase.SCHEMA + "," + ",".join(
        [
        ]
    )

    def __init__(self, parsed_data: NamedAccountData, account_update: "AccountUpdateRaw", idl_version: int):
        super().__init__(parsed_data, account_update, idl_version)


@dataclass
class LendingPoolBankUpdateRecord(AccountUpdateRecordBase):
    SCHEMA = AccountUpdateRecordBase.SCHEMA + "," + ",".join(
        [
        ]
    )

    def __init__(self, parsed_data: NamedAccountData, account_update: "AccountUpdateRaw", idl_version: int):
        super().__init__(parsed_data, account_update, idl_version)


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
