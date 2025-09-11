use crate::{
    bank_signer, check,
    prelude::*,
    state::bank::{BankImpl, BankVaultType},
    utils,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use marginfi_type_crate::{
    constants::LIQUIDITY_VAULT_AUTHORITY_SEED,
    types::{Bank, BankOperationalState, MarginfiGroup},
};

// This tx is a one-time tx to sunset the Arena product and refund users OTC. Only Arena banks
const ALLOWED_ARENA_BANK_KEYS: &[Pubkey] = &[
    pubkey!("9qw3SryJCQKXckJwSiHWxaibGrR9iXHr2JkusTW7J1Ye"),
    pubkey!("59yr2vuW1qv3UVQx9HC6Q8mxns5S6g7fjS8YWgRgaLA7"),
    pubkey!("34haqTJQS8gXEuRxPyzwEPQ1w3C7J9mdWjkgCpkD87YN"),
    pubkey!("7M1YiL5R822qMQjtEDreGCrXwJAAnPcw7LYnZXfq24Gs"),
    pubkey!("A7vBgCowCYeja7GTc3pyqUBdC9Gkue2gWaMjGZW38meM"),
    pubkey!("Dj3PndQ3j1vuga5ApiFWWAfQ4h3wBtgS2SeLZBT2LD4g"),
    pubkey!("8AJqtD4LSwFhZ91nNCRiEfXTv3DqvSvZzZh42h7KKUsY"),
    pubkey!("C5sidFUZ8cqZSBxaK8seBW1k3g3For8NHMSo64SYe98n"),
    pubkey!("ATePKG1xadGgLFS9d4aR8PbQTDxsiuw1gzeiHVpwpgTS"),
    pubkey!("4dUia8ru6fazJJr6xjpiH8VTbuMmtqTLKcQEQKw2YHPN"),
    pubkey!("H74n2jNaQkgworPbs3qc6kBAosmtz4odZRbKQVjv94kn"),
    pubkey!("HWza7EWdQZSvZhRPsN2uSVieYZRNCTKtDj8niCEAgVaD"),
    pubkey!("845oEvt1oduoBj5zQxTr21cWWaUVnRjGerJuW3yMo2nn"),
    pubkey!("EXrnNVfLagt3j4hCHSD9WqK75o6dkZBtjpnrSrSC78MA"),
    pubkey!("4KVgqboBYPMCmXxWTw3TopTAoVDym9hVDEL8FjXu1Rcr"),
    pubkey!("KiQZJtxt3tb2Dm4GG3Rt6wEZtCG4k1TZUj4YJVNskrK"),
    pubkey!("CESW47a4scrro6Jrv7dvjNkmDJHNdHVKwHi6Rb9dqmeK"),
    pubkey!("4F7KP1gGeJGSFjQaTD54Zfd6o5UhLKYQUZGxapTszkGV"),
    pubkey!("6ti7He1Mq9SapnAGFXnurZ9xeMB6MaF7vgEnZoqmkPGb"),
    pubkey!("9zSRNNU4oDE3CmaQcjZwnfrhUzxUuBP3o1grryu1oMan"),
    pubkey!("BMLVKrJGEr91RdbLxxPFRVnUq8bwo3KpSECvEcjP6hoq"),
    pubkey!("ChEisF8AkfpYED5aPgC9Aangx6CXfjqGPzuyg7CyjayQ"),
    pubkey!("9yNnhJ8c1vGbu3DMf6eeeUi6TDJ2ddGgaRA88rL2R3rP"),
    pubkey!("Br3yzg2WSb81RaFWK9UsKtq8fD5viwooZG34mKqQWxdM"),
    pubkey!("3mcAAcVnWXwLth6EKVpjd2XtFkPmyDL6o29ahPxAvnkz"),
    pubkey!("2Byn5sVRox8vKC6ndY7zQZm4N1i9afEE7kubP1UfSqap"),
    pubkey!("AWiMcY6NZyNzsAKrJJ24ywF38FqMmZQeReXNXXCn7Gt9"),
    pubkey!("4Bobu53fz6oeeezNax8gpXCgapNDYARqJ1L9DyMh2wYv"),
    pubkey!("D8b2RSTVbTjQ8cwzh1Y5LPSvFFiQ8xRCpKmNB9ftj5zT"),
    pubkey!("3PEVT4PWfpLdetLnhhzr6K5h5q9PgDX4YLBwGbQgGhUG"),
    pubkey!("7wms3cjYBE761HHSwNbybER3uC63pXsoeZ911vCLp4mR"),
    pubkey!("DhphS53vjik85NmNoPaRgwwTaE6tFZvn2HypA9Deswu8"),
    pubkey!("Fkz3sRcPEwcDfFXRkyoV5asEXresfJHzadXm4gKuq5oQ"),
    pubkey!("DFzg5hDZ55Nuc7TVSd2CcdHogbwdN8KAcN7KL3J7DCCd"),
    pubkey!("AWQwjMeG9KVJuDMwX9pdG5f1PtzqDDGk2urxmAXKV3yg"),
    pubkey!("9ARcMWiwN5mJdALVmrjBT8A3XkSF6tqqrhiqHvUTXmmU"),
    pubkey!("9RaajEmUyg9CkMKzqw6iGeSENo5QYCSdqMXMPLTBLuuN"),
    pubkey!("3J5rKmCi7JXG6qmiobFJyAidVTnnNAMGj4jomfBxKGRM"),
    pubkey!("6cgYhBFWCc5sNHxkvSRhd5H9AdAHR41zKwuF37HmLry5"),
    pubkey!("4EpDJVX1XkwwLrhG9bQQ5vt5mUhexD3cW4Sx98VZh3yB"),
    pubkey!("4jSd6HMz32o187jvhiyaA1sAhejQJSjTr1MVEW8evisR"),
    pubkey!("5tAgDoSxJBW995stfnvphtquP8JHn11xD3V3FvtLpxNQ"),
    pubkey!("AKEg31GR9rDD36Px5iwJpseve9FD34pGiSUgNqnri7Tw"),
    pubkey!("6YAVn7cEwiKBPiCXMFVY9cv5oWRj56WuPhNFjJyXWFad"),
    pubkey!("CFyznshAA978t6HCm4xprQpnN62c2qFSQrsCWN8q5UDB"),
    pubkey!("EFSqtDRH4yg2EhcT2zxNAubGBCPg5hRSaPSfC4o4ETkk"),
    pubkey!("HKHvcCZKJzWPycqQdgCCT5oxt7GWdbPHrg9HSdxpdsEL"),
    pubkey!("4V17N9er3oFmgxc1Ruh4FB1vHP5qSq2Mc1kSMPpHjBiE"),
];

pub fn admin_super_withdraw<'info>(
    mut ctx: Context<'_, '_, 'info, 'info, AdminSuperWithdraw<'info>>,
) -> MarginfiResult {
    let AdminSuperWithdraw {
        destination_token_account,
        liquidity_vault: bank_liquidity_vault,
        token_program,
        bank_liquidity_vault_authority,
        bank: bank_loader,
        ..
    } = ctx.accounts;

    let maybe_bank_mint = utils::maybe_take_bank_mint(
        &mut ctx.remaining_accounts,
        &*bank_loader.load()?,
        token_program.key,
    )?;

    let bank_key = bank_loader.key();
    check!(
        ALLOWED_ARENA_BANK_KEYS.contains(&bank_key),
        MarginfiError::Unauthorized
    );

    let mut bank = bank_loader.load_mut()?;
    let amount_to_withdraw: u64 = bank_liquidity_vault.amount;
    let liquidity_vault_authority_bump = bank.liquidity_vault_authority_bump;
    bank.config.operational_state = BankOperationalState::Paused;
    bank.config.deposit_limit = 0;
    bank.config.borrow_limit = 0;

    bank.withdraw_spl_transfer(
        // NOTE: For T22 assets with transfer fees, should still withdraw whole post-fee balance.
        amount_to_withdraw,
        bank_liquidity_vault.to_account_info(),
        destination_token_account.to_account_info(),
        bank_liquidity_vault_authority.to_account_info(),
        maybe_bank_mint.as_ref(),
        token_program.to_account_info(),
        bank_signer!(
            BankVaultType::Liquidity,
            bank_loader.key(),
            liquidity_vault_authority_bump
        ),
        ctx.remaining_accounts,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct AdminSuperWithdraw<'info> {
    #[account(
        has_one = admin
    )]
    pub group: AccountLoader<'info, MarginfiGroup>,

    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = group,
        has_one = liquidity_vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Seed constraint check
    #[account(
        seeds = [
            LIQUIDITY_VAULT_AUTHORITY_SEED.as_bytes(),
            bank.key().as_ref(),
        ],
        bump = bank.load()?.liquidity_vault_authority_bump,
    )]
    pub bank_liquidity_vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub liquidity_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}
