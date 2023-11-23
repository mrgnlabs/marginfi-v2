import base64
from dataclasses import asdict
from datetime import datetime
from typing import List, TypedDict, Dict, Any, Tuple, Generator
from decimal import Decimal

from anchorpy_core.idl import Idl
from based58 import based58  # type: ignore
from solders.pubkey import Pubkey
import apache_beam as beam  # type: ignore
from anchorpy.program.common import NamedInstruction as NamedAccountData

from dataflow_etls.idl_versions import VersionedProgram, IdlPool, Cluster
from dataflow_etls.orm.accounts import ACCOUNT_UPDATE_TO_RECORD_TYPE, AccountUpdateRecord

AccountUpdateRaw = TypedDict('AccountUpdateRaw', {
    'id': str,
    'created_at': datetime,
    'timestamp': datetime,
    'owner': str,
    'slot': Decimal,
    'pubkey': str,
    'txn_signature': str,
    'lamports': Decimal,
    'executable': bool,
    'rent_epoch': Decimal,
    'data': str,
})


class OwnerProgramNotSupported(Exception):
    pass


def parse_account(account_update: AccountUpdateRaw, min_idl_version: int, cluster: Cluster,
                  idl_pool: IdlPool) -> List[AccountUpdateRecord]:
    owner_program_id_str = account_update["owner"]
    owner_program_id = Pubkey.from_string(owner_program_id_str)
    account_update_slot = int(account_update["slot"])

    try:
        idl_raw, idl_version = idl_pool.get_idl_for_slot(owner_program_id_str, account_update_slot)
    except KeyError:
        raise OwnerProgramNotSupported(f"Unsupported program {owner_program_id_str}")

    idl = Idl.from_json(idl_raw)
    program = VersionedProgram(cluster, idl_version, idl, owner_program_id)

    if idl_version < min_idl_version:
        return []

    account_data_bytes = base64.b64decode(account_update["data"])

    try:
        parsed_account_data: NamedAccountData = program.coder.accounts.parse(account_data_bytes)
    except Exception as e:
        print(f"failed to parse account data in update {account_update['id']}", e)
        return []

    if parsed_account_data.name not in ACCOUNT_UPDATE_TO_RECORD_TYPE:
        print(f"discarding unsupported account type {parsed_account_data.name} in update {account_update['id']}")
        return []
    else:
        # noinspection PyPep8Naming
        AccountUpdateRecordType = ACCOUNT_UPDATE_TO_RECORD_TYPE[parsed_account_data.name]
        return [AccountUpdateRecordType(parsed_account_data, account_update, idl_version)]


class DispatchEventsDoFn(beam.DoFn):  # type: ignore
    def process(self, record: AccountUpdateRecord, *args: Tuple[Any], **kwargs: Dict[str, Tuple[Any]]) -> Generator[
        str, None, None]:
        yield beam.pvalue.TaggedOutput(record.get_tag(), record)


def dictionify_record(record: AccountUpdateRecord) -> Dict[str, Any]:
    return asdict(record)
