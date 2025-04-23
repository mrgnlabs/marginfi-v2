use crate::ui_to_native;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{
        get_associated_token_address_with_program_id,
        spl_associated_token_account::instruction::create_associated_token_account,
    },
    token::{spl_token, Mint, TokenAccount},
    token_2022::{
        self,
        spl_token_2022::{
            self,
            extension::{
                interest_bearing_mint::InterestBearingConfig,
                mint_close_authority::MintCloseAuthority, permanent_delegate::PermanentDelegate,
                transfer_fee::TransferFee, transfer_hook::TransferHook, BaseState,
                BaseStateWithExtensions, ExtensionType, StateWithExtensionsOwned,
            },
        },
    },
    token_interface::spl_pod::bytemuck::pod_get_packed_len,
};
use solana_cli_output::CliAccount;
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    account::{AccountSharedData, ReadableAccount, WritableAccount},
    commitment_config::CommitmentLevel,
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    program_pack::{Pack, Sealed},
    signature::Keypair,
    signer::Signer,
    system_instruction::{self, create_account},
    transaction::Transaction,
};
use spl_transfer_hook_interface::{
    get_extra_account_metas_address, instruction::initialize_extra_account_meta_list,
};
use std::{cell::RefCell, fs::File, io::Read, path::PathBuf, rc::Rc, str::FromStr};
use transfer_hook::TEST_HOOK_ID;

#[derive(Clone)]
pub struct MintFixture {
    pub ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
    pub mint: spl_token_2022::state::Mint,
    pub token_program: Pubkey,
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
            let ctx = ctx.borrow_mut();

            let rent = ctx.banks_client.get_rent().await.unwrap();

            let init_account_ix = create_account(
                &ctx.payer.pubkey(),
                &keypair.pubkey(),
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            );
            let init_mint_ix = spl_token::instruction::initialize_mint(
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

            ctx.banks_client
                .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
                .await
                .unwrap();

            let mint_account = ctx
                .banks_client
                .get_account(keypair.pubkey())
                .await
                .unwrap()
                .unwrap();

            spl_token_2022::state::Mint::unpack(mint_account.data.as_slice()).unwrap()
        };

        MintFixture {
            ctx: ctx_ref,
            key: keypair.pubkey(),
            mint,
            token_program: spl_token::id(),
        }
    }

    pub async fn new_token_22(
        ctx: Rc<RefCell<ProgramTestContext>>,
        mint_keypair: Option<Keypair>,
        mint_decimals: Option<u8>,
        extensions: &[SupportedExtension],
    ) -> MintFixture {
        let ctx_ref = Rc::clone(&ctx);
        let keypair = mint_keypair.unwrap_or_else(Keypair::new);
        let program = token_2022::ID;
        let mint = {
            let ctx = ctx.borrow_mut();

            let rent = ctx.banks_client.get_rent().await.unwrap();

            let extension_types = SupportedExtension::types(extensions.iter());
            let len = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(
                &extension_types,
            )
            .unwrap();
            let init_account_ix = create_account(
                &ctx.payer.pubkey(),
                &keypair.pubkey(),
                rent.minimum_balance(len),
                len as u64,
                &program,
            );
            let init_mint_ix = spl_token_2022::instruction::initialize_mint(
                &program,
                &keypair.pubkey(),
                &ctx.payer.pubkey(),
                None,
                mint_decimals.unwrap_or(6),
            )
            .unwrap();

            let mut ixs = vec![init_account_ix];
            ixs.extend(
                extensions
                    .iter()
                    .map(|e| e.instruction(&keypair.pubkey(), &ctx.payer.pubkey())),
            );
            ixs.push(init_mint_ix);
            let extra_metas_address = get_extra_account_metas_address(
                &keypair.pubkey(),
                &super::transfer_hook::TEST_HOOK_ID,
            );
            if extensions.contains(&SupportedExtension::TransferHook) {
                ixs.push(system_instruction::transfer(
                    &ctx.payer.pubkey(),
                    &extra_metas_address,
                    10 * LAMPORTS_PER_SOL,
                ));
                ixs.push(initialize_extra_account_meta_list(
                    &super::transfer_hook::TEST_HOOK_ID,
                    &extra_metas_address,
                    &keypair.pubkey(),
                    &ctx.payer.pubkey(),
                    &[],
                ))
            }

            let tx = Transaction::new_signed_with_payer(
                &ixs,
                Some(&ctx.payer.pubkey()),
                &[&ctx.payer, &keypair],
                ctx.last_blockhash,
            );

            ctx.banks_client
                .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
                .await
                .unwrap();

            if extensions.contains(&SupportedExtension::TransferHook) {
                ctx.banks_client
                    .get_account(extra_metas_address)
                    .await
                    .unwrap()
                    .unwrap();
            }

            let mint_account = ctx
                .banks_client
                .get_account(keypair.pubkey())
                .await
                .unwrap()
                .unwrap();

            StateWithExtensionsOwned::<spl_token_2022::state::Mint>::unpack(mint_account.data)
                .unwrap()
                .base
        };

        MintFixture {
            ctx: ctx_ref,
            key: keypair.pubkey(),
            mint,
            token_program: token_2022::ID,
        }
    }

    pub fn new_from_file(
        ctx: &Rc<RefCell<ProgramTestContext>>,
        relative_path: &str,
    ) -> MintFixture {
        let ctx_ref = Rc::clone(ctx);

        let (address, account_info) = {
            let mut ctx = ctx.borrow_mut();

            // load cargo workspace path from env
            let mut path = PathBuf::from_str(env!("CARGO_MANIFEST_DIR")).unwrap();
            path.push(relative_path);
            let mut file = File::open(&path).unwrap();
            let mut account_info_raw = String::new();
            file.read_to_string(&mut account_info_raw).unwrap();

            let account: CliAccount = serde_json::from_str(&account_info_raw).unwrap();
            let address = Pubkey::from_str(&account.keyed_account.pubkey).unwrap();
            let mut account_info: AccountSharedData =
                account.keyed_account.account.decode().unwrap();

            let mut mint =
                spl_token::state::Mint::unpack(&account_info.data()[..Mint::LEN]).unwrap();
            let payer = ctx.payer.pubkey();
            mint.mint_authority.replace(payer);

            let mint_bytes = &mut [0; Mint::LEN];
            spl_token::state::Mint::pack(mint, mint_bytes).unwrap();

            account_info.data_as_mut_slice()[..Mint::LEN].copy_from_slice(mint_bytes);

            ctx.set_account(&address, &account_info);

            (address, account_info)
        };

        let mint = spl_token_2022::state::Mint::unpack(&account_info.data()[..Mint::LEN]).unwrap();

        MintFixture {
            ctx: ctx_ref,
            key: address,
            mint,
            token_program: account_info.owner().to_owned(),
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
        self.mint =
            StateWithExtensionsOwned::<spl_token_2022::state::Mint>::unpack(mint_account.data)
                .unwrap()
                .base;
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
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await
            .unwrap();

        self.reload().await
    }

    pub fn make_mint_to_ix(&self, dest: &Pubkey, amount: u64) -> Instruction {
        let ctx = self.ctx.borrow();
        spl_token_2022::instruction::mint_to(
            &self.token_program,
            &self.key,
            dest,
            &ctx.payer.pubkey(),
            &[&ctx.payer.pubkey()],
            amount,
        )
        .unwrap()
    }

    pub async fn create_empty_token_account(&self) -> TokenAccountFixture {
        self.create_token_account_and_mint_to(0.0).await
    }

    pub async fn create_token_account_and_mint_to<'a, T: Into<f64>>(
        &self,
        ui_amount: T,
    ) -> TokenAccountFixture {
        let payer = self.ctx.borrow().payer.pubkey();
        let token_account_f = TokenAccountFixture::new_with_token_program(
            self.ctx.clone(),
            &self.key,
            &payer,
            &self.token_program,
        )
        .await;

        let mint_to_ix = self.make_mint_to_ix(
            &token_account_f.key,
            ui_to_native!(ui_amount.into(), self.mint.decimals),
        );

        let ctx = self.ctx.borrow_mut();

        let tx = Transaction::new_signed_with_payer(
            &[mint_to_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await
            .unwrap();

        token_account_f
    }

    pub async fn load_state(&self) -> StateWithExtensionsOwned<spl_token_2022::state::Mint> {
        let mint_account = self
            .ctx
            .borrow_mut()
            .banks_client
            .get_account(self.key)
            .await
            .unwrap()
            .unwrap();

        StateWithExtensionsOwned::unpack(mint_account.data).unwrap()
    }
}

pub struct TokenAccountFixture {
    ctx: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
    pub token: spl_token_2022::state::Account,
    pub token_program: Pubkey,
}

impl TokenAccountFixture {
    pub async fn create_ixs(
        ctx: &Rc<RefCell<ProgramTestContext>>,
        rent: Rent,
        mint_pk: &Pubkey,
        payer_pk: &Pubkey,
        owner_pk: &Pubkey,
        keypair: &Keypair,
        token_program: &Pubkey,
    ) -> Vec<Instruction> {
        let mut ixs = vec![];

        // Get extensions if t22 (should return no exts if spl_token)
        // 1) Fetch mint
        let mint_account = ctx
            .borrow_mut()
            .banks_client
            .get_account(*mint_pk)
            .await
            .unwrap()
            .unwrap();
        let mint_exts =
            spl_token_2022::extension::StateWithExtensions::<spl_token_2022::state::Mint>::unpack(
                &mint_account.data,
            )
            .unwrap();

        let mint_extensions = mint_exts.get_extension_types().unwrap();
        let required_extensions =
            ExtensionType::get_required_init_account_extensions(&mint_extensions);

        let space = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Account>(
            &required_extensions,
        )
        .unwrap();

        // Init account
        ixs.push(create_account(
            payer_pk,
            &keypair.pubkey(),
            rent.minimum_balance(space),
            space as u64,
            token_program,
        ));

        // 2) Add instructions
        if required_extensions.contains(&ExtensionType::ImmutableOwner) {
            ixs.push(
                spl_token_2022::instruction::initialize_immutable_owner(
                    token_program,
                    &keypair.pubkey(),
                )
                .unwrap(),
            )
        }

        // Token Account init
        ixs.push(
            spl_token_2022::instruction::initialize_account(
                token_program,
                &keypair.pubkey(),
                mint_pk,
                owner_pk,
            )
            .unwrap(),
        );

        ixs
    }

    pub async fn new_account(&self) -> Pubkey {
        let keypair = Keypair::new();
        let ctx = self.ctx.borrow_mut();

        let ixs = Self::create_ixs(
            &self.ctx,
            ctx.banks_client.get_rent().await.unwrap(),
            &self.token.mint,
            &ctx.payer.pubkey(),
            &ctx.payer.pubkey(),
            &keypair,
            &self.token_program,
        )
        .await;

        let tx = Transaction::new_signed_with_payer(
            &ixs,
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &keypair],
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
            .await
            .unwrap();

        keypair.pubkey()
    }

    #[allow(unused)]
    pub async fn new_with_keypair(
        ctx: Rc<RefCell<ProgramTestContext>>,
        mint_pk: &Pubkey,
        owner_pk: &Pubkey,
        keypair: &Keypair,
        token_program: &Pubkey,
    ) -> Self {
        let ctx_ref = ctx.clone();

        {
            let payer = ctx.borrow().payer.pubkey();
            let rent = ctx.borrow_mut().banks_client.get_rent().await.unwrap();
            let instructions = Self::create_ixs(
                &ctx,
                rent,
                mint_pk,
                &payer,
                owner_pk,
                keypair,
                token_program,
            )
            .await;

            // Token extensions

            let tx = Transaction::new_signed_with_payer(
                &instructions,
                Some(&ctx.borrow().payer.pubkey()),
                &[&ctx.borrow().payer, keypair],
                ctx.borrow().last_blockhash,
            );

            ctx.borrow_mut()
                .banks_client
                .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
                .await
                .unwrap();
        }

        let ctx = ctx.borrow_mut();
        let account = ctx
            .banks_client
            .get_account(keypair.pubkey())
            .await
            .unwrap()
            .unwrap();

        Self {
            ctx: ctx_ref.clone(),
            key: keypair.pubkey(),
            token: StateWithExtensionsOwned::<spl_token_2022::state::Account>::unpack(account.data)
                .unwrap()
                .base,
            token_program: *token_program,
        }
    }

    pub async fn new_from_ata(
        ctx: Rc<RefCell<ProgramTestContext>>,
        mint_pk: &Pubkey,
        owner_pk: &Pubkey,
        token_program: &Pubkey,
    ) -> Self {
        let ctx_ref = ctx.clone();
        let ata_address =
            get_associated_token_address_with_program_id(owner_pk, mint_pk, token_program);

        {
            let create_ata_ix = create_associated_token_account(
                &ctx.borrow().payer.pubkey(),
                owner_pk,
                mint_pk,
                token_program,
            );

            let tx = Transaction::new_signed_with_payer(
                &[create_ata_ix],
                Some(&ctx.borrow().payer.pubkey()),
                &[&ctx.borrow().payer],
                ctx.borrow().last_blockhash,
            );

            ctx.borrow_mut()
                .banks_client
                .process_transaction_with_preflight_and_commitment(tx, CommitmentLevel::Confirmed)
                .await
                .unwrap();
        }

        // Now retrieve the account info for the newly created ATA
        let ctx = ctx.borrow_mut();
        let account = ctx
            .banks_client
            .get_account(ata_address)
            .await
            .unwrap()
            .unwrap();

        Self {
            ctx: ctx_ref.clone(),
            key: ata_address, // Use the ATA address as the key
            token: StateWithExtensionsOwned::<spl_token_2022::state::Account>::unpack(account.data)
                .unwrap()
                .base,
            token_program: *token_program,
        }
    }

    pub async fn new(
        ctx: Rc<RefCell<ProgramTestContext>>,
        mint_fixture: &MintFixture,
        owner_pk: &Pubkey,
    ) -> TokenAccountFixture {
        let keypair = Keypair::new();
        let mint_pk = mint_fixture.key;
        TokenAccountFixture::new_with_keypair(
            ctx,
            &mint_pk,
            owner_pk,
            &keypair,
            &mint_fixture.token_program,
        )
        .await
    }

    pub async fn new_with_token_program(
        ctx: Rc<RefCell<ProgramTestContext>>,
        mint_pk: &Pubkey,
        owner_pk: &Pubkey,
        token_program: &Pubkey,
    ) -> TokenAccountFixture {
        let keypair = Keypair::new();
        TokenAccountFixture::new_with_keypair(ctx, mint_pk, owner_pk, &keypair, token_program).await
    }

    pub async fn fetch(
        ctx: Rc<RefCell<ProgramTestContext>>,
        address: Pubkey,
    ) -> TokenAccountFixture {
        let token: spl_token_2022::state::Account =
            get_and_deserialize_t22(ctx.clone(), address).await;
        let token_program = token.owner;

        Self {
            ctx: ctx.clone(),
            key: address,
            token,
            token_program,
        }
    }

    pub async fn balance(&self) -> u64 {
        let token_account: spl_token_2022::state::Account =
            get_and_deserialize_t22(self.ctx.clone(), self.key).await;

        token_account.amount
    }
}

pub async fn get_and_deserialize<T: AccountDeserialize>(
    ctx: Rc<RefCell<ProgramTestContext>>,
    pubkey: Pubkey,
) -> T {
    let ctx = ctx.borrow_mut();
    let account = ctx.banks_client.get_account(pubkey).await.unwrap().unwrap();

    T::try_deserialize(&mut account.data.as_slice()).unwrap()
}
pub async fn get_and_deserialize_t22<T: BaseState + Pack + Sealed>(
    ctx: Rc<RefCell<ProgramTestContext>>,
    pubkey: Pubkey,
) -> T {
    let ctx = ctx.borrow_mut();
    let account = ctx.banks_client.get_account(pubkey).await.unwrap().unwrap();

    StateWithExtensionsOwned::<T>::unpack(account.data)
        .unwrap()
        .base
}

pub async fn balance_of(ctx: Rc<RefCell<ProgramTestContext>>, pubkey: Pubkey) -> u64 {
    let token_account: TokenAccount = get_and_deserialize(ctx, pubkey).await;

    token_account.amount
}

#[derive(Debug, Clone, PartialEq)]
pub enum SupportedExtension {
    MintCloseAuthority,
    InterestBearing,
    PermanentDelegate,
    TransferHook,
    TransferFee,
}

impl SupportedExtension {
    pub fn instruction(&self, mint: &Pubkey, key: &Pubkey) -> Instruction {
        match self {
            SupportedExtension::MintCloseAuthority => {
                spl_token_2022::instruction::initialize_mint_close_authority(
                    &token_2022::ID,
                    mint,
                    Some(key),
                )
                .unwrap()
            }
            SupportedExtension::InterestBearing => {
                spl_token_2022::extension::interest_bearing_mint::instruction::initialize(
                    &token_2022::ID,
                    mint,
                    Some(*key),
                    1,
                )
                .unwrap()
            }
            SupportedExtension::PermanentDelegate => {
                spl_token_2022::instruction::initialize_permanent_delegate(
                    &token_2022::ID,
                    mint,
                    key,
                )
                .unwrap()
            }
            Self::TransferHook => {
                spl_token_2022::extension::transfer_hook::instruction::initialize(
                    &token_2022::ID,
                    mint,
                    Some(*key),
                    Some(TEST_HOOK_ID),
                )
                .unwrap()
            }
            Self::TransferFee => {
                spl_token_2022::extension::transfer_fee::instruction::initialize_transfer_fee_config(
                &token_2022::ID,
                mint,
                None,
                None,
                500,
                u64::MAX,
            )
            .unwrap()
            }
        }
    }

    pub fn space<'a>(exts: impl Iterator<Item = &'a SupportedExtension>) -> usize {
        exts.map(|e| match e {
            SupportedExtension::MintCloseAuthority => pod_get_packed_len::<MintCloseAuthority>(),
            SupportedExtension::InterestBearing => pod_get_packed_len::<InterestBearingConfig>(),
            SupportedExtension::PermanentDelegate => pod_get_packed_len::<PermanentDelegate>(),
            SupportedExtension::TransferHook => pod_get_packed_len::<TransferHook>(),
            SupportedExtension::TransferFee => pod_get_packed_len::<TransferFee>(),
        })
        .sum()
    }

    fn types<'a>(exts: impl Iterator<Item = &'a SupportedExtension>) -> Vec<ExtensionType> {
        exts.map(|e| match e {
            SupportedExtension::MintCloseAuthority => ExtensionType::MintCloseAuthority,
            SupportedExtension::InterestBearing => ExtensionType::InterestBearingConfig,
            SupportedExtension::PermanentDelegate => ExtensionType::PermanentDelegate,
            SupportedExtension::TransferHook => ExtensionType::TransferHook,
            SupportedExtension::TransferFee => ExtensionType::TransferFeeConfig,
        })
        .collect()
    }
}
