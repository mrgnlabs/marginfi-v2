use std::collections::HashMap;

use anchor_lang::{AnchorDeserialize, Discriminator};
use marginfi::{
    instruction::{
        LendingAccountBorrow, LendingAccountCloseBalance, LendingAccountDeposit,
        LendingAccountEndFlashloan, LendingAccountLiquidate, LendingAccountRepay,
        LendingAccountSettleEmissions, LendingAccountStartFlashloan, LendingAccountWithdraw,
        LendingAccountWithdrawEmissions, LendingPoolAccrueBankInterest, LendingPoolAddBank,
        LendingPoolAddBankWithSeed, LendingPoolConfigureBank, MarginfiAccountInitialize,
        SetNewAccountAuthority,
    },
    state::marginfi_group::{BankConfig, BankConfigCompact},
};
use solana_sdk::{
    hash::Hash,
    instruction::CompiledInstruction,
    message::SimpleAddressLoader,
    pubkey,
    pubkey::Pubkey,
    signature::Signature,
    transaction::{MessageHash, SanitizedTransaction},
};
use solana_transaction_status::{
    InnerInstruction, InnerInstructions, VersionedTransactionWithStatusMeta,
};
use tracing::{error, warn};

const SPL_TRANSFER_DISCRIMINATOR: u8 = 3;
pub const MARGINFI_GROUP_ADDRESS: Pubkey = pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8");
const COMPACT_BANK_CONFIG_ARG_UPGRADE_SLOT: u64 = 232836972;

#[derive(Debug)]
pub enum MarginfiEvent {
    // User actions
    CreateAccount(CreateAccountEvent),
    AccountAuthorityTransfer(AccountAuthorityTransferEvent),
    Deposit(DepositEvent),
    Borrow(BorrowEvent),
    Repay(RepayEvent),
    Withdraw(WithdrawEvent),
    WithdrawEmissions(WithdrawEmissionsEvent),
    Liquidate(LiquidateEvent),

    // Admin actions
    AddBank(AddBankEvent),
}

#[derive(Debug)]
pub struct MarginfiEventWithMeta {
    pub timestamp: i64,
    pub tx_sig: Signature,
    pub event: MarginfiEvent,
    pub in_flashloan: bool,
    pub call_stack: Vec<Pubkey>,
}

#[derive(Debug)]
pub struct CreateAccountEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
}

#[derive(Debug)]
pub struct AccountAuthorityTransferEvent {
    pub account: Pubkey,
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
}

#[derive(Debug)]
pub struct DepositEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
    pub bank: Pubkey,
    pub amount: u64,
}

#[derive(Debug)]
pub struct BorrowEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
    pub bank: Pubkey,
    pub amount: u64,
}

#[derive(Debug)]
pub struct RepayEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
    pub bank: Pubkey,
    pub amount: u64,
    pub all: bool,
}

#[derive(Debug)]
pub struct WithdrawEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
    pub bank: Pubkey,
    pub amount: u64,
    pub all: bool,
}

#[derive(Debug)]
pub struct WithdrawEmissionsEvent {
    pub account: Pubkey,
    pub authority: Pubkey,
    pub bank: Pubkey,
    pub emissions_mint: Pubkey,
    pub amount: u64,
}

#[derive(Debug)]
pub struct LiquidateEvent {
    pub asset_amount: u64,
    pub asset_bank: Pubkey,
    pub liability_bank: Pubkey,
    pub liquidator_account: Pubkey,
    pub liquidator_authority: Pubkey,
    pub liquidatee_account: Pubkey,
}

#[derive(Debug)]
pub struct AddBankEvent {
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub config: BankConfig,
}

pub struct MarginfiEventParser {
    program_id: Pubkey,
    marginfi_group: Pubkey,
}

impl MarginfiEventParser {
    pub fn new(program_id: Pubkey, marginfi_group: Pubkey) -> Self {
        Self {
            program_id,
            marginfi_group,
        }
    }

    pub fn extract_events(
        &self,
        timestamp: i64,
        slot: u64,
        tx_with_meta: VersionedTransactionWithStatusMeta,
    ) -> Vec<MarginfiEventWithMeta> {
        let tx_sig = tx_with_meta.transaction.signatures[0];

        let mut events: Vec<MarginfiEventWithMeta> = vec![];

        let mut in_flashloan = false;

        let sanitized_tx = SanitizedTransaction::try_create(
            tx_with_meta.transaction,
            MessageHash::Precomputed(Hash::default()),
            None,
            SimpleAddressLoader::Enabled(tx_with_meta.meta.loaded_addresses),
            true,
        )
        .unwrap();

        let mut inner_instructions: HashMap<u8, Vec<InnerInstruction>> = HashMap::new();
        for InnerInstructions {
            instructions,
            index,
        } in tx_with_meta
            .meta
            .inner_instructions
            .unwrap_or_default()
            .into_iter()
        {
            inner_instructions.insert(index, instructions);
        }

        for (outer_ix_index, instruction) in
            sanitized_tx.message().instructions().iter().enumerate()
        {
            let account_keys = sanitized_tx
                .message()
                .account_keys()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            let top_level_program_id = instruction.program_id(&account_keys);

            let mut call_stack = vec![];

            let inner_instructions = inner_instructions
                .remove(&(outer_ix_index as u8))
                .unwrap_or_default();

            if top_level_program_id.eq(&self.program_id) {
                // println!("Instruction {}: {:?}", i, top_level_program_id);
                let event = self.parse_event(
                    slot,
                    &tx_sig,
                    &instruction,
                    &inner_instructions,
                    &account_keys,
                    &mut in_flashloan,
                );
                if let Some(event) = event {
                    let call_stack = call_stack.iter().cloned().cloned().collect();
                    let event_with_meta = MarginfiEventWithMeta {
                        timestamp,
                        tx_sig,
                        event,
                        in_flashloan,
                        call_stack,
                    };
                    // info!("Event: {:?}", event_with_meta);
                    events.push(event_with_meta);
                }
            }

            if inner_instructions.is_empty() {
                continue;
            }

            call_stack.push(top_level_program_id);

            for (inner_ix_index, inner_instruction) in inner_instructions.iter().enumerate() {
                let cpi_program_id = inner_instruction.instruction.program_id(&account_keys);

                if cpi_program_id.eq(&self.program_id) {
                    let remaining_instructions = if inner_instructions.len() > inner_ix_index + 1 {
                        &inner_instructions[(inner_ix_index + 1)..]
                    } else {
                        &[]
                    };

                    let event = self.parse_event(
                        slot,
                        &tx_sig,
                        &inner_instruction.instruction,
                        remaining_instructions,
                        &account_keys,
                        &mut in_flashloan,
                    );
                    if let Some(event) = event {
                        let call_stack = call_stack.iter().cloned().cloned().collect();
                        let event_with_meta = MarginfiEventWithMeta {
                            timestamp,
                            tx_sig,
                            event,
                            in_flashloan,
                            call_stack,
                        };
                        // info!("Inner event: {:?}", event_with_meta);
                        events.push(event_with_meta);
                    }
                }

                if let Some(stack_height) = inner_instruction.stack_height {
                    if stack_height - 1 > call_stack.len() as u32 {
                        call_stack.push(cpi_program_id);
                    } else {
                        call_stack.truncate(stack_height as usize);
                    }
                }
            }
        }

        events
    }

    pub fn parse_event(
        &self,
        slot: u64,
        tx_signature: &Signature,
        instruction: &CompiledInstruction,
        remaining_instructions: &[InnerInstruction],
        account_keys: &[Pubkey],
        in_flashloan: &mut bool,
    ) -> Option<MarginfiEvent> {
        if instruction.data.len() < 8 {
            error!("Instruction data too short");
            return None;
        }

        let ix_accounts = instruction
            .accounts
            .iter()
            .map(|ix| account_keys[*ix as usize])
            .collect::<Vec<_>>();

        let discriminator: [u8; 8] = instruction.data[..8].try_into().ok()?;
        let mut instruction_data = &instruction.data[8..];
        match discriminator {
            MarginfiAccountInitialize::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let authority = *ix_accounts.get(2).unwrap();

                Some(MarginfiEvent::CreateAccount(CreateAccountEvent {
                    account: marginfi_account,
                    authority,
                }))
            }
            SetNewAccountAuthority::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(1).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let marginfi_account = *ix_accounts.get(0).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let new_authority = *ix_accounts.get(3).unwrap();

                Some(MarginfiEvent::AccountAuthorityTransfer(
                    AccountAuthorityTransferEvent {
                        account: marginfi_account,
                        old_authority: signer,
                        new_authority,
                    },
                ))
            }
            LendingAccountDeposit::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                if remaining_instructions.is_empty() {
                    warn!(
                        "Expected non-empty remaining instructions after deposit in {:?}",
                        tx_signature
                    );
                    return None;
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();

                Some(MarginfiEvent::Deposit(DepositEvent {
                    account: marginfi_account,
                    authority: signer,
                    bank,
                    amount: spl_transfer_amount,
                }))
            }
            LendingAccountBorrow::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                if remaining_instructions.is_empty() {
                    warn!(
                        "Expected non-empty remaining instructions after borrow in {:?}",
                        tx_signature
                    );
                    return None;
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();

                Some(MarginfiEvent::Borrow(BorrowEvent {
                    account: marginfi_account,
                    authority: signer,
                    bank,
                    amount: spl_transfer_amount,
                }))
            }
            LendingAccountRepay::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let instruction = LendingAccountRepay::deserialize(&mut instruction_data).ok()?;

                if remaining_instructions.is_empty() {
                    warn!(
                        "Expected non-empty remaining instructions after repay in {:?}",
                        tx_signature
                    );
                    return None;
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();

                Some(MarginfiEvent::Repay(RepayEvent {
                    account: marginfi_account,
                    authority: signer,
                    bank,
                    amount: spl_transfer_amount,
                    all: instruction.repay_all.unwrap_or(false),
                }))
            }
            LendingAccountWithdraw::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let instruction =
                    LendingAccountWithdraw::deserialize(&mut instruction_data).ok()?;

                if remaining_instructions.is_empty() {
                    warn!(
                        "Expected non-empty remaining instructions after withdraw in {:?}",
                        tx_signature
                    );
                    return None;
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();

                Some(MarginfiEvent::Withdraw(WithdrawEvent {
                    account: marginfi_account,
                    authority: signer,
                    bank,
                    amount: spl_transfer_amount,
                    all: instruction.withdraw_all.unwrap_or(false),
                }))
            }
            LendingAccountLiquidate::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let instruction =
                    LendingAccountLiquidate::deserialize(&mut instruction_data).ok()?;

                let asset_bank = *ix_accounts.get(1).unwrap();
                let liability_bank = *ix_accounts.get(2).unwrap();
                let liquidator_account = *ix_accounts.get(3).unwrap();
                let liquidator_authority = *ix_accounts.get(4).unwrap();
                let liquidatee_account = *ix_accounts.get(5).unwrap();

                Some(MarginfiEvent::Liquidate(LiquidateEvent {
                    asset_amount: instruction.asset_amount,
                    asset_bank,
                    liability_bank,
                    liquidator_account,
                    liquidator_authority,
                    liquidatee_account,
                }))
            }
            LendingAccountWithdrawEmissions::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                if remaining_instructions.is_empty() {
                    return None;
                }

                let transfer_ix = &remaining_instructions.get(0).unwrap().instruction;
                let spl_transfer_amount = get_spl_transfer_amount(transfer_ix, account_keys)?;

                let marginfi_account = *ix_accounts.get(1).unwrap();
                let signer = *ix_accounts.get(2).unwrap();
                let bank = *ix_accounts.get(3).unwrap();
                let emissions_mint = *ix_accounts.get(4).unwrap();

                Some(MarginfiEvent::WithdrawEmissions(WithdrawEmissionsEvent {
                    account: marginfi_account,
                    authority: signer,
                    bank,
                    emissions_mint,
                    amount: spl_transfer_amount,
                }))
            }
            LendingPoolAddBank::DISCRIMINATOR => {
                let marginfi_group = *ix_accounts.get(0).unwrap();
                if !marginfi_group.eq(&self.marginfi_group) {
                    return None;
                }

                let bank_config = if slot < COMPACT_BANK_CONFIG_ARG_UPGRADE_SLOT {
                    BankConfig::deserialize(&mut &instruction_data[..531]).unwrap()
                } else {
                    BankConfigCompact::deserialize(&mut &instruction_data[..531])
                        .unwrap()
                        .into()
                };

                let bank_mint = *ix_accounts.get(3).unwrap();
                let bank = *ix_accounts.get(4).unwrap();

                Some(MarginfiEvent::AddBank(AddBankEvent {
                    bank,
                    mint: bank_mint,
                    config: bank_config,
                }))
            }
            LendingAccountStartFlashloan::DISCRIMINATOR => {
                *in_flashloan = true;

                None
            }
            LendingAccountEndFlashloan::DISCRIMINATOR => {
                *in_flashloan = false;

                None
            }
            LendingAccountCloseBalance::DISCRIMINATOR
            | LendingPoolAccrueBankInterest::DISCRIMINATOR
            | LendingAccountSettleEmissions::DISCRIMINATOR
            | LendingPoolConfigureBank::DISCRIMINATOR
            | LendingPoolAddBankWithSeed::DISCRIMINATOR => None,
            _ => {
                warn!(
                    "Unknown instruction discriminator {:?} in {:?}",
                    discriminator, tx_signature
                );
                None
            }
        }
    }
}

fn get_spl_transfer_amount(
    instruction: &CompiledInstruction,
    account_keys: &[Pubkey],
) -> Option<u64> {
    let transfer_ix_pid = instruction.program_id(account_keys);
    if !transfer_ix_pid.eq(&spl_token::id()) || instruction.data[0] != SPL_TRANSFER_DISCRIMINATOR {
        warn!(
            "Expected following instruction to be {:?}/{} in deposit, got {:?}/{:?} instead",
            spl_token::id(),
            SPL_TRANSFER_DISCRIMINATOR,
            transfer_ix_pid,
            instruction.data[0]
        );
        return None;
    }

    let spl_transfer_amount: u64 = u64::from_le_bytes(instruction.data[1..9].try_into().unwrap());
    Some(spl_transfer_amount)
}
