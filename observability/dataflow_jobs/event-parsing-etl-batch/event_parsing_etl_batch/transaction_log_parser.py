import re
from dataclasses import dataclass
from typing import List
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
    cpis: List["InstructionWithLogs"]
    logs_truncated: bool


INVOKE_MESSAGE = "Program log: "
PROGRAM_LOG = "Program log: "
PROGRAM_DATA = "Program data: "
LOG_TRUNCATED = "Log truncated"


def get_latest_ix_ref(instructions: List[InstructionWithLogs], stack_depth: int) -> "InstructionWithLogs":
    target_instruction_list = instructions
    for i in range(stack_depth - 1):
        target_instruction_list = target_instruction_list[-1].cpis
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
                    target_instruction_list = target_instruction_list[-1].cpis
                target_instruction_list.append(
                    InstructionWithLogs(logs=[log], message=instructions[instructions_consumed],
                                        cpis=[], logs_truncated=False))
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
