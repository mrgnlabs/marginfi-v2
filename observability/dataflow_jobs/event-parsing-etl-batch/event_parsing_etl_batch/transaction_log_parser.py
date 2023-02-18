import base64
import re
from dataclasses import dataclass
from typing import List, Any, Callable, Optional

from solders.instruction import CompiledInstruction
from solders.pubkey import Pubkey


@dataclass
class Instruction:
    program_id: Pubkey
    accounts: List[Pubkey]
    data: bytes


@dataclass
class InstructionWithLogs:
    message: Instruction
    logs: List[str]
    inner_instructions: List["InstructionWithLogs"]
    logs_truncated: bool


INVOKE_MESSAGE = "Program log: "
PROGRAM_LOG = "Program log: "
PROGRAM_DATA = "Program data: "
LOG_TRUNCATED = "Log truncated"


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
        inner_ixs_index = search(inner_instructions, lambda inner_ixs: inner_ixs["index"] == ix_index)
        if inner_ixs_index is not None:
            for ix_raw in inner_instructions[inner_ixs_index]["instructions"]:
                compiled_instructions.append(
                    CompiledInstruction(program_id_index=ix_raw["programIdIndex"], accounts=bytes(ix_raw["accounts"]),
                                        data=base64.b85decode(ix_raw["data"])))

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


def get_latest_ix_ref(instructions: List[InstructionWithLogs], stack_depth: int) -> "InstructionWithLogs":
    target_instruction_list = instructions
    for i in range(stack_depth - 1):
        target_instruction_list = target_instruction_list[-1].inner_instructions
    return target_instruction_list[-1]


def reconcile_instruction_logs(instructions: List[Instruction], logs: List[str]) -> List[InstructionWithLogs]:
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
                target_instruction_list.append(
                    InstructionWithLogs(logs=[log], message=instructions[instructions_consumed],
                                        inner_instructions=[], logs_truncated=False))
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