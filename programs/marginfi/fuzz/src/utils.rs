use solana_sdk::account_info::AccountInfo;

/// This is safe because it shortens lifetimes 'info: 'o and 'a: 'o to that of 'o
pub fn account_info_ref_lifetime_shortener<'info: 'a + 'o, 'a: 'o, 'o>(
    ai: &'a AccountInfo<'info>,
) -> &'o AccountInfo<'o> {
    unsafe { core::mem::transmute(ai) }
}

/// This is safe because it shortens lifetimes 'info: 'o to that of 'o
pub fn account_info_lifetime_shortener<'info: 'o, 'o>(ai: AccountInfo<'info>) -> AccountInfo<'o> {
    unsafe { core::mem::transmute(ai) }
}

/// This is safe because it shortens lifetimes 'info: 'o and 'a: 'o to that of 'o
pub fn account_info_slice_lifetime_shortener<'info: 'a + 'o, 'a: 'o, 'o>(
    ai: &'a [AccountInfo<'info>],
) -> &'o [AccountInfo<'o>] {
    unsafe { core::mem::transmute(ai) }
}
