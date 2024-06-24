use std::{fs::File, io::Read, path::PathBuf, str::FromStr};

use anchor_lang::AccountDeserialize;
use anyhow::bail;
use base64::{prelude::BASE64_STANDARD, Engine};
use fixed::types::I80F48;
use marginfi::state::marginfi_account::MarginfiAccount;
use solana_account_decoder::UiAccountData;
use solana_cli_output::CliAccount;
use solana_program::pubkey;
use solana_program_test::tokio;

#[tokio::test]
async fn account_field_values_reg() -> anyhow::Result<()> {
    let account_fixtures_path = "tests/fixtures/marginfi_account";

    // Sample 1

    let mut path = PathBuf::from_str(account_fixtures_path).unwrap();
    path.push("marginfi_account_sample_1.json");
    let mut file = File::open(&path).unwrap();
    let mut account_info_raw = String::new();
    file.read_to_string(&mut account_info_raw).unwrap();

    let account: CliAccount = serde_json::from_str(&account_info_raw).unwrap();
    let UiAccountData::Binary(data, _) = account.keyed_account.account.data else {
        bail!("Expecting Binary format for fixtures")
    };
    let account = MarginfiAccount::try_deserialize(&mut BASE64_STANDARD.decode(data)?.as_slice())?;

    assert_eq!(
        account.authority,
        pubkey!("Dq7wypbedtaqQK9QqEFvfrxc4ppfRGXCeTVd7ee7n2jw")
    );

    let balance_1 = account.lending_account.balances[0];
    assert!(balance_1.active);
    assert_eq!(
        I80F48::from(balance_1.asset_shares),
        I80F48::from_str("1650216221.466876226897366").unwrap()
    );
    assert_eq!(
        I80F48::from(balance_1.liability_shares),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(balance_1.emissions_outstanding),
        I80F48::from_str("0").unwrap()
    );

    let balance_2 = account.lending_account.balances[1];
    assert!(balance_2.active);
    assert_eq!(
        I80F48::from(balance_2.asset_shares),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(balance_2.liability_shares),
        I80F48::from_str("3806372611.588862122556122").unwrap()
    );
    assert_eq!(
        I80F48::from(balance_2.emissions_outstanding),
        I80F48::from_str("0").unwrap()
    );

    // Sample 2

    let mut path = PathBuf::from_str(account_fixtures_path).unwrap();
    path.push("marginfi_account_sample_2.json");
    let mut file = File::open(&path).unwrap();
    let mut account_info_raw = String::new();
    file.read_to_string(&mut account_info_raw).unwrap();

    let account: CliAccount = serde_json::from_str(&account_info_raw).unwrap();
    let UiAccountData::Binary(data, _) = account.keyed_account.account.data else {
        bail!("Expecting Binary format for fixtures")
    };
    let account = MarginfiAccount::try_deserialize(&mut BASE64_STANDARD.decode(data)?.as_slice())?;

    assert_eq!(
        account.authority,
        pubkey!("3T1kGHp7CrdeW9Qj1t8NMc2Ks233RyvzVhoaUPWoBEFK")
    );

    let balance_1 = account.lending_account.balances[0];
    assert!(balance_1.active);
    assert_eq!(
        I80F48::from(balance_1.asset_shares),
        I80F48::from_str("470.952530958931234").unwrap()
    );
    assert_eq!(
        I80F48::from(balance_1.liability_shares),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(balance_1.emissions_outstanding),
        I80F48::from_str("26891413.388324654086347").unwrap()
    );

    let balance_2 = account.lending_account.balances[1];
    assert!(!balance_2.active);
    assert_eq!(
        I80F48::from(balance_2.asset_shares),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(balance_2.liability_shares),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(balance_2.emissions_outstanding),
        I80F48::from_str("0").unwrap()
    );

    // Sample 3

    let mut path = PathBuf::from_str(account_fixtures_path).unwrap();
    path.push("marginfi_account_sample_3.json");
    let mut file = File::open(&path).unwrap();
    let mut account_info_raw = String::new();
    file.read_to_string(&mut account_info_raw).unwrap();

    let account: CliAccount = serde_json::from_str(&account_info_raw).unwrap();
    let UiAccountData::Binary(data, _) = account.keyed_account.account.data else {
        bail!("Expecting Binary format for fixtures")
    };
    let account = MarginfiAccount::try_deserialize(&mut BASE64_STANDARD.decode(data)?.as_slice())?;

    assert_eq!(
        account.authority,
        pubkey!("7hmfVTuXc7HeX3YQjpiCXGVQuTeXonzjp795jorZukVR")
    );

    let balance_1 = account.lending_account.balances[0];
    assert!(!balance_1.active);
    assert_eq!(
        I80F48::from(balance_1.asset_shares),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(balance_1.liability_shares),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(balance_1.emissions_outstanding),
        I80F48::from_str("0").unwrap()
    );

    Ok(())
}
