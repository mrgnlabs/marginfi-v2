import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

export const deriveLiquidityVaultAuthority = (
  programId: PublicKey,
  bank: PublicKey
) => {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("liquidity_vault_auth", "utf-8"), bank.toBuffer()],
    programId
  );
};

export const deriveLiquidityVault = (programId: PublicKey, bank: PublicKey) => {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("liquidity_vault", "utf-8"), bank.toBuffer()],
    programId
  );
};

export const deriveInsuranceVaultAuthority = (
  programId: PublicKey,
  bank: PublicKey
) => {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("insurance_vault_auth", "utf-8"), bank.toBuffer()],
    programId
  );
};

export const deriveInsuranceVault = (programId: PublicKey, bank: PublicKey) => {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("insurance_vault", "utf-8"), bank.toBuffer()],
    programId
  );
};

export const deriveFeeVaultAuthority = (
  programId: PublicKey,
  bank: PublicKey
) => {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fee_vault_auth", "utf-8"), bank.toBuffer()],
    programId
  );
};

export const deriveFeeVault = (programId: PublicKey, bank: PublicKey) => {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fee_vault", "utf-8"), bank.toBuffer()],
    programId
  );
};

export const deriveEmissionsAuth = (
  programId: PublicKey,
  bank: PublicKey,
  mint: PublicKey
) => {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("emissions_auth_seed", "utf-8"),
      bank.toBuffer(),
      mint.toBuffer(),
    ],
    programId
  );
};

export const deriveEmissionsTokenAccount = (
  programId: PublicKey,
  bank: PublicKey,
  mint: PublicKey
) => {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("emissions_token_account_seed", "utf-8"),
      bank.toBuffer(),
      mint.toBuffer(),
    ],
    programId
  );
};

export const deriveBankWithSeed = (
  programId: PublicKey,
  group: PublicKey,
  bankMint: PublicKey,
  seed: BN
) => {
  return PublicKey.findProgramAddressSync(
    [group.toBuffer(), bankMint.toBuffer(), seed.toArrayLike(Buffer, "le", 8)],
    programId
  );
};

// ************* Below this line, not yet included in package ****************

export const deriveGlobalFeeState = (programId: PublicKey) => {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("feestate", "utf-8")],
    programId
  );
};

export const deriveStakedSettings = (
  programId: PublicKey,
  group: PublicKey
) => {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("staked_settings", "utf-8"), group.toBuffer()],
    programId
  );
};
