use fixed::types::I80F48;
use marginfi::{
    constants::TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE,
    state::{
        marginfi_account::{
            calc_asset_value, Balance, BalanceSide, MarginfiAccount, RiskRequirementType,
            WeightType,
        },
        marginfi_group::Bank,
        price::{OraclePriceFeedAdapter, PriceAdapter},
    },
};
use solana_sdk::pubkey::Pubkey;

pub struct BankAccountWithPriceFeed2 {
    bank: Bank,
    price_feed: OraclePriceFeedAdapter,
    balance: Balance,
}

impl BankAccountWithPriceFeed2 {
    pub fn load(
        marginfi_account: &MarginfiAccount,
        banks: &std::collections::HashMap<Pubkey, Bank>,
        price_feeds: &std::collections::HashMap<Pubkey, OraclePriceFeedAdapter>,
    ) -> anyhow::Result<Vec<Self>> {
        marginfi_account
            .lending_account
            .balances
            .into_iter()
            .filter(|balance| balance.active)
            .enumerate()
            .map(|(_, balance)| {
                let bank = banks.get(&balance.bank_pk).cloned().unwrap();
                let price_feed = price_feeds
                    .get(&bank.config.oracle_keys[0])
                    .cloned()
                    .unwrap();
                Ok(BankAccountWithPriceFeed2 {
                    bank,
                    price_feed,
                    balance,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()
    }

    #[inline(always)]
    pub fn calc_weighted_assets_and_liabilities_values(
        &self,
        weight_type: WeightType,
    ) -> anyhow::Result<(I80F48, I80F48)> {
        let (worst_price, best_price) = self.price_feed.get_price_range()?;
        let (mut asset_weight, liability_weight) = self.bank.config.get_weights(weight_type);
        let mint_decimals = self.bank.mint_decimals;

        let asset_amount = self
            .bank
            .get_asset_amount(self.balance.asset_shares.into())?;
        let liability_amount = self
            .bank
            .get_liability_amount(self.balance.liability_shares.into())?;

        if matches!(weight_type, WeightType::Initial)
            && self.bank.config.total_asset_value_init_limit
                != TOTAL_ASSET_VALUE_INIT_LIMIT_INACTIVE
        {
            let bank_total_assets_value = calc_asset_value(
                self.bank
                    .get_asset_amount(self.bank.total_asset_shares.into())?,
                worst_price,
                mint_decimals,
                None,
            )?;

            let total_asset_value_init_limit =
                I80F48::from_num(self.bank.config.total_asset_value_init_limit);

            if bank_total_assets_value > total_asset_value_init_limit {
                let discount = total_asset_value_init_limit
                    .checked_div(bank_total_assets_value)
                    .unwrap();

                asset_weight = asset_weight.checked_mul(discount).unwrap();
            }
        }

        Ok((
            calc_asset_value(asset_amount, worst_price, mint_decimals, Some(asset_weight))?,
            calc_asset_value(
                liability_amount,
                best_price,
                mint_decimals,
                Some(liability_weight),
            )?,
        ))
    }

    #[inline]
    pub fn is_empty(&self, side: BalanceSide) -> bool {
        self.balance.is_empty(side)
    }
}

pub struct RiskEngine2 {
    bank_accounts_with_price: Vec<BankAccountWithPriceFeed2>,
}

impl RiskEngine2 {
    pub fn load(
        marginfi_account: &MarginfiAccount,
        banks: &std::collections::HashMap<Pubkey, Bank>,
        price_feeds: &std::collections::HashMap<Pubkey, OraclePriceFeedAdapter>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            bank_accounts_with_price: BankAccountWithPriceFeed2::load(
                marginfi_account,
                banks,
                price_feeds,
            )?,
        })
    }

    /// Returns the total assets and liabilities of the account in the form of (assets, liabilities)
    pub fn get_account_health_components(
        &self,
        requirement_type: RiskRequirementType,
    ) -> anyhow::Result<(I80F48, I80F48)> {
        let mut total_assets = I80F48::ZERO;
        let mut total_liabilities = I80F48::ZERO;

        for a in &self.bank_accounts_with_price {
            let (assets, liabilities) =
                a.calc_weighted_assets_and_liabilities_values(requirement_type.to_weight_type())?;

            total_assets = total_assets.checked_add(assets).unwrap();
            total_liabilities = total_liabilities.checked_add(liabilities).unwrap();
        }

        Ok((total_assets, total_liabilities))
    }

    pub fn get_equity_components(&self) -> anyhow::Result<(I80F48, I80F48)> {
        self.bank_accounts_with_price
            .iter()
            .map(|a: &BankAccountWithPriceFeed2| {
                a.calc_weighted_assets_and_liabilities_values(WeightType::Equity)
            })
            .try_fold(
                (I80F48::ZERO, I80F48::ZERO),
                |(total_assets, total_liabilities), res| {
                    let (assets, liabilities) = res?;
                    let total_assets_sum = total_assets.checked_add(assets).unwrap();
                    let total_liabilities_sum = total_liabilities.checked_add(liabilities).unwrap();

                    Ok::<_, anyhow::Error>((total_assets_sum, total_liabilities_sum))
                },
            )
    }

    pub fn get_account_health(
        &self,
        requirement_type: RiskRequirementType,
    ) -> anyhow::Result<I80F48> {
        let (total_weighted_assets, total_weighted_liabilities) =
            self.get_account_health_components(requirement_type)?;

        Ok(total_weighted_assets
            .checked_sub(total_weighted_liabilities)
            .unwrap())
    }
}
