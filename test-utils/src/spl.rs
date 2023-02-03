use crate::ui_to_native;
use anchor_lang::prelude::*;
use anchor_spl::token::{
    spl_token::{
        self,
        instruction::{initialize_mint, mint_to},
    },
    Mint, TokenAccount,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    instruction::Instruction, signature::Keypair, signer::Signer,
    system_instruction::create_account, transaction::Transaction,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub struct MintFixture {
    pub ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
    pub mint: Mint,
}

impl MintFixture {
    pub async fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        mint_keypair: Option<Keypair>,
        mint_decimals: Option<u8>,
    ) -> MintFixture {
        let ctx_ref = Rc::clone(&ctx);
        let keypair = mint_keypair.unwrap_or_else(Keypair::new);
        let mint = {
            let mut ctx = ctx.borrow_mut();

            let rent = ctx.banks_client.get_rent().await.unwrap();

            let init_account_ix = create_account(
                &ctx.payer.pubkey(),
                &keypair.pubkey(),
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            );
            let init_mint_ix = initialize_mint(
                &spl_token::id(),
                &keypair.pubkey(),
                &ctx.payer.pubkey(),
                None,
                mint_decimals.unwrap_or(6),
            )
            .unwrap();

            let tx = Transaction::new_signed_with_payer(
                &[init_account_ix, init_mint_ix],
                Some(&ctx.payer.pubkey()),
                &[&ctx.payer, &keypair],
                ctx.last_blockhash,
            );

            ctx.banks_client.process_transaction(tx).await.unwrap();

            let mint_account = ctx
                .banks_client
                .get_account(keypair.pubkey())
                .await
                .unwrap()
                .unwrap();

            Mint::try_deserialize(&mut mint_account.data.as_slice()).unwrap()
        };

        MintFixture {
            ctx: ctx_ref,
            key: keypair.pubkey(),
            mint,
        }
    }

    #[allow(unused)]
    pub async fn reload(&mut self) {
        let mint_account = self
            .ctx
            .borrow_mut()
            .banks_client
            .get_account(self.key)
            .await
            .unwrap()
            .unwrap();
        self.mint = Mint::try_deserialize(&mut mint_account.data.as_slice()).unwrap();
    }

    pub async fn mint_to<T: Into<f64>>(&mut self, dest: &Pubkey, ui_amount: T) {
        let tx = {
            let ctx = self.ctx.borrow();
            let mint_to_ix =
                self.make_mint_to_ix(dest, ui_to_native!(ui_amount.into(), self.mint.decimals));
            Transaction::new_signed_with_payer(
                &[mint_to_ix],
                Some(&ctx.payer.pubkey()),
                &[&ctx.payer],
                ctx.last_blockhash,
            )
        };

        self.ctx
            .borrow_mut()
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();

        self.reload().await
    }

    pub fn make_mint_to_ix(&self, dest: &Pubkey, amount: u64) -> Instruction {
        let ctx = self.ctx.borrow();
        mint_to(
            &spl_token::id(),
            &self.key,
            dest,
            &ctx.payer.pubkey(),
            &[&ctx.payer.pubkey()],
            amount,
        )
        .unwrap()
    }

    pub async fn create_token_account_and_mint_to<T: Into<f64>>(
        &self,
        ui_amount: T,
    ) -> TokenAccountFixture {
        let payer = self.ctx.borrow().payer.pubkey();
        let token_account_f = TokenAccountFixture::new(self.ctx.clone(), &self.key, &payer).await;

        let mint_to_ix = self.make_mint_to_ix(
            &token_account_f.key,
            ui_to_native!(ui_amount.into(), self.mint.decimals),
        );

        let mut ctx = self.ctx.borrow_mut();

        let tx = Transaction::new_signed_with_payer(
            &[mint_to_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await.unwrap();

        token_account_f
    }
}

pub struct TokenAccountFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
    pub token: TokenAccount,
}

impl TokenAccountFixture {
    pub async fn create_ixs(
        rent: Rent,
        mint_pk: &Pubkey,
        payer_pk: &Pubkey,
        owner_pk: &Pubkey,
        keypair: &Keypair,
    ) -> [Instruction; 2] {
        let init_account_ix = create_account(
            payer_pk,
            &keypair.pubkey(),
            rent.minimum_balance(TokenAccount::LEN),
            TokenAccount::LEN as u64,
            &spl_token::id(),
        );

        let init_token_ix = spl_token::instruction::initialize_account(
            &spl_token::id(),
            &keypair.pubkey(),
            mint_pk,
            owner_pk,
        )
        .unwrap();

        [init_account_ix, init_token_ix]
    }

    pub async fn new_account(&self) -> Pubkey {
        let keypair = Keypair::new();
        let mut ctx = self.ctx.borrow_mut();

        let ixs = Self::create_ixs(
            ctx.banks_client.get_rent().await.unwrap(),
            &self.token.mint,
            &ctx.payer.pubkey(),
            &ctx.payer.pubkey(),
            &keypair,
        )
        .await;

        let tx = Transaction::new_signed_with_payer(
            &ixs,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &keypair],
            ctx.last_blockhash,
        );

        ctx.banks_client.process_transaction(tx).await.unwrap();

        keypair.pubkey()
    }

    #[allow(unused)]
    pub async fn new_with_keypair(
        ctx: Rc<RefCell<ProgramTestContext>>,
        mint_pk: &Pubkey,
        owner_pk: &Pubkey,
        keypair: &Keypair,
    ) -> Self {
        let ctx_ref = ctx.clone();

        {
            let mut ctx = ctx.borrow_mut();

            let rent = ctx.banks_client.get_rent().await.unwrap();
            let instructions =
                Self::create_ixs(rent, mint_pk, &ctx.payer.pubkey(), owner_pk, keypair).await;

            let tx = Transaction::new_signed_with_payer(
                &instructions,
                Some(&ctx.payer.pubkey()),
                &[&ctx.payer, keypair],
                ctx.last_blockhash,
            );

            ctx.banks_client.process_transaction(tx).await.unwrap();
        }

        Self {
            ctx: ctx_ref.clone(),
            key: keypair.pubkey(),
            token: get_and_deserialize(ctx_ref.clone(), keypair.pubkey()).await,
        }
    }

    pub async fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        mint_pk: &Pubkey,
        owner_pk: &Pubkey,
    ) -> TokenAccountFixture {
        let keypair = Keypair::new();
        TokenAccountFixture::new_with_keypair(ctx, mint_pk, owner_pk, &keypair).await
    }

    pub async fn fetch(
        ctx: Rc<RefCell<ProgramTestContext>>,
        address: Pubkey,
    ) -> TokenAccountFixture {
        let token = get_and_deserialize(ctx.clone(), address).await;

        Self {
            ctx: ctx.clone(),
            key: address,
            token,
        }
    }

    pub async fn balance(&self) -> u64 {
        let token_account: TokenAccount = get_and_deserialize(self.ctx.clone(), self.key).await;

        token_account.amount
    }
}

pub async fn get_and_deserialize<T: AccountDeserialize>(
    ctx: Rc<RefCell<ProgramTestContext>>,
    pubkey: Pubkey,
) -> T {
    let mut ctx = ctx.borrow_mut();
    let account = ctx.banks_client.get_account(pubkey).await.unwrap().unwrap();
    T::try_deserialize(&mut account.data.as_slice()).unwrap()
}

pub async fn balance_of(ctx: Rc<RefCell<ProgramTestContext>>, pubkey: Pubkey) -> u64 {
    let token_account: TokenAccount = get_and_deserialize(ctx, pubkey).await;

    token_account.amount
}
