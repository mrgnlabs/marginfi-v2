import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { Marginfi } from "../../target/types/marginfi";

export type GroupInitializeArgs = {
  marginfiGroup: PublicKey;
  admin: PublicKey;
};

export const groupInitialize = (
  program: Program<Marginfi>,
  args: GroupInitializeArgs
) => {
  const ix = program.methods
    .marginfiGroupInitialize()
    .accounts({
      marginfiGroup: args.marginfiGroup,
      admin: args.admin,
      // systemProgram: SystemProgram.programId,
    })
    .instruction();

  return ix;
};
