use arbitrary::Arbitrary;
use fixed_macro::types::I80F48;
use marginfi::state::marginfi_group::WrappedI80F48;
use strum::{EnumDiscriminants, VariantArray};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PriceChange(pub i64);

impl<'a> Arbitrary<'a> for PriceChange {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        if u.is_empty() {
            panic!("Byte exhaustion detected, stopping early");
        }
        Ok(Self(u.int_in_range(0..=1_000_000_000_000)?))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountIdx(pub u8);
pub const N_USERS: usize = 2;
impl<'a> Arbitrary<'a> for AccountIdx {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        if u.is_empty() {
            panic!("Byte exhaustion detected, stopping early");
        }
        let i: u8 = u.int_in_range(0..=N_USERS as u8 - 1)?;
        Ok(AccountIdx(i))
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (1, Some(1))
    }

    fn arbitrary_take_rest(mut u: arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Self::arbitrary(&mut u)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BankIdx(pub u8);
pub const N_BANKS: usize = 16;
impl<'a> Arbitrary<'a> for BankIdx {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        if u.is_empty() {
            panic!("Byte exhaustion detected, stopping early");
        }
        Ok(BankIdx(u.int_in_range(0..=N_BANKS - 1)? as u8))
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (1, Some(1))
    }

    fn arbitrary_take_rest(mut u: arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Self::arbitrary(&mut u)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetAmount(pub u64);

pub const ASSET_UNIT: u64 = 1_000_000_000;
impl<'a> Arbitrary<'a> for AssetAmount {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        if u.is_empty() {
            panic!("Byte exhaustion detected, stopping early");
        }
        Ok(AssetAmount(u.int_in_range(1..=10)? * ASSET_UNIT))
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (8, Some(8))
    }

    fn arbitrary_take_rest(mut u: arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Self::arbitrary(&mut u)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BankAndOracleConfig {
    pub oracle_native_price: u64,
    pub mint_decimals: u8,

    pub asset_weight_init: WrappedI80F48,
    pub asset_weight_maint: WrappedI80F48,

    pub liability_weight_init: WrappedI80F48,
    pub liability_weight_maint: WrappedI80F48,

    pub deposit_limit: u64,
    pub borrow_limit: u64,
    pub token_type: TokenType,

    pub risk_tier_isolated: bool,
}

impl<'a> Arbitrary<'a> for BankAndOracleConfig {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mint_decimals = u.int_in_range(2..=3)? * 3;
        let top_limit = 1_000_000 * 10u64.pow(mint_decimals as u32);
        let borrow_limit = u.int_in_range(1..=10)? * top_limit;
        let deposit_limit = borrow_limit + u.int_in_range(1..=10)? * top_limit;

        let max_price = 100 * 10u64.pow(mint_decimals as u32);

        let risk_tier_isolated: bool = u.arbitrary()?;

        let token_type = u.arbitrary()?;

        Ok(Self {
            oracle_native_price: u.int_in_range(1..=10)? * max_price,
            mint_decimals,
            asset_weight_init: if !risk_tier_isolated {
                I80F48!(0.5).into()
            } else {
                I80F48!(0).into()
            },
            asset_weight_maint: if !risk_tier_isolated {
                I80F48!(0.75).into()
            } else {
                I80F48!(0).into()
            },
            token_type,
            liability_weight_init: I80F48!(1.5).into(),
            liability_weight_maint: I80F48!(1.25).into(),
            deposit_limit,
            borrow_limit,
            risk_tier_isolated,
        })
    }
}

impl BankAndOracleConfig {
    pub fn dummy() -> Self {
        Self {
            oracle_native_price: 10 * 10u64.pow(6),
            mint_decimals: 6,
            asset_weight_init: I80F48!(0.75).into(),
            asset_weight_maint: I80F48!(0.8).into(),
            liability_weight_init: I80F48!(1.2).into(),
            liability_weight_maint: I80F48!(1.1).into(),
            deposit_limit: 1_000_000_000_000 * 10u64.pow(6),
            borrow_limit: 1_000_000_000_000 * 10u64.pow(6),
            token_type: TokenType::Tokenkeg,
            risk_tier_isolated: false,
        }
    }
}

#[derive(Debug, Clone, Copy, EnumDiscriminants)]
#[strum_discriminants(derive(VariantArray))]
pub enum TokenType {
    Tokenkeg,
    Token22,
    Token22WithFee {
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    },
}

impl<'a> Arbitrary<'a> for TokenType {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let discriminant = u.choose(&TokenTypeDiscriminants::VARIANTS)?;

        match discriminant {
            TokenTypeDiscriminants::Tokenkeg => Ok(TokenType::Tokenkeg),
            TokenTypeDiscriminants::Token22 => Ok(TokenType::Token22),
            TokenTypeDiscriminants::Token22WithFee => {
                // Get fee
                let fee_bps: u16 = u.int_in_range(0..=10_000)?;

                // Get max fee
                let max_fee: u64 = u.int_in_range(0..=1_000_000)?;

                Ok(TokenType::Token22WithFee {
                    transfer_fee_basis_points: fee_bps,
                    maximum_fee: max_fee,
                })
            }
        }
    }
}
