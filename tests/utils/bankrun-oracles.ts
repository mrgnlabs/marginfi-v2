import { Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { BanksClient, ProgramTestContext } from "solana-bankrun";
import { Oracles } from "./mocks";
import { ORACLE_CONF_INTERVAL, DRIFT_ORACLE_RECEIVER_PROGRAM_ID } from "./types";
import { processBankrunTransaction } from "./tools";

/**
 * Creates a blank pyth feed account in bankrun (300 bytes).
 *
 * @param owner - The program that owns this feed account. For Drift oracles, use DRIFT_ORACLE_RECEIVER_PROGRAM_ID.
 */
export async function createBankrunPythFeedAccount(
  bankrunContext: ProgramTestContext,
  banksClient: BanksClient,
  feedKeypair: Keypair,
  owner: PublicKey
): Promise<Keypair> {
  const space = 300;
  const rent = await banksClient.getRent();
  const lamports = Number(rent.minimumBalance(BigInt(space)));

  const tx = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: bankrunContext.payer.publicKey,
      newAccountPubkey: feedKeypair.publicKey,
      lamports,
      space,
      programId: owner,
    })
  );

  await processBankrunTransaction(bankrunContext, tx, [bankrunContext.payer, feedKeypair]);

  return feedKeypair;
}

/**
 * Creates a blank pyth oracle account in bankrun with specified owner (134 bytes).
 *
 * @param owner - The program that owns this oracle account. For Drift oracles, use DRIFT_ORACLE_RECEIVER_PROGRAM_ID.
 */
export async function createBankrunPythOracleAccount(
  bankrunContext: ProgramTestContext,
  banksClient: BanksClient,
  oracleKeypair: Keypair,
  owner: PublicKey
): Promise<Keypair> {
  const space = 134;
  const rent = await banksClient.getRent();
  const lamports = Number(rent.minimumBalance(BigInt(space)));

  const tx = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: bankrunContext.payer.publicKey,
      newAccountPubkey: oracleKeypair.publicKey,
      lamports,
      space,
      programId: owner,
    })
  );

  await processBankrunTransaction(bankrunContext, tx, [bankrunContext.payer, oracleKeypair]);

  return oracleKeypair;
}

/**
 * Sets a Pyth Pull oracle price directly using bankrun, bypassing transactions.
 * This avoids "Account in use" errors from concurrent transactions.
 * 
 * @param bankrunContext - The bankrun context
 * @param banksClient - The banks client to get clock from
 * @param oracleAccount - The PriceUpdateV2 account to update
 * @param feedAccount - The feed account referenced in the price update
 * @param price - The price value (will be multiplied by 10^decimals)
 * @param decimals - Number of decimals for the price
 * @param confidence - Confidence value as a multiplier (e.g., 0.01 for 1%)
 * @param owner - The program that owns the oracle account (defaults to pyth-solana-receiver)
 */
export async function setPythPullOraclePrice(
  bankrunContext: ProgramTestContext,
  banksClient: BanksClient,
  oracleAccount: PublicKey,
  feedAccount: PublicKey,
  price: number,
  decimals: number,
  confidence: number,
  owner: PublicKey = new PublicKey("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ")
) {
  // Get current clock for slot and timestamp
  const clock = await banksClient.getClock();
  const slot = new BN(Number(clock.slot));
  const publishTime = Number(clock.unixTimestamp);

  // Convert price to BN with decimals
  const priceBN = new BN(price * 10 ** decimals);
  const confBN = new BN(price * confidence * 10 ** decimals);

  // Build PriceUpdateV2 buffer (134 bytes)
  const buffer = Buffer.alloc(134);
  let offset = 0;

  // Discriminator (8 bytes)
  Buffer.from([34, 241, 35, 99, 157, 126, 244, 205]).copy(buffer, offset);
  offset += 8;

  // Write authority (32 bytes) - can be any pubkey
  PublicKey.unique().toBuffer().copy(buffer, offset);
  offset += 32;

  // Verification level - Full (1 byte)
  buffer.writeUInt8(1, offset);
  offset += 1;

  // Feed ID (32 bytes)
  feedAccount.toBuffer().copy(buffer, offset);
  offset += 32;

  // Price (i64, 8 bytes)
  priceBN.toArrayLike(Buffer, "le", 8).copy(buffer, offset);
  offset += 8;

  // Confidence (u64, 8 bytes)
  confBN.toArrayLike(Buffer, "le", 8).copy(buffer, offset);
  offset += 8;

  // Exponent (i32, 4 bytes)
  buffer.writeInt32LE(-decimals, offset);
  offset += 4;

  // Publish time (i64, 8 bytes)
  new BN(publishTime).toArrayLike(Buffer, "le", 8).copy(buffer, offset);
  offset += 8;

  // Previous publish time (i64, 8 bytes)
  new BN(publishTime - 1).toArrayLike(Buffer, "le", 8).copy(buffer, offset);
  offset += 8;

  // EMA price (i64, 8 bytes) - use same as price
  priceBN.toArrayLike(Buffer, "le", 8).copy(buffer, offset);
  offset += 8;

  // EMA conf (u64, 8 bytes) - use same as conf
  confBN.toArrayLike(Buffer, "le", 8).copy(buffer, offset);
  offset += 8;

  // Posted slot (u64, 8 bytes)
  slot.toArrayLike(Buffer, "le", 8).copy(buffer, offset);

  // Get existing account or create if it doesn't exist
  const existing = await banksClient.getAccount(oracleAccount);

  if (!existing) {
    console.log("Account does not exist, not creating because this causes bankrun issues")
    return
  } else {
    // Update existing account with new data
    bankrunContext.setAccount(oracleAccount, {
      lamports: existing.lamports,
      data: buffer,
      owner: existing.owner, // Preserve existing owner
      executable: existing.executable,
      rentEpoch: existing.rentEpoch,
    });
  }
}

/**
 * Updates all Pyth Pull oracles in the oracles object.
 * This is a drop-in replacement for refreshPullOracles that avoids "Account in use" errors.
 * 
 * @param oracles - The oracles object containing all oracle accounts and price data
 * @param bankrunContext - The bankrun context
 * @param banksClient - The banks client to get clock from
 * @param owner - The program that owns the oracle accounts (defaults to pyth-solana-receiver)
 */
export async function refreshPullOraclesBankrun(
  oracles: Oracles,
  bankrunContext: ProgramTestContext,
  banksClient: BanksClient,
  owner: PublicKey = new PublicKey("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ")
) {
  // Update each oracle sequentially to avoid any race conditions
  await setPythPullOraclePrice(
    bankrunContext,
    banksClient,
    oracles.pythPullLst.publicKey,
    oracles.pythPullLstOracleFeed.publicKey,
    oracles.lstAlphaPrice,
    oracles.lstAlphaDecimals,
    ORACLE_CONF_INTERVAL,
    owner
  );

  await setPythPullOraclePrice(
    bankrunContext,
    banksClient,
    oracles.wsolOracle.publicKey,
    oracles.wsolOracleFeed.publicKey,
    oracles.wsolPrice,
    oracles.wsolDecimals,
    ORACLE_CONF_INTERVAL,
    owner
  );

  await setPythPullOraclePrice(
    bankrunContext,
    banksClient,
    oracles.usdcOracle.publicKey,
    oracles.usdcOracleFeed.publicKey,
    oracles.usdcPrice,
    oracles.usdcDecimals,
    ORACLE_CONF_INTERVAL,
    owner
  );

  await setPythPullOraclePrice(
    bankrunContext,
    banksClient,
    oracles.tokenAOracle.publicKey,
    oracles.tokenAOracleFeed.publicKey,
    oracles.tokenAPrice,
    oracles.tokenADecimals,
    ORACLE_CONF_INTERVAL,
    owner
  );

  // await setPythPullOraclePrice(
  //   bankrunContext,
  //   banksClient,
  //   oracles.tokenBOracle.publicKey,
  //   oracles.tokenBOracleFeed.publicKey,
  //   oracles.tokenBPrice,
  //   oracles.tokenBDecimals,
  //   ORACLE_CONF_INTERVAL,
  //   owner
  // );
}