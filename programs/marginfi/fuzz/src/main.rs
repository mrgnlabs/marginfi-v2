use anchor_lang::{
    prelude::{AccountLoader, Rent},
    Discriminator, Key,
};
use anyhow::Result;
use marginfi::prelude::MarginfiGroup;
use marginfi_fuzz::{setup_marginfi_group, AccountIdx, AssetAmount, BankAndOracleConfig, BankIdx};

fn main() -> Result<()> {
    let bump = bumpalo::Bump::new();
    let mut a = setup_marginfi_group(&bump);

    a.setup_banks(&bump, Rent::free(), 8, &[BankAndOracleConfig::dummy(); 8]);

    a.setup_users(&bump, Rent::free(), 1);

    let al = AccountLoader::<MarginfiGroup>::try_from_unchecked(&marginfi::id(), &a.marginfi_group)
        .unwrap();

    assert_eq!(al.load().unwrap().admin, a.owner.key());

    a.process_action_deposits(&AccountIdx(0), &BankIdx(0), &AssetAmount(1000))?;
    a.process_action_withdraw(&AccountIdx(0), &BankIdx(0), &AssetAmount(999))?;

    println!("Done!");

    Ok(())
}
