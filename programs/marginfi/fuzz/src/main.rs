use anchor_lang::{prelude::AccountLoader, Discriminator, Key};
use anyhow::Result;
use marginfi::prelude::MarginfiGroup;
use marginfi_fuzz::setup_marginfi_group;

fn main() -> Result<()> {
    let bump = bumpalo::Bump::new();
    let a = setup_marginfi_group(&bump);

    let al = AccountLoader::<MarginfiGroup>::try_from_unchecked(&marginfi::id(), &a.marginfi_group)
        .unwrap();

    assert_eq!(al.load().unwrap().admin, a.owner.key());

    Ok(())
}
