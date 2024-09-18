import { BN, Program } from "@coral-xyz/anchor";
import { AccountMeta, PublicKey, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { Marginfi } from "../../target/types/marginfi";
import {
  deriveFeeVault,
  deriveFeeVaultAuthority,
  deriveInsuranceVault,
  deriveInsuranceVaultAuthority,
  deriveLiquidityVault,
  deriveLiquidityVaultAuthority,
} from "./pdas";
import { BankConfig } from "./types";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BankConfigOptRaw } from "@mrgnlabs/marginfi-client-v2";

export const MAX_ORACLE_KEYS = 5;

/**
 * * admin/feePayer - must sign
 * * bank - use a fresh keypair, must sign
 */
export type AddBankArgs = {
  marginfiGroup: PublicKey;
  admin: PublicKey;
  feePayer: PublicKey;
  bankMint: PublicKey;
  bank: PublicKey;
  config: BankConfig;
};

export const addBank = (program: Program<Marginfi>, args: AddBankArgs) => {
  // const id = program.programId;
  // const bank = args.bank;

  // Note: oracle is passed as a key in config AND as an acc in remaining accs
  const oracleMeta: AccountMeta = {
    pubkey: args.config.oracleKey,
    isSigner: false,
    isWritable: false,
  };

  const ix = program.methods
    .lendingPoolAddBank({
      assetWeightInit: args.config.assetWeightInit,
      assetWeightMaint: args.config.assetWeightMaint,
      liabilityWeightInit: args.config.liabilityWeightInit,
      liabilityWeightMaint: args.config.liabilityWeightMain,
      depositLimit: args.config.depositLimit,
      interestRateConfig: args.config.interestRateConfig,
      operationalState: args.config.operationalState,
      oracleSetup: args.config.oracleSetup,
      oracleKey: args.config.oracleKey,
      borrowLimit: args.config.borrowLimit,
      riskTier: args.config.riskTier,
      pad0: [0, 0, 0, 0, 0, 0, 0],
      totalAssetValueInitLimit: args.config.totalAssetValueInitLimit,
      oracleMaxAge: args.config.oracleMaxAge,
    })
    .accounts({
      marginfiGroup: args.marginfiGroup,
      admin: args.admin,
      feePayer: args.feePayer,
      bankMint: args.bankMint,
      bank: args.bank,
      // liquidityVaultAuthority = deriveLiquidityVaultAuthority(id, bank);
      // liquidityVault = deriveLiquidityVault(id, bank);
      // insuranceVaultAuthority = deriveInsuranceVaultAuthority(id, bank);
      // insuranceVault = deriveInsuranceVault(id, bank);
      // feeVaultAuthority = deriveFeeVaultAuthority(id, bank);
      // feeVault = deriveFeeVault(id, bank);
      // rent = SYSVAR_RENT_PUBKEY
      tokenProgram: TOKEN_PROGRAM_ID,
      // systemProgram: SystemProgram.programId,
    })
    .remainingAccounts([oracleMeta])
    .instruction();

  return ix;
};

/**
 * newAdmin - (Optional) pass null to keep current admin
 * admin - must sign, must be current admin of marginfiGroup
 */
export type GroupConfigureArgs = {
  newAdmin: PublicKey | null;
  marginfiGroup: PublicKey;
  admin: PublicKey;
};

export const groupConfigure = (
  program: Program<Marginfi>,
  args: GroupConfigureArgs
) => {
  const ix = program.methods
    .marginfiGroupConfigure({ admin: args.newAdmin })
    .accounts({
      marginfiGroup: args.marginfiGroup,
      admin: args.admin,
    })
    .instruction();

  return ix;
};

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

export type ConfigureBankArgs = {
  marginfiGroup: PublicKey;
  admin: PublicKey;
  bank: PublicKey;
  bankConfigOpt: BankConfigOptRaw;
};

export const configureBank = (
  program: Program<Marginfi>,
  args: ConfigureBankArgs
) => {
  const ix = program.methods
    .lendingPoolConfigureBank(args.bankConfigOpt)
    .accounts({
      marginfiGroup: args.marginfiGroup,
      admin: args.admin,
      bank: args.bank,
    })
    .instruction();
  return ix;
};

export type SetupEmissionsArgs = {
  marginfiGroup: PublicKey;
  admin: PublicKey;
  bank: PublicKey;
  emissionsMint: PublicKey;
  fundingAccount: PublicKey;
  emissionsFlags: BN;
  emissionsRate: BN;
  totalEmissions: BN;
};

export const setupEmissions = (
  program: Program<Marginfi>,
  args: SetupEmissionsArgs
) => {
  const ix = program.methods
    .lendingPoolSetupEmissions(
      args.emissionsFlags,
      args.emissionsRate,
      args.totalEmissions
    )
    .accounts({
      marginfiGroup: args.marginfiGroup,
      admin: args.admin,
      bank: args.bank,
      emissionsMint: args.emissionsMint,
      // emissionsAuth: deriveEmissionsAuth()
      // emissionsTokenAccount: deriveEmissionsTokenAccount()
      emissionsFundingAccount: args.fundingAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
      // systemProgram: SystemProgram.programId,
    })
    .instruction();
  return ix;
};

export type UpdateEmissionsArgs = {
  marginfiGroup: PublicKey;
  admin: PublicKey;
  bank: PublicKey;
  emissionsMint: PublicKey;
  fundingAccount: PublicKey;
  emissionsFlags: BN | null;
  emissionsRate: BN | null;
  additionalEmissions: BN | null;
};

export const updateEmissions = (
  program: Program<Marginfi>,
  args: UpdateEmissionsArgs
) => {
  const ix = program.methods
    .lendingPoolUpdateEmissionsParameters(
      args.emissionsFlags,
      args.emissionsRate,
      args.additionalEmissions
    )
    .accounts({
      marginfiGroup: args.marginfiGroup,
      admin: args.admin,
      bank: args.bank,
      emissionsMint: args.emissionsMint,
      // emissionsAuth: deriveEmissionsAuth()
      // emissionsTokenAccount: deriveEmissionsTokenAccount()
      emissionsFundingAccount: args.fundingAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
      // systemProgram: SystemProgram.programId,
    })
    .instruction();
  return ix;
};