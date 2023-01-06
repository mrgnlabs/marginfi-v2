import { PublicKey, SystemProgram } from "@solana/web3.js";
import { MarginfiProgram } from "./types";

async function makeInitMarginfiAccountIx(
  mfProgram: MarginfiProgram,
  accounts: {
    marginfiGroupPk: PublicKey;
    marginfiAccountPk: PublicKey;
    signerPk: PublicKey;
  }
) {
  return mfProgram.methods
    .initializeMarginfiAccount()
    .accounts({
      marginfiGroup: accounts.marginfiGroupPk,
      marginfiAccount: accounts.marginfiAccountPk,
      signer: accounts.signerPk,
      systemProgram: SystemProgram.programId,
    })
    .instruction();
}

const instructions = {
  makeInitMarginfiAccountIx,
};

export default instructions;
