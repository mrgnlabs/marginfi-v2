use anchor_lang::{
    prelude::{AccountLoader, Rent},
    Discriminator, Key,
};
use anyhow::Result;
use marginfi::prelude::MarginfiGroup;
use marginfi_fuzz::{setup_marginfi_group, BankAndOracleConfig};

fn main() -> Result<()> {
    let bump = bumpalo::Bump::new();
    let mut a = setup_marginfi_group(&bump);

    a.setup_banks(&bump, Rent::free(), 1, &[BankAndOracleConfig::dummy()]);

    let al = AccountLoader::<MarginfiGroup>::try_from_unchecked(&marginfi::id(), &a.marginfi_group)
        .unwrap();

    assert_eq!(al.load().unwrap().admin, a.owner.key());

    Ok(())
}
