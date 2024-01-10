use fixed::types::I80F48;
use marginfi::state::{
    marginfi_account::{
        calc_value, Balance, BalanceSide, MarginfiAccount, RequirementType, RiskRequirementType,
    },
    marginfi_group::{Bank, RiskTier},
    price::{OraclePriceFeedAdapter, PriceAdapter, PriceBias},
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
        requirement_type: RequirementType,
    ) -> anyhow::Result<(I80F48, I80F48)> {
        match self.balance.get_side() {
            Some(side) => {
                let bank = &self.bank;
                match side {
                    BalanceSide::Assets => Ok((
                        self.calc_weighted_assets(requirement_type, bank)?,
                        I80F48::ZERO,
                    )),
                    BalanceSide::Liabilities => Ok((
                        I80F48::ZERO,
                        self.calc_weighted_liabs(requirement_type, bank)?,
                    )),
                }
            }
            None => Ok((I80F48::ZERO, I80F48::ZERO)),
        }
    }

    #[inline(always)]
    fn calc_weighted_assets(
        &self,
        requirement_type: RequirementType,
        bank: &Bank,
    ) -> anyhow::Result<I80F48> {
        match bank.config.risk_tier {
            RiskTier::Collateral => {
                let price_feed = &self.price_feed;
                let mut asset_weight = bank
                    .config
                    .get_weight(requirement_type, BalanceSide::Assets);

                let lower_price = price_feed.get_price_of_type(
                    requirement_type.get_oracle_price_type(),
                    Some(PriceBias::Low),
                )?;

                if matches!(requirement_type, RequirementType::Initial) {
                    if let Some(discount) =
                        bank.maybe_get_asset_weight_init_discount(lower_price)?
                    {
                        asset_weight = asset_weight
                            .checked_mul(discount)
                            .ok_or_else(|| anyhow::anyhow!("Math error"))?;
                    }
                }

                Ok(calc_value(
                    bank.get_asset_amount(self.balance.asset_shares.into())?,
                    lower_price,
                    bank.mint_decimals,
                    Some(asset_weight),
                )
                .unwrap())
            }
            RiskTier::Isolated => Ok(I80F48::ZERO),
        }
    }

    #[inline(always)]
    fn calc_weighted_liabs(
        &self,
        requirement_type: RequirementType,
        bank: &Bank,
    ) -> anyhow::Result<I80F48> {
        let price_feed = &self.price_feed;
        let liability_weight = bank
            .config
            .get_weight(requirement_type, BalanceSide::Liabilities);

        let higher_price = price_feed.get_price_of_type(
            requirement_type.get_oracle_price_type(),
            Some(PriceBias::High),
        )?;

        Ok(calc_value(
            bank.get_liability_amount(self.balance.liability_shares.into())?,
            higher_price,
            bank.mint_decimals,
            Some(liability_weight),
        )?)
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
                a.calc_weighted_assets_and_liabilities_values(RequirementType::Equity)
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
