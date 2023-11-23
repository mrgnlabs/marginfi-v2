import base64
import json
import re
from dataclasses import dataclass, asdict
from datetime import datetime
from typing import List, Any, Callable, Optional, Dict, Sequence, Tuple, Generator, Union, TypedDict
from decimal import Decimal

from anchorpy_core.idl import Idl
from solders.message import Message, MessageV0
from based58 import based58  # type: ignore
from anchorpy import NamedInstruction
from solders.instruction import CompiledInstruction
from solders.pubkey import Pubkey
import apache_beam as beam  # type: ignore

from dataflow_etls.idl_versions import VersionedProgram, IdlPool, Cluster
from dataflow_etls.orm.events import EVENT_TO_RECORD_TYPE, EventRecord


@dataclass
class Instruction:
    program_id: Pubkey
    accounts: List[Pubkey]
    data: bytes


@dataclass
class InstructionWithLogs:
    timestamp: datetime
    idl_version: int
    signature: str
    message: Instruction
    logs: List[str]
    inner_instructions: List["InstructionWithLogs"]
    logs_truncated: bool
    is_cpi: bool
    # call_stack: List[Pubkey]


INVOKE_MESSAGE = "Program log: "
PROGRAM_LOG = "Program log: "
PROGRAM_DATA = "Program data: "
LOG_TRUNCATED = "Log truncated"

TransactionRaw = TypedDict('TransactionRaw', {
    'id': str,
    'created_at': datetime,
    'timestamp': datetime,
    'signature': str,
    'indexing_address': str,
    'slot': Decimal,
    'signer': str,
    'success': bool,
    'version': str,
    'fee': Decimal,
    'meta': str,
    'message': str,
})


class IndexedProgramNotSupported(Exception):
    pass


def extract_events_from_tx(tx: TransactionRaw, min_idl_version: int, cluster: Cluster, idl_pool: IdlPool) -> List[
    EventRecord]:
    indexed_program_id_str = tx["indexing_address"]
    indexed_program_id = Pubkey.from_string(indexed_program_id_str)
    tx_slot = int(tx["slot"])

    try:
        idl_raw, idl_version = idl_pool.get_idl_for_slot(indexed_program_id_str, tx_slot)
    except KeyError:
        raise IndexedProgramNotSupported(f"Unsupported indexed program {indexed_program_id_str}")

    idl = Idl.from_json(idl_raw)
    program = VersionedProgram(cluster, idl_version, idl, indexed_program_id)

    if min_idl_version is not None and idl_version < min_idl_version:
        return []

    meta = json.loads(tx["meta"])
    message_bytes = base64.b64decode(tx["message"])

    tx_version = tx["version"]
    message_decoded: Union[Message, MessageV0]
    if tx_version == "legacy":
        message_decoded = Message.from_bytes(message_bytes)
    elif tx_version == "0":
        message_decoded = MessageV0.from_bytes(message_bytes[1:])
    else:
        return []

    merged_instructions = merge_instructions_and_cpis(message_decoded.instructions, meta["innerInstructions"])
    expanded_instructions = expand_instructions(message_decoded.account_keys, merged_instructions)
    ixs_with_logs = reconcile_instruction_logs(tx["timestamp"], tx["signature"], expanded_instructions,
                                               meta["logMessages"], idl_version)

    records_list = []
    for ix_with_logs in ixs_with_logs:
        records_list.extend(extract_events_from_ix(ix_with_logs, program))

    return records_list


def merge_instructions_and_cpis(message_instructions: List[CompiledInstruction], inner_instructions: List[Any]) -> List[
    CompiledInstruction]:
    def search(array: List[Any], callback: Callable[[Any], bool]) -> Optional[int]:
        for i, elem in enumerate(array):
            if callback(elem):
                return i
        return None

    compiled_instructions: List[CompiledInstruction] = []
    for ix_index, instruction in enumerate(message_instructions):
        compiled_instructions.append(instruction)
        inner_ixs_index = search(inner_instructions, lambda inner_ixs: bool(inner_ixs["index"] == ix_index))
        if inner_ixs_index is not None:
            for ix_raw in inner_instructions[inner_ixs_index]["instructions"]:
                compiled_instructions.append(
                    CompiledInstruction(program_id_index=ix_raw["programIdIndex"], accounts=bytes(ix_raw["accounts"]),
                                        data=based58.b58decode(str.encode(ix_raw["data"]))))

    return compiled_instructions


def expand_instructions(account_keys: List[Pubkey], compiled_instructions: List[CompiledInstruction]) -> List[
    Instruction]:
    expanded_instructions = []
    for ix in compiled_instructions:
        expanded_instruction = Instruction(data=ix.data,
                                           accounts=[account_keys[account_index] for account_index in ix.accounts],
                                           program_id=account_keys[ix.program_id_index])
        expanded_instructions.append(expanded_instruction)
    return expanded_instructions


def reconcile_instruction_logs(timestamp: datetime, signature: str, instructions: List[Instruction], logs: List[str],
                               idl_version: int) -> \
        List[InstructionWithLogs]:
    depth = 0
    instructions_consumed = 0
    instructions_with_logs: List[InstructionWithLogs] = []

    for log in logs:
        if log.startswith(LOG_TRUNCATED):
            ix = get_latest_ix_ref(instructions_with_logs, depth)
            ix.logs_truncated = True
        else:
            invoke_regex = r"Program (?P<pid>\w+) invoke"
            matches = re.search(invoke_regex, log)
            if matches is not None:
                target_instruction_list = instructions_with_logs
                for i in range(depth):
                    target_instruction_list = target_instruction_list[-1].inner_instructions

                message = instructions[instructions_consumed]
                target_instruction_list.append(
                    InstructionWithLogs(timestamp=timestamp, idl_version=idl_version, signature=signature, logs=[log],
                                        message=message,
                                        inner_instructions=[], logs_truncated=False, is_cpi=(depth > 0)))
                depth += 1
                instructions_consumed += 1
            else:
                if "success" in log or "failed" in log:
                    ix = get_latest_ix_ref(instructions_with_logs, depth)
                    ix.logs.append(log)
                    depth -= 1
                else:
                    ix = get_latest_ix_ref(instructions_with_logs, depth)
                    ix.logs.append(log)

    return instructions_with_logs


def get_latest_ix_ref(instructions: List[InstructionWithLogs], stack_depth: int) -> "InstructionWithLogs":
    target_instruction_list = instructions
    for i in range(stack_depth - 1):
        target_instruction_list = target_instruction_list[-1].inner_instructions
    return target_instruction_list[-1]


def extract_events_from_ix(ix: InstructionWithLogs, program: VersionedProgram) -> List[EventRecord]:
    ix_events: List[EventRecord] = []

    if ix.message.program_id == program.program_id:
        ix_events.extend(create_records_from_ix(ix, program))

    for inner_ix in ix.inner_instructions:
        ix_events.extend(extract_events_from_ix(inner_ix, program))

    return ix_events


def create_records_from_ix(ix: InstructionWithLogs, program: VersionedProgram) -> Sequence[EventRecord]:
    records: List[EventRecord] = []

    try:
        parsed_ix: NamedInstruction = program.coder.instruction.parse(ix.message.data)
    except Exception as e:
        print(f"failed to parse instruction data in tx {ix.signature} ({ix.timestamp})", e)
        return records

    for log in ix.logs:
        if not log.startswith(PROGRAM_DATA):
            continue

        event_encoded = log[len(PROGRAM_DATA):]
        try:
            event_bytes = base64.b64decode(event_encoded)
        except Exception as e:
            print(f"error: failed to decode base64 event string in tx {ix.signature}", e)
            continue

        try:
            event = program.coder.events.parse(event_bytes)
        except Exception as e:
            print(f"failed to parse event in tx {ix.signature}", e)
            continue

        if event is None or event.name not in EVENT_TO_RECORD_TYPE:
            print(f"discarding unsupported event in tx {ix.signature}")
            print(event)
        else:
            # noinspection PyPep8Naming
            RecordType = EVENT_TO_RECORD_TYPE[event.name]
            records.append(RecordType(event, ix, parsed_ix))

    return records


class DispatchEventsDoFn(beam.DoFn):  # type: ignore
    def process(self, record: EventRecord, *args: Tuple[Any], **kwargs: Dict[str, Tuple[Any]]) -> Generator[
        str, None, None]:
        yield beam.pvalue.TaggedOutput(record.get_tag(), record)


def dictionify_record(record: EventRecord) -> Dict[str, Any]:
    return asdict(record)
