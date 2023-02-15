"""This module contains code for handling Anchor events.

Straight copy from anchorpy, modified to parse events into statically generated types
"""
import binascii
from base64 import b64decode
from dataclasses import dataclass
from typing import Callable, List, Optional, cast, Union
from solders.pubkey import Pubkey
from anchorpy.coder.coder import Coder
from anchorpy.coder.event import EVENT_DISCRIMINATOR_SIZE

from event_parsing_etl_batch.client.events import *

PROGRAM_LOG = "Program log: "
PROGRAM_DATA = "Program data: "
PROGRAM_LOG_START_INDEX = len(PROGRAM_LOG)
PROGRAM_DATA_START_INDEX = len(PROGRAM_DATA)

MarginfiEvent = Union[
    MarginfiAccountCreateEvent, LendingAccountDepositEvent, LendingAccountWithdrawEvent, LendingAccountRepayEvent, LendingAccountBorrowEvent]


@dataclass
class Event:
    name: str
    data: MarginfiEvent


class _ExecutionContext:
    """Stack frame execution context, allowing one to track what program is executing for a given log."""  # noqa: E501

    def __init__(self, log: str) -> None:
        """Init.

        Args:
            log: The log to process.

        Raises:
            ValueError: If the log line is malformed.
        """
        try:
            program = log.split("Program ")[1].split(" invoke [")[0]
        except IndexError as e:
            raise ValueError("Could not find program invocation log line") from e
        self.stack = [program]

    def program(self) -> str:
        """Return the currently executing program.

        Returns:
            The name of the program.
        """
        return self.stack[-1]

    def push(self, new_program: str) -> None:
        """Add to the stack.

        Args:
            new_program: The program to add.
        """
        self.stack.append(new_program)

    def pop(self) -> None:
        """Pop from the stack."""
        self.stack.pop()


@dataclass
class EventParser:
    """Parser to handle on_logs callbacks."""

    program_id: Pubkey
    coder: Coder

    def parse_logs(self, logs: List[str], callback: Callable[[Event], None]) -> None:
        """Parse a list of logs using a provided callback.

        Args:
            logs: The logs to parse.
            callback: The function to handle the parsed log.
        """
        log_scanner = _LogScanner(logs)
        execution = _ExecutionContext(cast(str, log_scanner.to_next()))
        log = log_scanner.to_next()
        while log is not None:
            event, new_program, did_pop = self.handle_log(execution, log)
            if event is not None:
                callback(event)
            if new_program is not None:
                execution.push(new_program)
            if did_pop:
                execution.pop()
            log = log_scanner.to_next()

    def handle_log(
            self,
            execution: _ExecutionContext,
            log: str,
    ) -> tuple[Optional[Event], Optional[str], bool]:
        """Main log handler.

        Args:
            execution: The execution stack.
            log: log string from the RPC node.

        Returns:
            A three element array of the event, the next program
            that was invoked for CPI, and a boolean indicating if
            a program has completed execution (and thus should be popped off the
            execution stack).
        """
        # Executing program is this program.
        if execution.stack and execution.program() == str(self.program_id):
            return self.handle_program_log(log)
        # Executing program is not this program.
        return (None, *self.handle_system_log(log))

    def handle_program_log(
            self, log: str
    ) -> tuple[Optional[Event], Optional[str], bool]:
        """Handle logs from *this* program.

        Args:
            log: log string from the RPC node.

        """
        # This is a `msg!` log or a `sol_log_data!` log.
        # `log.startswith(PROGRAM_LOG)` removed under the assumption that we log through `emit!` / `sol_log_data!`
        if log.startswith(PROGRAM_DATA):
            log_str = log[PROGRAM_DATA_START_INDEX:]
            try:
                decoded = b64decode(log_str)
            except binascii.Error:
                return None, None, False

            disc = decoded[:EVENT_DISCRIMINATOR_SIZE]
            if disc == MarginfiAccountCreateEvent.discriminator:
                event = Event(name="MarginfiAccountCreateEvent", data=MarginfiAccountCreateEvent.decode(decoded))
            elif disc == LendingAccountDepositEvent.discriminator:
                event = Event(name="LendingAccountDepositEvent", data=LendingAccountDepositEvent.decode(decoded))
            elif disc == LendingAccountWithdrawEvent.discriminator:
                event = Event(name="LendingAccountWithdrawEvent", data=LendingAccountWithdrawEvent.decode(decoded))
            elif disc == LendingAccountBorrowEvent.discriminator:
                event = Event(name="LendingAccountBorrowEvent", data=LendingAccountBorrowEvent.decode(decoded))
            elif disc == LendingAccountRepayEvent.discriminator:
                event = Event(name="LendingAccountRepayEvent", data=LendingAccountRepayEvent.decode(decoded))
            elif disc == LendingAccountWithdrawEvent.discriminator:
                event = Event(name="LendingAccountWithdrawEvent", data=LendingAccountWithdrawEvent.decode(decoded))
            elif disc == LendingPoolBankAddEvent.discriminator:
                event = Event(name="LendingPoolBankAddEvent", data=LendingPoolBankAddEvent.decode(decoded))
            elif disc == LendingPoolBankAccrueInterestEvent.discriminator:
                event = Event(name="LendingPoolBankAccrueInterestEvent",
                              data=LendingPoolBankAccrueInterestEvent.decode(decoded))
            else:
                print("Program event not supported", decoded, log_str)
                event = None
            return event, None, False
        return (None, *self.handle_system_log(log))

    def handle_system_log(self, log: str) -> tuple[Optional[str], bool]:
        """Handle logs when the current program being executing is *not* this.

        Args:
            log: log string from the RPC node.

        """
        log_start = log.split(":")[0]
        splitted = log_start.split(" ")
        invoke_msg = f"Program {str(self.program_id)} invoke"
        if len(splitted) == 3 and splitted[0] == "Program" and splitted[2] == "success":
            return None, True
        if log_start.startswith(invoke_msg):
            return str(self.program_id), False
        if "invoke" in log_start:
            return "cpi", False
        return None, False


@dataclass
class _LogScanner:
    """Object that iterates over logs."""

    logs: list[str]

    def to_next(self) -> Optional[str]:
        """Move to the next log item.

        Returns:
            The next log line, or None if there's nothing to return.
        """
        if self.logs:
            log = self.logs[0]
            self.logs = self.logs[1:]
            return log
        return None
