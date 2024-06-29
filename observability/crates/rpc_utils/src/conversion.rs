use std::str::FromStr;

use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use solana_sdk::{account::Account, instruction::CompiledInstruction, message::v0::LoadedAddresses, pubkey::Pubkey, transaction_context::TransactionReturnData};
use solana_transaction_status::{EncodedTransactionWithStatusMeta, InnerInstruction, InnerInstructions, TransactionStatusMeta, TransactionTokenBalance, UiInnerInstructions, UiInstruction, UiLoadedAddresses, UiTransactionReturnData, UiTransactionStatusMeta, UiTransactionTokenBalance, VersionedTransactionWithStatusMeta};


pub fn convert_encoded_ui_transaction(
  encoded_tx: EncodedTransactionWithStatusMeta,
) -> anyhow::Result<VersionedTransactionWithStatusMeta> {
  Ok(VersionedTransactionWithStatusMeta {
      transaction: encoded_tx.transaction.decode().unwrap(),
      meta: convert_meta(encoded_tx.meta.unwrap())?,
  })
}

pub fn convert_meta(ui_meta: UiTransactionStatusMeta) -> anyhow::Result<TransactionStatusMeta> {
  let inner_instructions: Option<Vec<UiInnerInstructions>> = ui_meta.inner_instructions.into();
  let log_messages: Option<Vec<String>> = ui_meta.log_messages.into();
  let pre_token_balances: Option<Vec<UiTransactionTokenBalance>> =
      ui_meta.pre_token_balances.into();
  let post_token_balances: Option<Vec<UiTransactionTokenBalance>> =
      ui_meta.post_token_balances.into();
  let rewards: Option<_> = ui_meta.rewards.into();
  let return_data: Option<UiTransactionReturnData> = ui_meta.return_data.into();
  let compute_units_consumed: Option<_> = ui_meta.compute_units_consumed.into();
  let loaded_addresses: Option<UiLoadedAddresses> = ui_meta.loaded_addresses.into();

  Ok(TransactionStatusMeta {
      status: match ui_meta.err {
          Some(err) => Err(err),
          None => Ok(()),
      },
      fee: ui_meta.fee,
      pre_balances: ui_meta.pre_balances,
      post_balances: ui_meta.post_balances,
      inner_instructions: inner_instructions
          .map(|ixs| {
              ixs.into_iter()
                  .map(|ix| convert_inner_instructions(ix))
                  .collect::<Result<Vec<_>>>()
          })
          .transpose()?,
      log_messages,
      pre_token_balances: pre_token_balances
          .map(|balances| {
              balances
                  .into_iter()
                  .map(|balance| convert_token_balance(balance))
                  .collect::<Result<Vec<_>>>()
          })
          .transpose()?,
      post_token_balances: post_token_balances
          .map(|balances| {
              balances
                  .into_iter()
                  .map(|balance| convert_token_balance(balance))
                  .collect::<Result<Vec<_>>>()
          })
          .transpose()?,
      rewards,
      loaded_addresses: convert_loaded_addresses(loaded_addresses.unwrap())?,
      return_data: return_data
          .map(|data| convert_return_data(data))
          .transpose()?,
      compute_units_consumed,
  })
}

fn convert_loaded_addresses(
  ui_loaded_addresses: UiLoadedAddresses,
) -> anyhow::Result<LoadedAddresses> {
  Ok(LoadedAddresses {
      writable: ui_loaded_addresses
          .writable
          .into_iter()
          .map(|address| Ok(Pubkey::from_str(&address)?))
          .collect::<Result<Vec<Pubkey>>>()?,
      readonly: ui_loaded_addresses
          .readonly
          .into_iter()
          .map(|address| Ok(Pubkey::from_str(&address)?))
          .collect::<Result<Vec<_>>>()?,
  })
}

fn convert_return_data(
  ui_return_data: UiTransactionReturnData,
) -> anyhow::Result<TransactionReturnData> {
  Ok(TransactionReturnData {
      program_id: Pubkey::from_str(&ui_return_data.program_id)?,
      data: STANDARD.decode(&ui_return_data.data.0)?,
  })
}

fn convert_token_balance(
  ui_balance: UiTransactionTokenBalance,
) -> anyhow::Result<TransactionTokenBalance> {
  let owner: Option<_> = ui_balance.owner.into();
  let program_id: Option<_> = ui_balance.program_id.into();

  Ok(TransactionTokenBalance {
      owner: owner.ok_or(anyhow!("Owner is missing"))?,
      program_id: program_id.ok_or(anyhow!("Program id is missing"))?,
      account_index: ui_balance.account_index,
      mint: ui_balance.mint,
      ui_token_amount: ui_balance.ui_token_amount,
  })
}

fn convert_inner_instructions(
  ui_instructions: UiInnerInstructions,
) -> anyhow::Result<InnerInstructions> {
  let index: Option<_> = ui_instructions.index.into();

  Ok(InnerInstructions {
      index: index.ok_or(anyhow!("Index is missing"))?,
      instructions: ui_instructions
          .instructions
          .iter()
          .map(|ix| match ix {
              UiInstruction::Parsed(_) => {
                  bail!("There should not be parsed instruction here")
              }
              UiInstruction::Compiled(instruction) => Ok(InnerInstruction {
                  instruction: CompiledInstruction {
                      program_id_index: instruction.program_id_index,
                      accounts: instruction.accounts.clone(),
                      data: bs58::decode(&instruction.data).into_vec()?,
                  },
                  stack_height: instruction.stack_height,
              }),
          })
          .collect::<Result<Vec<_>>>()?,
  })
}

pub fn convert_account(
  account_update: yellowstone_grpc_proto::geyser::SubscribeUpdateAccountInfo,
) -> Result<Account, String> {
  Ok(Account {
      lamports: account_update.lamports,
      data: account_update.data,
      owner: Pubkey::try_from(account_update.owner).unwrap(),
      executable: account_update.executable,
      rent_epoch: account_update.rent_epoch,
  })
}
