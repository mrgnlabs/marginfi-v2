import typing
from dataclasses import dataclass
from solders.pubkey import Pubkey
import borsh_construct as borsh
from anchorpy.coder.event import EVENT_DISCRIMINATOR_SIZE
from anchorpy.error import EventInvalidDiscriminator
from anchorpy.borsh_extension import BorshPubkey
from .. import types


class LendingAccountDepositEventJSON(typing.TypedDict):
    header: types.account_event_header.AccountEventHeaderJSON
    bank: str
    mint: str
    amount: int


@dataclass
class LendingAccountDepositEvent:
    discriminator: typing.ClassVar = b"\xa16\xed\xd9i\xf8z\x97"
    layout: typing.ClassVar = borsh.CStruct(
        "header" / types.account_event_header.AccountEventHeader.layout,
        "bank" / BorshPubkey,
        "mint" / BorshPubkey,
        "amount" / borsh.U64,
    )
    header: types.account_event_header.AccountEventHeader
    bank: Pubkey
    mint: Pubkey
    amount: int

    @classmethod
    def decode(cls, data: bytes) -> "LendingAccountDepositEvent":
        if data[:EVENT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise EventInvalidDiscriminator(
                "The discriminator for this event is invalid"
            )
        dec = LendingAccountDepositEvent.layout.parse(data[EVENT_DISCRIMINATOR_SIZE:])
        return cls(
            header=types.account_event_header.AccountEventHeader.from_decoded(
                dec.header
            ),
            bank=dec.bank,
            mint=dec.mint,
            amount=dec.amount,
        )

    def to_json(self) -> LendingAccountDepositEventJSON:
        return {
            "header": self.header.to_json(),
            "bank": str(self.bank),
            "mint": str(self.mint),
            "amount": self.amount,
        }

    @classmethod
    def from_json(
        cls, obj: LendingAccountDepositEventJSON
    ) -> "LendingAccountDepositEvent":
        return cls(
            header=types.account_event_header.AccountEventHeader.from_json(
                obj["header"]
            ),
            bank=Pubkey.from_string(obj["bank"]),
            mint=Pubkey.from_string(obj["mint"]),
            amount=obj["amount"],
        )
