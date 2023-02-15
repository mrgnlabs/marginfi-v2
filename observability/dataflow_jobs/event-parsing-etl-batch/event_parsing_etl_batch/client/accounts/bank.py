import typing
from dataclasses import dataclass
from solders.pubkey import Pubkey
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Commitment
import borsh_construct as borsh
from anchorpy.coder.accounts import ACCOUNT_DISCRIMINATOR_SIZE
from anchorpy.error import AccountInvalidDiscriminator
from anchorpy.utils.rpc import get_multiple_accounts
from anchorpy.borsh_extension import BorshPubkey
from ..program_id import PROGRAM_ID
from .. import types


class BankJSON(typing.TypedDict):
    mint: str
    mint_decimals: int
    group: str
    asset_share_value: types.wrapped_i80f48.WrappedI80F48JSON
    liability_share_value: types.wrapped_i80f48.WrappedI80F48JSON
    liquidity_vault: str
    liquidity_vault_bump: int
    liquidity_vault_authority_bump: int
    insurance_vault: str
    insurance_vault_bump: int
    insurance_vault_authority_bump: int
    collected_insurance_fees_outstanding: types.wrapped_i80f48.WrappedI80F48JSON
    fee_vault: str
    fee_vault_bump: int
    fee_vault_authority_bump: int
    collected_group_fees_outstanding: types.wrapped_i80f48.WrappedI80F48JSON
    total_liability_shares: types.wrapped_i80f48.WrappedI80F48JSON
    total_asset_shares: types.wrapped_i80f48.WrappedI80F48JSON
    last_update: int
    config: types.bank_config.BankConfigJSON
    padding0: list[int]
    padding1: list[int]


@dataclass
class Bank:
    discriminator: typing.ClassVar = b"\x8e1\xa6\xf22Ba\xbc"
    layout: typing.ClassVar = borsh.CStruct(
        "mint" / BorshPubkey,
        "mint_decimals" / borsh.U8,
        "group" / BorshPubkey,
        "asset_share_value" / types.wrapped_i80f48.WrappedI80F48.layout,
        "liability_share_value" / types.wrapped_i80f48.WrappedI80F48.layout,
        "liquidity_vault" / BorshPubkey,
        "liquidity_vault_bump" / borsh.U8,
        "liquidity_vault_authority_bump" / borsh.U8,
        "insurance_vault" / BorshPubkey,
        "insurance_vault_bump" / borsh.U8,
        "insurance_vault_authority_bump" / borsh.U8,
        "collected_insurance_fees_outstanding"
        / types.wrapped_i80f48.WrappedI80F48.layout,
        "fee_vault" / BorshPubkey,
        "fee_vault_bump" / borsh.U8,
        "fee_vault_authority_bump" / borsh.U8,
        "collected_group_fees_outstanding" / types.wrapped_i80f48.WrappedI80F48.layout,
        "total_liability_shares" / types.wrapped_i80f48.WrappedI80F48.layout,
        "total_asset_shares" / types.wrapped_i80f48.WrappedI80F48.layout,
        "last_update" / borsh.I64,
        "config" / types.bank_config.BankConfig.layout,
        "padding0" / borsh.U128[32],
        "padding1" / borsh.U128[32],
    )
    mint: Pubkey
    mint_decimals: int
    group: Pubkey
    asset_share_value: types.wrapped_i80f48.WrappedI80F48
    liability_share_value: types.wrapped_i80f48.WrappedI80F48
    liquidity_vault: Pubkey
    liquidity_vault_bump: int
    liquidity_vault_authority_bump: int
    insurance_vault: Pubkey
    insurance_vault_bump: int
    insurance_vault_authority_bump: int
    collected_insurance_fees_outstanding: types.wrapped_i80f48.WrappedI80F48
    fee_vault: Pubkey
    fee_vault_bump: int
    fee_vault_authority_bump: int
    collected_group_fees_outstanding: types.wrapped_i80f48.WrappedI80F48
    total_liability_shares: types.wrapped_i80f48.WrappedI80F48
    total_asset_shares: types.wrapped_i80f48.WrappedI80F48
    last_update: int
    config: types.bank_config.BankConfig
    padding0: list[int]
    padding1: list[int]

    @classmethod
    async def fetch(
        cls,
        conn: AsyncClient,
        address: Pubkey,
        commitment: typing.Optional[Commitment] = None,
        program_id: Pubkey = PROGRAM_ID,
    ) -> typing.Optional["Bank"]:
        resp = await conn.get_account_info(address, commitment=commitment)
        info = resp.value
        if info is None:
            return None
        if info.owner != program_id:
            raise ValueError("Account does not belong to this program")
        bytes_data = info.data
        return cls.decode(bytes_data)

    @classmethod
    async def fetch_multiple(
        cls,
        conn: AsyncClient,
        addresses: list[Pubkey],
        commitment: typing.Optional[Commitment] = None,
        program_id: Pubkey = PROGRAM_ID,
    ) -> typing.List[typing.Optional["Bank"]]:
        infos = await get_multiple_accounts(conn, addresses, commitment=commitment)
        res: typing.List[typing.Optional["Bank"]] = []
        for info in infos:
            if info is None:
                res.append(None)
                continue
            if info.account.owner != program_id:
                raise ValueError("Account does not belong to this program")
            res.append(cls.decode(info.account.data))
        return res

    @classmethod
    def decode(cls, data: bytes) -> "Bank":
        if data[:ACCOUNT_DISCRIMINATOR_SIZE] != cls.discriminator:
            raise AccountInvalidDiscriminator(
                "The discriminator for this account is invalid"
            )
        dec = Bank.layout.parse(data[ACCOUNT_DISCRIMINATOR_SIZE:])
        return cls(
            mint=dec.mint,
            mint_decimals=dec.mint_decimals,
            group=dec.group,
            asset_share_value=types.wrapped_i80f48.WrappedI80F48.from_decoded(
                dec.asset_share_value
            ),
            liability_share_value=types.wrapped_i80f48.WrappedI80F48.from_decoded(
                dec.liability_share_value
            ),
            liquidity_vault=dec.liquidity_vault,
            liquidity_vault_bump=dec.liquidity_vault_bump,
            liquidity_vault_authority_bump=dec.liquidity_vault_authority_bump,
            insurance_vault=dec.insurance_vault,
            insurance_vault_bump=dec.insurance_vault_bump,
            insurance_vault_authority_bump=dec.insurance_vault_authority_bump,
            collected_insurance_fees_outstanding=types.wrapped_i80f48.WrappedI80F48.from_decoded(
                dec.collected_insurance_fees_outstanding
            ),
            fee_vault=dec.fee_vault,
            fee_vault_bump=dec.fee_vault_bump,
            fee_vault_authority_bump=dec.fee_vault_authority_bump,
            collected_group_fees_outstanding=types.wrapped_i80f48.WrappedI80F48.from_decoded(
                dec.collected_group_fees_outstanding
            ),
            total_liability_shares=types.wrapped_i80f48.WrappedI80F48.from_decoded(
                dec.total_liability_shares
            ),
            total_asset_shares=types.wrapped_i80f48.WrappedI80F48.from_decoded(
                dec.total_asset_shares
            ),
            last_update=dec.last_update,
            config=types.bank_config.BankConfig.from_decoded(dec.config),
            padding0=dec.padding0,
            padding1=dec.padding1,
        )

    def to_json(self) -> BankJSON:
        return {
            "mint": str(self.mint),
            "mint_decimals": self.mint_decimals,
            "group": str(self.group),
            "asset_share_value": self.asset_share_value.to_json(),
            "liability_share_value": self.liability_share_value.to_json(),
            "liquidity_vault": str(self.liquidity_vault),
            "liquidity_vault_bump": self.liquidity_vault_bump,
            "liquidity_vault_authority_bump": self.liquidity_vault_authority_bump,
            "insurance_vault": str(self.insurance_vault),
            "insurance_vault_bump": self.insurance_vault_bump,
            "insurance_vault_authority_bump": self.insurance_vault_authority_bump,
            "collected_insurance_fees_outstanding": self.collected_insurance_fees_outstanding.to_json(),
            "fee_vault": str(self.fee_vault),
            "fee_vault_bump": self.fee_vault_bump,
            "fee_vault_authority_bump": self.fee_vault_authority_bump,
            "collected_group_fees_outstanding": self.collected_group_fees_outstanding.to_json(),
            "total_liability_shares": self.total_liability_shares.to_json(),
            "total_asset_shares": self.total_asset_shares.to_json(),
            "last_update": self.last_update,
            "config": self.config.to_json(),
            "padding0": self.padding0,
            "padding1": self.padding1,
        }

    @classmethod
    def from_json(cls, obj: BankJSON) -> "Bank":
        return cls(
            mint=Pubkey.from_string(obj["mint"]),
            mint_decimals=obj["mint_decimals"],
            group=Pubkey.from_string(obj["group"]),
            asset_share_value=types.wrapped_i80f48.WrappedI80F48.from_json(
                obj["asset_share_value"]
            ),
            liability_share_value=types.wrapped_i80f48.WrappedI80F48.from_json(
                obj["liability_share_value"]
            ),
            liquidity_vault=Pubkey.from_string(obj["liquidity_vault"]),
            liquidity_vault_bump=obj["liquidity_vault_bump"],
            liquidity_vault_authority_bump=obj["liquidity_vault_authority_bump"],
            insurance_vault=Pubkey.from_string(obj["insurance_vault"]),
            insurance_vault_bump=obj["insurance_vault_bump"],
            insurance_vault_authority_bump=obj["insurance_vault_authority_bump"],
            collected_insurance_fees_outstanding=types.wrapped_i80f48.WrappedI80F48.from_json(
                obj["collected_insurance_fees_outstanding"]
            ),
            fee_vault=Pubkey.from_string(obj["fee_vault"]),
            fee_vault_bump=obj["fee_vault_bump"],
            fee_vault_authority_bump=obj["fee_vault_authority_bump"],
            collected_group_fees_outstanding=types.wrapped_i80f48.WrappedI80F48.from_json(
                obj["collected_group_fees_outstanding"]
            ),
            total_liability_shares=types.wrapped_i80f48.WrappedI80F48.from_json(
                obj["total_liability_shares"]
            ),
            total_asset_shares=types.wrapped_i80f48.WrappedI80F48.from_json(
                obj["total_asset_shares"]
            ),
            last_update=obj["last_update"],
            config=types.bank_config.BankConfig.from_json(obj["config"]),
            padding0=obj["padding0"],
            padding1=obj["padding1"],
        )
