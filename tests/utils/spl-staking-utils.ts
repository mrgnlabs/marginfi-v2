import {
  findPoolMintAddress,
  findPoolStakeAuthorityAddress,
  SinglePoolInstruction,
} from "@solana/spl-single-pool-classic";
import {
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  Connection,
  PublicKey,
  STAKE_CONFIG_ID,
  StakeAuthorizationLayout,
  StakeProgram,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_SLOT_HISTORY_PUBKEY,
  SYSVAR_STAKE_HISTORY_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";
import { SINGLE_POOL_PROGRAM_ID } from "./types";
import { ProgramTestContext } from "solana-bankrun";

export enum SinglePoolAccountType {
  Uninitialized = 0,
  Pool = 1,
}

export type SinglePool = {
  accountType: SinglePoolAccountType;
  voteAccountAddress: PublicKey;
};

const decodeSinglePoolAccountType = (buffer: Buffer, offset: number) => {
  const accountType = buffer.readUInt8(offset);
  if (accountType === 0) {
    return SinglePoolAccountType.Uninitialized;
  } else if (accountType === 1) {
    return SinglePoolAccountType.Pool;
  } else {
    throw new Error("Unknown SinglePoolAccountType");
  }
};

/**
 * Decode an spl single pool from buffer.
 *
 * Get the data buffer with `const data = (await provider.connection.getAccountInfo(poolKey)).data;`
 * and note that there is no discriminator (i.e. pass data directly without additional slicing)
 */
export const decodeSinglePool = (buffer: Buffer) => {
  let offset = 0;

  const accountType = decodeSinglePoolAccountType(buffer, offset);
  offset += 1;

  const voteAccountAddress = new PublicKey(
    buffer.subarray(offset, offset + 32)
  );
  offset += 32;

  return {
    accountType,
    voteAccountAddress,
  };
};

// See `https://www.npmjs.com/package/@solana/spl-single-pool` transactions.ts for the original

/**
 * Builds ixes to create the LST ata as-needed, pass stake authority to the spl pool, and deposit to
 * the stake pool
 * @param connection
 * @param userWallet
 * @param splPool
 * @param userStakeAccount
 * @param verbose
 * @returns
 */
export const depositToSinglePoolIxes = async (
  connection: Connection,
  userWallet: PublicKey,
  splPool: PublicKey,
  userStakeAccount: PublicKey,
  verbose: boolean = false
) => {
  const splMint = await findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, splPool);

  const splAuthority = await findPoolStakeAuthorityAddress(
    SINGLE_POOL_PROGRAM_ID,
    splPool
  );

  const ixes: TransactionInstruction[] = [];
  const lstAta = getAssociatedTokenAddressSync(splMint, userWallet);
  try {
    await connection.getAccountInfo(lstAta);
    if (verbose) {
      console.log("Existing LST ata at: " + lstAta);
    }
  } catch (err) {
    if (verbose) {
      console.log("Failed to find ata, creating: " + lstAta);
    }
    ixes.push(
      createAssociatedTokenAccountInstruction(
        userWallet,
        lstAta,
        userWallet,
        splMint
      )
    );
  }

  const authorizeStakerIxes = StakeProgram.authorize({
    stakePubkey: userStakeAccount,
    authorizedPubkey: userWallet,
    newAuthorizedPubkey: splAuthority,
    stakeAuthorizationType: StakeAuthorizationLayout.Staker,
  }).instructions;

  ixes.push(...authorizeStakerIxes);

  const authorizeWithdrawIxes = StakeProgram.authorize({
    stakePubkey: userStakeAccount,
    authorizedPubkey: userWallet,
    newAuthorizedPubkey: splAuthority,
    stakeAuthorizationType: StakeAuthorizationLayout.Withdrawer,
  }).instructions;

  ixes.push(...authorizeWithdrawIxes);

  const depositIx = await SinglePoolInstruction.depositStake(
    splPool,
    userStakeAccount,
    lstAta,
    userWallet
  );

  ixes.push(depositIx);

  return ixes;
};

/**
 * Generally, use this instead of `bankrunContext.lastBlockhash` (which does not work if the test
 * has already run for some time and the blockhash has advanced)
 * @param bankrunContext
 * @returns
 */
export const getBankrunBlockhash = async (
  bankrunContext: ProgramTestContext
) => {
  return (await bankrunContext.banksClient.getLatestBlockhash())[0];
};

/**
 * Spl Single Pool's InitializeTempStake instruction.
 *
 * Accounts (in order):
 *
 *   0. [] Pool account
 *   1. [w] Pool stake account
 *   2. [w] Temp stake account
 *   3. [] Pool stake authority
 *   4. [] Vote account
 *   5. [] Rent sysvar
 *   6. [] Clock sysvar
 *   7. [] Stake history sysvar
 *   8. [] Stake config sysvar
 *   9. [] System program
 *  10. [] Stake program
 *
 * @param poolAccount - The pool account public key.
 * @param tempStakeAccount - The temporary stake account public key (writable).
 * @param poolStakeAuthority - The pool stake authority public key.
 * @param voteAccount - The vote account public key.
 * @param programId - The program ID for svsp (typically `SINGLE_POOL_PROGRAM_ID`)
 *
 * @returns A TransactionInstruction that can be added to a transaction.
 */
export function createInitializeTempStakeInstruction(
  poolAccount: PublicKey,
  poolStakeAccount: PublicKey,
  tempStakeAccount: PublicKey,
  poolStakeAuthority: PublicKey,
  voteAccount: PublicKey,
  programId: PublicKey
): TransactionInstruction {
  const keys = [
    { pubkey: poolAccount, isSigner: false, isWritable: false },
    { pubkey: poolStakeAccount, isSigner: false, isWritable: true },
    { pubkey: tempStakeAccount, isSigner: false, isWritable: true },
    { pubkey: poolStakeAuthority, isSigner: false, isWritable: false },
    { pubkey: voteAccount, isSigner: false, isWritable: false },
    { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    { pubkey: SYSVAR_STAKE_HISTORY_PUBKEY, isSigner: false, isWritable: false },
    { pubkey: STAKE_CONFIG_ID, isSigner: false, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    { pubkey: StakeProgram.programId, isSigner: false, isWritable: false },
  ];

  // TODO don't hard code the instruction index? (or why not, it's not gna change is it?)
  const data = Buffer.from(Uint8Array.of(6));

  return new TransactionInstruction({
    keys,
    programId,
    data,
  });
}
