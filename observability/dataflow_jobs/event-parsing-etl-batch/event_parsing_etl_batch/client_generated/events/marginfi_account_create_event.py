import typing
from dataclasses import dataclass
import borsh_construct as borsh
from anchorpy.coder.event import EVENT_DISCRIMINATOR_SIZE
from anchorpy.error import EventInvalidDiscriminator
from .. import types


class MarginfiAccountCreateEventJSON(typing.TypedDict):
    header: types.account_event_header.AccountEventHeaderJSON


@dataclass
class MarginfiAccountCreateEvent:
    discriminator: typing.ClassVar = b"\xb7\x05uhz\xc7D3"
    layout: typing.ClassVar = borsh.CStruct(
        "header" / types.account_event_header.AccountEventHeader.layout
    )
    header: types.account_event_header.AccountEventHeader

    @classmethod
    def decode(cls, data: bytes) -> "MarginfiAccountCreateEvent":
        if data[:EVENT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise EventInvalidDiscriminator(
                "The discriminator for this event is invalid"
            )
        dec = MarginfiAccountCreateEvent.layout.parse(data[EVENT_DISCRIMINATOR_SIZE:])
        return cls(
            header=types.account_event_header.AccountEventHeader.from_decoded(
                dec.header
            ),
        )

    def to_json(self) -> MarginfiAccountCreateEventJSON:
        return {
            "header": self.header.to_json(),
        }

    @classmethod
    def from_json(
        cls, obj: MarginfiAccountCreateEventJSON
    ) -> "MarginfiAccountCreateEvent":
        return cls(
            header=types.account_event_header.AccountEventHeader.from_json(
                obj["header"]
            ),
        )
