use crate::{
    state::{health_cache::HealthCache, marginfi_group::BankConfigOpt},
    StakedSettingsEditConfig,
};
use anchor_lang::prelude::*;

// Event headers

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GroupEventHeader {
    pub signer: Option<Pubkey>,
    pub marginfi_group: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AccountEventHeader {
    pub signer: Option<Pubkey>,
    pub marginfi_account: Pubkey,
    pub marginfi_account_authority: Pubkey,
    pub marginfi_group: Pubkey,
}

// marginfi group events

#[event]
pub struct MarginfiGroupCreateEvent {
    pub header: GroupEventHeader,
}

#[event]
pub struct MarginfiGroupConfigureEvent {
    pub header: GroupEventHeader,
    pub admin: Pubkey,
    pub flags: u64,
}

#[event]
pub struct LendingPoolBankCreateEvent {
    pub header: GroupEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
}

#[event]
pub struct LendingPoolBankConfigureEvent {
    pub header: GroupEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub config: BankConfigOpt,
}

#[event]
pub struct LendingPoolBankConfigureOracleEvent {
    pub header: GroupEventHeader,
    pub bank: Pubkey,
    pub oracle_setup: u8,
    pub oracle: Pubkey,
}

#[event]
pub struct LendingPoolBankConfigureFrozenEvent {
    pub header: GroupEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub deposit_limit: u64,
    pub borrow_limit: u64,
}

#[event]
pub struct EditStakedSettingsEvent {
    pub group: Pubkey,
    pub settings: StakedSettingsEditConfig,
}

#[event]
pub struct LendingPoolBankAccrueInterestEvent {
    pub header: GroupEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub delta: u64,
    pub fees_collected: f64,
    pub insurance_collected: f64,
}

#[event]
pub struct LendingPoolBankCollectFeesEvent {
    pub header: GroupEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub group_fees_collected: f64,
    pub group_fees_outstanding: f64,
    pub insurance_fees_collected: f64,
    pub insurance_fees_outstanding: f64,
}

#[event]
pub struct LendingPoolBankHandleBankruptcyEvent {
    pub header: AccountEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub bad_debt: f64,
    pub covered_amount: f64,
    pub socialized_amount: f64,
}

// marginfi account events

#[event]
pub struct MarginfiAccountCreateEvent {
    pub header: AccountEventHeader,
}

#[event]
pub struct LendingAccountDepositEvent {
    pub header: AccountEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

#[event]
pub struct LendingAccountRepayEvent {
    pub header: AccountEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub close_balance: bool,
}

#[event]
pub struct LendingAccountBorrowEvent {
    pub header: AccountEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

#[event]
pub struct LendingAccountWithdrawEvent {
    pub header: AccountEventHeader,
    pub bank: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub close_balance: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct LiquidationBalances {
    pub liquidatee_asset_balance: f64,
    pub liquidatee_liability_balance: f64,
    pub liquidator_asset_balance: f64,
    pub liquidator_liability_balance: f64,
}

#[event]
pub struct LendingAccountLiquidateEvent {
    pub header: AccountEventHeader,
    pub liquidatee_marginfi_account: Pubkey,
    pub liquidatee_marginfi_account_authority: Pubkey,
    pub asset_bank: Pubkey,
    pub asset_mint: Pubkey,
    pub liability_bank: Pubkey,
    pub liability_mint: Pubkey,
    pub liquidatee_pre_health: f64,
    pub liquidatee_post_health: f64,
    pub pre_balances: LiquidationBalances,
    pub post_balances: LiquidationBalances,
}

#[event]
pub struct MarginfiAccountTransferAccountAuthorityEvent {
    pub header: AccountEventHeader,
    pub old_account_authority: Pubkey,
    pub new_account_authority: Pubkey,
}

#[event]
pub struct HealthPulseEvent {
    pub account: Pubkey,
    pub health_cache: HealthCache,
}

// const config: Config = {
//     PROGRAM_ID: "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA",
//     TX_SIG: "45kWKVHmjudf5oaswAHDh6xrbR34sa5eVXnYC9fbPSjAdGCRd9DZbemkauFZ8obaTPmypjk5oKdapnNgbNUE6wtm",
//     // The Liquidate event is 408 bytes, plus something (probably the Option) adds one byte.
//     EVENT_EXPECTED_SIZE: 409,
//   };
  
//   async function main() {
//     marginfiIdl.address = config.PROGRAM_ID;
//     const connection = new Connection("https://api.mainnet-beta.solana.com", "confirmed");
//     const wallet = loadKeypairFromFile(process.env.HOME + "/keys/staging-deploy.json");
  
//     // @ts-ignore
//     const provider = new AnchorProvider(connection, wallet, {
//       preflightCommitment: "confirmed",
//     });
//     const program: Program<Marginfi> = new Program(marginfiIdl as Marginfi, provider);
  
//     const tx = await connection.getTransaction(config.TX_SIG, {
//       maxSupportedTransactionVersion: 0,
//     });
//     const coder = new BorshCoder(program.idl);
//     const parser = new EventParser(program.programId, coder);
  
//     const logs = tx.meta?.logMessages || [];
//     const decodedEvents = [];
  
//     // const key = new PublicKey("2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB");
//     // console.log("KEY ACTUAL:");
//     // const keyBytes = key.toBuffer();
//     // const keyHexStr = Array.from(keyBytes)
//     //   .map((b) => b.toString(16).padStart(2, "0"))
//     //   .join(" ");
//     // console.log(" ", keyHexStr);
//     // console.log("END KEY ACTUAL");
  
//     for (const logLine of logs) {
//       //console.log("line: " + logLine);
//       let decoded = base64.decode(logLine.substring("Program data: ".length));
//       // console.log("decoded bytes: " + decoded.length);
//       if (decoded.length == config.EVENT_EXPECTED_SIZE) {
//         console.log("decoded bytes (chunked hex):");
//         const bytesPerLine = 16;
//         for (let i = 0; i < decoded.length; i += bytesPerLine) {
//           const chunk = decoded.subarray(i, i + bytesPerLine);
//           const hexLine = Array.from(chunk)
//             .map((b) => b.toString(16).padStart(2, "0"))
//             .join(" ");
//           console.log(
//             `[${i.toString().padStart(4, "0")}..${(i + chunk.length - 1).toString().padStart(4, "0")}]  ${hexLine}`
//           );
//         }
  
//         // if you want, you can just read the raw bytes here into a float or whatever...
//       }
//       try {
//         // pass one event at a time because parseLogs throws on decode errors...
//         const events = parser.parseLogs([logLine], false);
//         decodedEvents.push(...events);
//       } catch (err) {
//         // If a log line fails just print and ignore it, we likely just don't have the decoder...
//         // console.warn(`Could not decode event from log:\n  ${logLine}\n`);
//         // console.warn(`Error:`, err);
//       }
//     }
  
//     console.log("Decoded events:");
//     for (const event of parser.parseLogs(logs)) {
//       console.log(event);
//     }
  
//     // Other usage ideas:
//     // (1) const events = [...program.parseLogs(logs)];
  
//     // (2) const parser = program.parseLogs(logs);
//     // const firstEvent = parser.next().value;
  
//     // (3) const eventsWithErrors = [...program.parseLogs(logs, true)];
//   }