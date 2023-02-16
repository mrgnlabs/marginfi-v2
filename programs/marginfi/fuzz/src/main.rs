use anchor_lang::{
    prelude::{AccountLoader, Rent},
    Key,
};
use anyhow::Result;
use fixed::types::I80F48;
use fixed_macro::types::I80F48;
use marginfi::{prelude::MarginfiGroup, state::marginfi_account::MarginfiAccount};
use marginfi_fuzz::{
    setup_marginfi_group, AccountIdx, AssetAmount, BankAndOracleConfig, BankIdx,
    MarginfiGroupAccounts, N_BANKS, N_USERS,
};

fn main() -> Result<()> {
    let bump = bumpalo::Bump::new();
    let a = MarginfiGroupAccounts::setup(
        &bump,
        &[BankAndOracleConfig::dummy(); N_BANKS as usize],
        N_USERS as usize,
    );
    let al = AccountLoader::<MarginfiGroup>::try_from_unchecked(&marginfi::id(), &a.marginfi_group)
        .unwrap();

    assert_eq!(al.load().unwrap().admin, a.owner.key());

    a.process_action_deposits(&AccountIdx(1), &BankIdx(1), &AssetAmount(1000))?;
    a.process_action_deposits(&AccountIdx(0), &BankIdx(0), &AssetAmount(1000))?;
    // a.process_action_withdraw(&AccountIdx(0), &BankIdx(0), &AssetAmount(999), Some(false))?;
    a.process_action_borrow(&AccountIdx(0), &BankIdx(1), &AssetAmount(999))?;

    let mfial = AccountLoader::<MarginfiAccount>::try_from(&a.marginfi_accounts[0].margin_account)?;
    let mfia = mfial.load()?;

    assert_eq!(
        I80F48::from(mfia.lending_account.balances[0].asset_shares),
        I80F48!(1000)
    );
    assert_eq!(
        I80F48::from(mfia.lending_account.balances[1].liability_shares),
        I80F48!(999)
    );

    println!("Done!");

    Ok(())
}
