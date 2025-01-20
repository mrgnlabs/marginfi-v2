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
      // systemProgram
    })
    .instruction();

  return ix;
};

export type DepositArgs = {
  marginfiAccount: PublicKey;
  bank: PublicKey;
  tokenAccount: PublicKey;
  amount: BN;
  bankRunAddons?: {
    group: PublicKey;
    authority: PublicKey;
  };
};

/**
 * Deposit to a bank
 * * `authority`- MarginfiAccount's authority must sign and own the `tokenAccount`
 * @param program
 * @param args
 * @returns
 */
export const depositIx = (program: Program<Marginfi>, args: DepositArgs) => {
  if (args.bankRunAddons) {
    const ix = program.methods
      .lendingAccountDeposit(args.amount)
      .accountsPartial({
        group: args.bankRunAddons.group,
        marginfiAccount: args.marginfiAccount,
        authority: args.bankRunAddons.authority,
        bank: args.bank,
        signerTokenAccount: args.tokenAccount,
        liquidityVault: deriveLiquidityVault(program.programId, args.bank)[0],
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    return ix;
  } else {
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
  }
};

export type BorrowIxArgs = {
  marginfiAccount: PublicKey;
  bank: PublicKey;
  tokenAccount: PublicKey;
  remaining: PublicKey[];
  amount: BN;
  bankRunAddons?: {
    group: PublicKey;
    authority: PublicKey;
  };
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
  if (args.bankRunAddons) {
    const ix = program.methods
      .lendingAccountBorrow(args.amount)
      .accountsPartial({
        group: args.bankRunAddons.group,
        marginfiAccount: args.marginfiAccount,
        authority: args.bankRunAddons.authority,
        bank: args.bank,
        destinationTokenAccount: args.tokenAccount,
        liquidityVault: deriveLiquidityVault(program.programId, args.bank)[0],
        // bankLiquidityVault = deriveLiquidityVault(id, bank)
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .remainingAccounts(oracleMeta)
      .instruction();
    return ix;
  } else {
    const ix = program.methods
      .lendingAccountBorrow(args.amount)
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
  }
};

export type WithdrawIxArgs = {
  marginfiAccount: PublicKey;
  bank: PublicKey;
  tokenAccount: PublicKey;
  remaining: PublicKey[];
  amount: BN;
  bankRunAddons?: {
    group: PublicKey;
    authority: PublicKey;
  };
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
  if (args.bankRunAddons) {
    const ix = program.methods
      .lendingAccountWithdraw(args.amount, null)
      .accountsPartial({
        group: args.bankRunAddons.group,
        marginfiAccount: args.marginfiAccount,
        authority: args.bankRunAddons.authority,
        bank: args.bank,
        destinationTokenAccount: args.tokenAccount,
        // bankLiquidityVaultAuthority = deriveLiquidityVaultAuthority(id, bank);
        liquidityVault: deriveLiquidityVault(program.programId, args.bank)[0],
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .remainingAccounts(oracleMeta)
      .instruction();
    return ix;
  } else {
    const ix = program.methods
      .lendingAccountWithdraw(args.amount, null)
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
  }
};
