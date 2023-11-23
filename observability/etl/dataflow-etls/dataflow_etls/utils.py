import re
from datetime import datetime, timezone
from typing import NamedTuple, TypeVar, Optional, Callable, Any

from decimal import Decimal


def pascal_to_snake_case(string: str) -> str:
    return re.sub('(?!^)([A-Z]+)', r'_\1', string).lower()


WrappedI80F48 = NamedTuple('WrappedI80F48', [('value', int)])


def wrapped_i80f48_to_float(wrapped_i80f48: WrappedI80F48) -> float:
    nb_of_fractional_bits = 48
    value = Decimal(wrapped_i80f48.value)
    value = value / 2 ** nb_of_fractional_bits
    return float(value)


def enum_to_str(enum: Any) -> str:
    return enum.__class__.__name__.lower()


InputType = TypeVar('InputType')
OutputType = TypeVar('OutputType')


def map_optional(element: Optional[InputType], fn: Callable[[InputType], OutputType]) -> Optional[OutputType]:
    if element is not None:
        return fn(element)
    else:
        return None


def time_str(dt: Optional[datetime] = None) -> str:
    if dt is None:
        dt = datetime.now(timezone.utc)
    return dt.strftime("%Y-%m-%d %H:%M:%S %Z")
