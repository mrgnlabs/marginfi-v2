use fixed::types::I80F48;
use fixtures::{assert_eq_noise, marginfi_account::MarginfiAccountFixture, prelude::*};
use marginfi_type_crate::types::BankCache;
use solana_program_test::tokio;

async fn setup_borrow_with_price_cache(
) -> anyhow::Result<(TestFixture, MarginfiAccountFixture, TokenAccountFixture)> {
    let test_settings = TestSettings {
        banks: vec![
            TestBankSetting {
                mint: BankMint::Usdc,
                ..Default::default()
            },
            TestBankSetting {
                mint: BankMint::Sol,
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let test_f = TestFixture::new(Some(test_settings)).await;
    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let usdc_bank = test_f.get_bank(&BankMint::Usdc);

    let lp_account = test_f.create_marginfi_account().await;
    let lp_sol_account = test_f.sol_mint.create_token_account_and_mint_to(50.0).await;
    lp_account
        .try_bank_deposit(lp_sol_account.key, sol_bank, 50.0, None)
        .await?;

    let borrower = test_f.create_marginfi_account().await;
    let borrower_usdc = test_f
        .usdc_mint
        .create_token_account_and_mint_to(200.0)
        .await;
    borrower
        .try_bank_deposit(borrower_usdc.key, usdc_bank, 200.0, None)
        .await?;

    let borrower_sol = test_f.sol_mint.create_empty_token_account().await;
    borrower
        .try_bank_borrow(borrower_sol.key, sol_bank, 5.0)
        .await?;

    Ok((test_f, borrower, borrower_sol))
}

#[tokio::test]
async fn bank_cache_records_last_oracle_price_on_borrow() -> anyhow::Result<()> {
    let (test_f, _, _) = setup_borrow_with_price_cache().await?;

    let sol_bank = test_f.get_bank(&BankMint::Sol);
    let bank_after = sol_bank.load().await;
    let cache = bank_after.cache;

    let cached_price: I80F48 = cache.last_oracle_price.into();
    let cached_confidence: I80F48 = cache.last_oracle_price_confidence.into();
    let expected_price = I80F48::from_num(sol_bank.get_price().await);

    assert_eq_noise!(cached_price, expected_price);
    assert_eq!(cached_confidence, I80F48::ZERO);
    assert_eq!(
        cache.last_oracle_price_timestamp,
        test_f.get_clock().await.unix_timestamp
    );

    Ok(())
}

#[tokio::test]
async fn bank_cache_resets_after_full_repay() -> anyhow::Result<()> {
    let (test_f, borrower, borrower_sol_account) = setup_borrow_with_price_cache().await?;
    let sol_bank = test_f.get_bank(&BankMint::Sol);

    let bank_after_borrow = sol_bank.load().await;
    let price_in_cache: I80F48 = bank_after_borrow.cache.last_oracle_price.into();
    assert!(price_in_cache > I80F48::ZERO);

    borrower
        .try_bank_repay(borrower_sol_account.key, sol_bank, 0, Some(true))
        .await?;

    let bank_after_repay = sol_bank.load().await;
    assert_eq!(bank_after_repay.cache, BankCache::default());

    Ok(())
}
