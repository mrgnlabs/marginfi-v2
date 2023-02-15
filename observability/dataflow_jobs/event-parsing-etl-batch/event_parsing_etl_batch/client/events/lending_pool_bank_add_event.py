import typing
from dataclasses import dataclass
from solders.pubkey import Pubkey
import borsh_construct as borsh
from anchorpy.coder.event import EVENT_DISCRIMINATOR_SIZE
from anchorpy.error import EventInvalidDiscriminator
from anchorpy.borsh_extension import BorshPubkey
from .. import types


class LendingPoolBankAddEventJSON(typing.TypedDict):
    header: types.group_event_header.GroupEventHeaderJSON
    bank: str
    mint: str


@dataclass
class LendingPoolBankAddEvent:
    discriminator: typing.ClassVar = b">\xaf?\xdc\xb9Th-"
    layout: typing.ClassVar = borsh.CStruct(
        "header" / types.group_event_header.GroupEventHeader.layout,
        "bank" / BorshPubkey,
        "mint" / BorshPubkey,
    )
    header: types.group_event_header.GroupEventHeader
    bank: Pubkey
    mint: Pubkey

    @classmethod
    def decode(cls, data: bytes) -> "LendingPoolBankAddEvent":
        if data[:EVENT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise EventInvalidDiscriminator(
                "The discriminator for this event is invalid"
            )
        dec = LendingPoolBankAddEvent.layout.parse(data[EVENT_DISCRIMINATOR_SIZE:])
        return cls(
            header=types.group_event_header.GroupEventHeader.from_decoded(dec.header),
            bank=dec.bank,
            mint=dec.mint,
        )

    def to_json(self) -> LendingPoolBankAddEventJSON:
        return {
            "header": self.header.to_json(),
            "bank": str(self.bank),
            "mint": str(self.mint),
        }

    @classmethod
    def from_json(cls, obj: LendingPoolBankAddEventJSON) -> "LendingPoolBankAddEvent":
        return cls(
            header=types.group_event_header.GroupEventHeader.from_json(obj["header"]),
            bank=Pubkey.from_string(obj["bank"]),
            mint=Pubkey.from_string(obj["mint"]),
        )
