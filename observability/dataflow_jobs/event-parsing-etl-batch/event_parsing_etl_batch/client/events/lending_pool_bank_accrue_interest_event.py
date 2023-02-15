import typing
from dataclasses import dataclass
from solders.pubkey import Pubkey
import borsh_construct as borsh
from anchorpy.coder.event import EVENT_DISCRIMINATOR_SIZE
from anchorpy.error import EventInvalidDiscriminator
from anchorpy.borsh_extension import BorshPubkey
from .. import types


class LendingPoolBankAccrueInterestEventJSON(typing.TypedDict):
    header: types.group_event_header.GroupEventHeaderJSON
    mint: str
    delta: int
    fees_collected: float
    insurance_collected: float


@dataclass
class LendingPoolBankAccrueInterestEvent:
    discriminator: typing.ClassVar = b"hu\xbb\x9co\x9aj\xba"
    layout: typing.ClassVar = borsh.CStruct(
        "header" / types.group_event_header.GroupEventHeader.layout,
        "mint" / BorshPubkey,
        "delta" / borsh.U64,
        "fees_collected" / borsh.F64,
        "insurance_collected" / borsh.F64,
    )
    header: types.group_event_header.GroupEventHeader
    mint: Pubkey
    delta: int
    fees_collected: float
    insurance_collected: float

    @classmethod
    def decode(cls, data: bytes) -> "LendingPoolBankAccrueInterestEvent":
        if data[:EVENT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise EventInvalidDiscriminator(
                "The discriminator for this event is invalid"
            )
        dec = LendingPoolBankAccrueInterestEvent.layout.parse(
            data[EVENT_DISCRIMINATOR_SIZE:]
        )
        return cls(
            header=types.group_event_header.GroupEventHeader.from_decoded(dec.header),
            mint=dec.mint,
            delta=dec.delta,
            fees_collected=dec.fees_collected,
            insurance_collected=dec.insurance_collected,
        )

    def to_json(self) -> LendingPoolBankAccrueInterestEventJSON:
        return {
            "header": self.header.to_json(),
            "mint": str(self.mint),
            "delta": self.delta,
            "fees_collected": self.fees_collected,
            "insurance_collected": self.insurance_collected,
        }

    @classmethod
    def from_json(
        cls, obj: LendingPoolBankAccrueInterestEventJSON
    ) -> "LendingPoolBankAccrueInterestEvent":
        return cls(
            header=types.group_event_header.GroupEventHeader.from_json(obj["header"]),
            mint=Pubkey.from_string(obj["mint"]),
            delta=obj["delta"],
            fees_collected=obj["fees_collected"],
            insurance_collected=obj["insurance_collected"],
        )
