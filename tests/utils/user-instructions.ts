import { BN, Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Marginfi } from "../../target/types/marginfi";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

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
  marginfiGroup: PublicKey;
  marginfiAccount: PublicKey;
  authority: PublicKey;
  bank: PublicKey;
  tokenAccount: PublicKey;
  amount: BN;
};

/**
 * Deposit to a bank
 * * `authority` must sign and own the `tokenAccount`
 * @param program
 * @param args
 * @returns
 */
export const depositIx = (program: Program<Marginfi>, args: DepositArgs) => {
  const ix = program.methods
    .lendingAccountDeposit(args.amount)
    .accounts({
      marginfiGroup: args.marginfiGroup,
      marginfiAccount: args.marginfiAccount,
      signer: args.authority,
      bank: args.bank,
      signerTokenAccount: args.tokenAccount,
      // liquidityVault = deriveLiquidityVault(id, bank)
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();

  return ix;
};
