use std::{fs::File, io::Read, path::PathBuf, str::FromStr};

use anchor_lang::AccountDeserialize;
use anyhow::bail;
use base64::{prelude::BASE64_STANDARD, Engine};
use fixed::types::I80F48;
use marginfi::state::{marginfi_account::MarginfiAccount, marginfi_group::Bank};
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

#[tokio::test]
async fn bank_field_values_reg() -> anyhow::Result<()> {
    let bank_fixtures_path = "tests/fixtures/bank";

    // Sample 1 (Jito)

    let mut path = PathBuf::from_str(bank_fixtures_path).unwrap();
    path.push("bank_sample_1.json");
    let mut file = File::open(&path).unwrap();
    let mut account_info_raw = String::new();
    file.read_to_string(&mut account_info_raw).unwrap();

    let account: CliAccount = serde_json::from_str(&account_info_raw).unwrap();
    let UiAccountData::Binary(data, _) = account.keyed_account.account.data else {
        bail!("Expecting Binary format for fixtures")
    };
    let bank = Bank::try_deserialize(&mut BASE64_STANDARD.decode(data)?.as_slice())?;

    assert_eq!(
        bank.mint,
        pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn")
    );
    assert_eq!(bank.mint_decimals, 9);
    assert_eq!(
        bank.group,
        pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8")
    );
    assert_eq!(
        I80F48::from(bank.asset_share_value),
        I80F48::from_str("1.000561264812955").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.liability_share_value),
        I80F48::from_str("1.00737674726716").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_insurance_fees_outstanding),
        I80F48::from_str("61174.580321107215052").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_group_fees_outstanding),
        I80F48::from_str("35660072279.35465946938668").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_liability_shares),
        I80F48::from_str("79763493059362.858709822356737").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_asset_shares),
        I80F48::from_str("998366336320727.44918120920092").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_init),
        I80F48::from_str("0.649999976158142").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_maint),
        I80F48::from_str("0.80000001192093").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_init),
        I80F48::from_str("1.3").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_maint),
        I80F48::from_str("1.2").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.optimal_utilization_rate),
        I80F48::from_str("0.8").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.plateau_interest_rate),
        I80F48::from_str("0.1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.max_interest_rate),
        I80F48::from_str("3").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_fee_fixed_apr),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_ir_fee),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_fixed_fee_apr),
        I80F48::from_str("0.01").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_ir_fee),
        I80F48::from_str("0.05").unwrap()
    );

    // Sample 2 (META)

    let mut path = PathBuf::from_str(bank_fixtures_path).unwrap();
    path.push("bank_sample_2.json");
    let mut file = File::open(&path).unwrap();
    let mut account_info_raw = String::new();
    file.read_to_string(&mut account_info_raw).unwrap();

    let account: CliAccount = serde_json::from_str(&account_info_raw).unwrap();
    let UiAccountData::Binary(data, _) = account.keyed_account.account.data else {
        bail!("Expecting Binary format for fixtures")
    };
    let bank = Bank::try_deserialize(&mut BASE64_STANDARD.decode(data)?.as_slice())?;

    assert_eq!(
        bank.mint,
        pubkey!("METADDFL6wWMWEoKTFJwcThTbUmtarRJZjRpzUvkxhr")
    );
    assert_eq!(bank.mint_decimals, 9);
    assert_eq!(
        bank.group,
        pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8")
    );
    assert_eq!(
        I80F48::from(bank.asset_share_value),
        I80F48::from_str("1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.liability_share_value),
        I80F48::from_str("1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_insurance_fees_outstanding),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_group_fees_outstanding),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_liability_shares),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_asset_shares),
        I80F48::from_str("698503862367").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_init),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_maint),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_init),
        I80F48::from_str("2.5").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_maint),
        I80F48::from_str("1.5").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.optimal_utilization_rate),
        I80F48::from_str("0.8").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.plateau_interest_rate),
        I80F48::from_str("0.1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.max_interest_rate),
        I80F48::from_str("3").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_fee_fixed_apr),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_ir_fee),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_fixed_fee_apr),
        I80F48::from_str("0.01").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_ir_fee),
        I80F48::from_str("0.05").unwrap()
    );

    // Sample 3 (USDT)

    let mut path = PathBuf::from_str(bank_fixtures_path).unwrap();
    path.push("bank_sample_3.json");
    let mut file = File::open(&path).unwrap();
    let mut account_info_raw = String::new();
    file.read_to_string(&mut account_info_raw).unwrap();

    let account: CliAccount = serde_json::from_str(&account_info_raw).unwrap();
    let UiAccountData::Binary(data, _) = account.keyed_account.account.data else {
        bail!("Expecting Binary format for fixtures")
    };
    let bank = Bank::try_deserialize(&mut BASE64_STANDARD.decode(data)?.as_slice())?;

    assert_eq!(
        bank.mint,
        pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB")
    );
    assert_eq!(bank.mint_decimals, 6);
    assert_eq!(
        bank.group,
        pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8")
    );
    assert_eq!(
        I80F48::from(bank.asset_share_value),
        I80F48::from_str("1.063003765188338").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.liability_share_value),
        I80F48::from_str("1.12089611736063").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_insurance_fees_outstanding),
        I80F48::from_str("45839.746526861401865").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.collected_group_fees_outstanding),
        I80F48::from_str("28634360131.219557095675654").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_liability_shares),
        I80F48::from_str("32109684419718.204607882232235").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.total_asset_shares),
        I80F48::from_str("43231381120800.339303417329994").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_init),
        I80F48::from_str("1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.asset_weight_maint),
        I80F48::from_str("1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_init),
        I80F48::from_str("1.25").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.liability_weight_maint),
        I80F48::from_str("1.1").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.optimal_utilization_rate),
        I80F48::from_str("0.8").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.plateau_interest_rate),
        I80F48::from_str("0.2").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.max_interest_rate),
        I80F48::from_str("4").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_fee_fixed_apr),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.insurance_ir_fee),
        I80F48::from_str("0").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_fixed_fee_apr),
        I80F48::from_str("0.01").unwrap()
    );
    assert_eq!(
        I80F48::from(bank.config.interest_rate_config.protocol_ir_fee),
        I80F48::from_str("0.05").unwrap()
    );

    Ok(())
}
