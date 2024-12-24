import { PublicKey } from "@solana/web3.js";

export const deriveStakeHolder = (
  programId: PublicKey,
  voteAccount: PublicKey,
  admin: PublicKey
) => {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("stakeholder", "utf-8"),
      voteAccount.toBuffer(),
      admin.toBuffer(),
    ],
    programId
  );
};

export const deriveStakeHolderStakeAccount = (
  programId: PublicKey,
  stakeholder: PublicKey
) => {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("stakeacc", "utf-8"), stakeholder.toBuffer()],
    programId
  );
};

export const deriveStakeUser = (programId: PublicKey, payer: PublicKey) => {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("stakeuser", "utf-8"), payer.toBuffer()],
    programId
  );
};
