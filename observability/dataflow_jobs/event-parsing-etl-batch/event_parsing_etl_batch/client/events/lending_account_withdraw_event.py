import typing
from dataclasses import dataclass
from solders.pubkey import Pubkey
import borsh_construct as borsh
from anchorpy.coder.event import EVENT_DISCRIMINATOR_SIZE
from anchorpy.error import EventInvalidDiscriminator
from anchorpy.borsh_extension import BorshPubkey
from .. import types


class LendingAccountWithdrawEventJSON(typing.TypedDict):
    header: types.account_event_header.AccountEventHeaderJSON
    bank: str
    mint: str
    amount: int
    close_balance: bool


@dataclass
class LendingAccountWithdrawEvent:
    discriminator: typing.ClassVar = b"\x03\xdc\x94\xf3!\xf96X"
    layout: typing.ClassVar = borsh.CStruct(
        "header" / types.account_event_header.AccountEventHeader.layout,
        "bank" / BorshPubkey,
        "mint" / BorshPubkey,
        "amount" / borsh.U64,
        "close_balance" / borsh.Bool,
    )
    header: types.account_event_header.AccountEventHeader
    bank: Pubkey
    mint: Pubkey
    amount: int
    close_balance: bool

    @classmethod
    def decode(cls, data: bytes) -> "LendingAccountWithdrawEvent":
        if data[:EVENT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise EventInvalidDiscriminator(
                "The discriminator for this event is invalid"
            )
        dec = LendingAccountWithdrawEvent.layout.parse(data[EVENT_DISCRIMINATOR_SIZE:])
        return cls(
            header=types.account_event_header.AccountEventHeader.from_decoded(
                dec.header
            ),
            bank=dec.bank,
            mint=dec.mint,
            amount=dec.amount,
            close_balance=dec.close_balance,
        )

    def to_json(self) -> LendingAccountWithdrawEventJSON:
        return {
            "header": self.header.to_json(),
            "bank": str(self.bank),
            "mint": str(self.mint),
            "amount": self.amount,
            "close_balance": self.close_balance,
        }

    @classmethod
    def from_json(
        cls, obj: LendingAccountWithdrawEventJSON
    ) -> "LendingAccountWithdrawEvent":
        return cls(
            header=types.account_event_header.AccountEventHeader.from_json(
                obj["header"]
            ),
            bank=Pubkey.from_string(obj["bank"]),
            mint=Pubkey.from_string(obj["mint"]),
            amount=obj["amount"],
            close_balance=obj["close_balance"],
        )
