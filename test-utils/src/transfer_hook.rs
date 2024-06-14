use {
    solana_program::{entrypoint::ProgramResult, program_error::ProgramError},
    solana_sdk::{
        account_info::{next_account_info, AccountInfo},
        msg,
        pubkey::Pubkey,
    },
    spl_transfer_hook_interface::instruction::TransferHookInstruction,
};

pub static TEST_HOOK_ID: Pubkey =
    solana_sdk::pubkey!("TRANSFERHKTRANSFERHKTRANSFERHKTRANSFERHKTRA");

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = TransferHookInstruction::unpack(input)?;
    let amount = match instruction {
        TransferHookInstruction::Execute { amount } => amount,
        _ => return Err(ProgramError::InvalidInstructionData),
    };
    let account_info_iter = &mut accounts.iter();

    // Pull out the accounts in order, none are validated in this test program
    let _source_account_info = next_account_info(account_info_iter)?;
    let _mint_info = next_account_info(account_info_iter)?;
    let _destination_account_info = next_account_info(account_info_iter)?;
    let _authority_info = next_account_info(account_info_iter)?;
    let _extra_account_metas_info = next_account_info(account_info_iter)?;

    msg!("transfering {} tokens", amount);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program_test::{processor, tokio, ProgramTest};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signer::Signer,
        transaction::Transaction,
    };
    use spl_discriminator::SplDiscriminate;
    use spl_transfer_hook_interface::instruction::ExecuteInstruction;

    #[tokio::test]
    async fn invoke_hook() {
        let mut ctx = ProgramTest::new(
            "transfer_hook",
            TEST_HOOK_ID,
            processor!(super::process_instruction),
        )
        .start_with_context()
        .await;

        let mut data = <ExecuteInstruction as SplDiscriminate>::SPL_DISCRIMINATOR_SLICE.to_vec();
        data.extend_from_slice(&0_u64.to_le_bytes());

        let ix = Instruction {
            program_id: TEST_HOOK_ID,
            accounts: vec![
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
            ],
            data,
        };
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            ctx.last_blockhash,
        );

        ctx.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }
}
