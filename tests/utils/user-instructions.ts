import { BN, Program } from "@coral-xyz/anchor";
import { AccountMeta, PublicKey } from "@solana/web3.js";
import { Marginfi } from "../../target/types/marginfi";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { deriveLiquidityVault } from "./pdas";

export type AccountInitArgs = {
  marginfiGroup: PublicKey;
  marginfiAccount: PublicKey;
  authority: PublicKey;
  feePayer: PublicKey;
};

/**
 * Init a user account for some group.
 * * fee payer and authority must both sign.
 * * account must be a fresh keypair and must also sign
 * @param program
 * @param args
 * @returns
 */
export const accountInit = (
  program: Program<Marginfi>,
  args: AccountInitArgs
) => {
  const ix = program.methods
    .marginfiAccountInitialize()
    .accounts({
      marginfiGroup: args.marginfiGroup,
      marginfiAccount: args.marginfiAccount,
      authority: args.authority,
      feePayer: args.feePayer,
    })
    .instruction();

  return ix;
};

export type DepositArgs = {
  marginfiAccount: PublicKey;
  bank: PublicKey;
  tokenAccount: PublicKey;
  amount: BN;
};

/**
 * Deposit to a bank
 * * `authority`- MarginfiAccount's authority must sign and own the `tokenAccount`
 * @param program
 * @param args
 * @returns
 */
export const depositIx = (program: Program<Marginfi>, args: DepositArgs) => {
  const ix = program.methods
    .lendingAccountDeposit(args.amount)
    .accounts({
      // marginfiGroup: args.marginfiGroup, // implied from bank
      marginfiAccount: args.marginfiAccount,
      // authority: args.authority, // implied from marginfiAccount
      bank: args.bank,
      signerTokenAccount: args.tokenAccount,
      // bankLiquidityVault:  deriveLiquidityVault(id, bank)
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();

  return ix;
};

export type SettleEmissionsArgs = {
  marginfiAccount: PublicKey;
  bank: PublicKey;
};

/**
 * (Permissionless) Settle emissions for a mrgnfi bank that is emitting some rewards. Generally runs
 * for all users before rates are updated, otherwise past emissions are retroactively credited at
 * the new rate as well. See `withdrawEmissionsIx` to actually claim the emissions to a wallet.
 * * `authority`- MarginfiAccount's authority must sign and own the `tokenAccount`
 * @param program
 * @param args
 * @returns
 */
export const settleEmissionsIx = (
  program: Program<Marginfi>,
  args: SettleEmissionsArgs
) => {
  const ix = program.methods
    .lendingAccountSettleEmissions()
    .accounts({
      marginfiAccount: args.marginfiAccount,
      bank: args.bank,
    })
    .instruction();

  return ix;
};

export type WithdrawEmissionsArgs = {
  marginfiAccount: PublicKey;
  bank: PublicKey;
  tokenAccount: PublicKey;
};

/**
 * Settles AND withdraws emissions to the user's given token account. Also see `settleEmissionsIx`, which settles but does not withdraw.
 * * `authority`- MarginfiAccount's authority must sign but does not have to own the `tokenAccount`
 * @param program
 * @param args
 * @returns
 */
export const withdrawEmissionsIx = (
  program: Program<Marginfi>,
  args: WithdrawEmissionsArgs
) => {
  const ix = program.methods
    .lendingAccountWithdrawEmissions()
    .accounts({
      // group: args.marginfiGroup, // implied from bank
      marginfiAccount: args.marginfiAccount,
      // authority: args.authority, // implied from marginfiAccount
      bank: args.bank,
      // emissions_mint // implied from bank
      // emissions_auth // pda derived from bank
      // emissions_vault // pda derived from bank
      destinationAccount: args.tokenAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();

  return ix;
};

export type BorrowIxArgs = {
  marginfiAccount: PublicKey;
  bank: PublicKey;
  tokenAccount: PublicKey;
  remaining: PublicKey[];
  amount: BN;
};

/**
 * Borrow from a bank
 * * `authority` - marginfiAccount's authority must sign, but does not have to own the `tokenAccount`
 * * `remaining` - pass bank/oracles for each bank the user is involved with, in the SAME ORDER they
 *   appear in userAcc.balances (e.g. `[bank0, oracle0, bank1, oracle1]`). For Token22 assets, pass
 *   the mint first, then the oracles/banks as described earlier.
 * @param program
 * @param args
 * @returns
 */
export const borrowIx = (program: Program<Marginfi>, args: BorrowIxArgs) => {
  const oracleMeta: AccountMeta[] = args.remaining.map((pubkey) => ({
    pubkey,
    isSigner: false,
    isWritable: false,
  }));
  const ix = program.methods
    .lendingAccountBorrow(args.amount)
    .accounts({
      // marginfiGroup: args.marginfiGroup, // implied from bank
      marginfiAccount: args.marginfiAccount,
      // authority: args.authority, // implied from account
      bank: args.bank,
      destinationTokenAccount: args.tokenAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .remainingAccounts(oracleMeta)
    .instruction();

  return ix;
};

export type WithdrawIxArgs = {
  marginfiAccount: PublicKey;
  bank: PublicKey;
  tokenAccount: PublicKey;
  remaining: PublicKey[];
  amount: BN;
  withdrawAll?: boolean;
};

/**
 * Withdraw from a bank
 * * `authority` - marginfiAccount's authority must sign, but does not have to own the `tokenAccount`
 * * `remaining` - pass bank/oracles for each bank the user is involved with, in the SAME ORDER they
 *   appear in userAcc.balances (e.g. `[bank0, oracle0, bank1, oracle1]`). For Token22 assets, pass
 *   the mint first, then the oracles/banks as described earlier.
 * @param program
 * @param args
 * @returns
 */
export const withdrawIx = (
  program: Program<Marginfi>,
  args: WithdrawIxArgs
) => {
  const oracleMeta: AccountMeta[] = args.remaining.map((pubkey) => ({
    pubkey,
    isSigner: false,
    isWritable: false,
  }));
  // False is the same as null, so if false we'll just pass null
  const all = args.withdrawAll === true ? true : null;
  const ix = program.methods
    .lendingAccountWithdraw(args.amount, all)
    .accounts({
      // marginfiGroup: args.marginfiGroup, // implied from bank
      marginfiAccount: args.marginfiAccount,
      // authority: args.authority, // implied from account
      bank: args.bank,
      destinationTokenAccount: args.tokenAccount,
      // bankLiquidityVaultAuthority = deriveLiquidityVaultAuthority(id, bank);
      // bankLiquidityVault = deriveLiquidityVault(id, bank)
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .remainingAccounts(oracleMeta)
    .instruction();

  return ix;
};

export type RepayIxArgs = {
  marginfiAccount: PublicKey;
  bank: PublicKey;
  tokenAccount: PublicKey;
  remaining: PublicKey[];
  amount: BN;
  repayAll?: boolean;
};

/**
 * Repay debt to a bank
 * * `authority` - MarginfiAccount's authority must sign and own the `tokenAccount`
 * * `remaining` - pass bank/oracles for each bank the user is involved with, in the SAME ORDER they
 *   appear in userAcc.balances (e.g. `[bank0, oracle0, bank1, oracle1]`). For Token22 assets, pass
 *   the mint first, then the oracles/banks as described earlier.
 * @param program
 * @param args
 * @returns
 */
export const repayIx = (program: Program<Marginfi>, args: RepayIxArgs) => {
  const oracleMeta: AccountMeta[] = args.remaining.map((pubkey) => ({
    pubkey,
    isSigner: false,
    isWritable: false,
  }));
  // False is the same as null, so if false we'll just pass null
  const all = args.repayAll === true ? true : null;
  const ix = program.methods
    .lendingAccountRepay(args.amount, all)
    .accounts({
      // marginfiGroup: args.marginfiGroup, // implied from bank
      marginfiAccount: args.marginfiAccount,
      // authority: args.authority, // implied from account
      bank: args.bank,
      signerTokenAccount: args.tokenAccount,
      // bankLiquidityVaultAuthority = deriveLiquidityVaultAuthority(id, bank);
      // bankLiquidityVault = deriveLiquidityVault(id, bank)
            tokenProgram: TOKEN_PROGRAM_ID,
    })
    .remainingAccounts(oracleMeta)
    .instruction();
    return ix;
};

export type LiquidateIxArgs = {
  assetBankKey: PublicKey;
  liabilityBankKey: PublicKey;
  liquidatorMarginfiAccount: PublicKey;
  liquidateeMarginfiAccount: PublicKey;
  remaining: PublicKey[];
  amount: BN;
};

/**
 * Creates a Liquidate instruction.
 * `remaining`:
 *     liab_mint_ai (if token2022 mint),
 *     asset_oracle_ai,
 *     liab_oracle_ai,
 *     liquidator_observation_ais...,
 *     liquidatee_observation_ais...,
 *
 * @param program - The marginfi program instance.
 * @param args - The arguments required to create the instruction.
 * @returns The TransactionInstruction object.
 */
export const liquidateIx = (
  program: Program<Marginfi>,
  args: LiquidateIxArgs
) => {
  const oracleMeta: AccountMeta[] = args.remaining.map((pubkey) => {
    if (!(pubkey instanceof PublicKey)) {
      console.error("Invalid remaining key:", pubkey);
      throw new Error("remaining contains invalid keys");
    }

    return { pubkey, isSigner: false, isWritable: false };
  });

  // Return the instruction
  return program.methods
    .lendingAccountLiquidate(args.amount)
    .accounts({
      assetBank: args.assetBankKey,
      liabBank: args.liabilityBankKey,
      liquidatorMarginfiAccount: args.liquidatorMarginfiAccount,
      liquidateeMarginfiAccount: args.liquidateeMarginfiAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .remainingAccounts(oracleMeta)
    .instruction();
};
