import { Program } from "@coral-xyz/anchor";
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
import { WrappedI80F48 } from "@mrgnlabs/mrgn-common";

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

  // Note that oracle is passed as a key in config and as an acc in remaining accs...
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
      // globalFeeState: deriveGlobalFeeState(id),
      // globalFeeWallet: args.globalFeeWallet,
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
      // feeState: deriveGlobalFeeState(id),
      admin: args.admin,
      // systemProgram: SystemProgram.programId,
    })
    .instruction();

  return ix;
};

export type InitGlobalFeeStateArgs = {
  payer: PublicKey;
  admin: PublicKey;
  wallet: PublicKey;
  bankInitFlatSolFee: number;
  programFeeFixed: WrappedI80F48;
  programFeeRate: WrappedI80F48;
};

export const initGlobalFeeState = (
  program: Program<Marginfi>,
  args: InitGlobalFeeStateArgs
) => {
  const ix = program.methods
    .initGlobalFeeState(
      args.admin,
      args.wallet,
      args.bankInitFlatSolFee,
      args.programFeeFixed,
      args.programFeeRate
    )
    .accounts({
      payer: args.payer,
      // feeState = deriveGlobalFeeState(id),
      // rent = SYSVAR_RENT_PUBKEY,
      // systemProgram: SystemProgram.programId,
    })
    .instruction();

  return ix;
};

export type EditGlobalFeeStateArgs = {
  admin: PublicKey;
  wallet: PublicKey;
  bankInitFlatSolFee: number;
  programFeeFixed: WrappedI80F48;
  programFeeRate: WrappedI80F48;
};

// TODO add test for this
export const editGlobalFeeState = (
  program: Program<Marginfi>,
  args: EditGlobalFeeStateArgs
) => {
  const ix = program.methods
    .editGlobalFeeState(
      args.wallet,
      args.bankInitFlatSolFee,
      args.programFeeFixed,
      args.programFeeRate
    )
    .accounts({
      globalFeeAdmin: args.admin,
      // feeState = deriveGlobalFeeState(id),
    })
    .instruction();

  return ix;
};
