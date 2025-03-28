import { BN, Program } from "@coral-xyz/anchor";
import { AccountMeta, PublicKey, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { Marginfi } from "../../target/types/marginfi";
import {
  deriveBankWithSeed,
  deriveFeeVault,
  deriveFeeVaultAuthority,
  deriveInsuranceVault,
  deriveInsuranceVaultAuthority,
  deriveLiquidityVault,
  deriveLiquidityVaultAuthority,
  deriveStakedSettings,
} from "./pdas";
import {
  BankConfig,
  BankConfigOptWithAssetTag,
  SINGLE_POOL_PROGRAM_ID,
  StakedSettingsConfig,
  StakedSettingsEdit,
} from "./types";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BankConfigOptRaw } from "@mrgnlabs/marginfi-client-v2";
import { WrappedI80F48 } from "@mrgnlabs/mrgn-common";

export const MAX_ORACLE_KEYS = 5;

/**
 * * admin/feePayer - must sign
 * * bank - use a fresh keypair, must sign
 */
export type AddBankArgs = {
  marginfiGroup: PublicKey;
  feePayer: PublicKey;
  bankMint: PublicKey;
  bank: PublicKey;
  config: BankConfig;
};

export const addBank = (program: Program<Marginfi>, args: AddBankArgs) => {
  const ix = program.methods
    .lendingPoolAddBank({
      assetWeightInit: args.config.assetWeightInit,
      assetWeightMaint: args.config.assetWeightMaint,
      liabilityWeightInit: args.config.liabilityWeightInit,
      liabilityWeightMaint: args.config.liabilityWeightMain,
      depositLimit: args.config.depositLimit,
      interestRateConfig: args.config.interestRateConfig,
      operationalState: args.config.operationalState,
      borrowLimit: args.config.borrowLimit,
      riskTier: args.config.riskTier,
      assetTag: args.config.assetTag,
      pad0: [0, 0, 0, 0, 0, 0],
      totalAssetValueInitLimit: args.config.totalAssetValueInitLimit,
      oracleMaxAge: args.config.oracleMaxAge,
    })
    .accounts({
      marginfiGroup: args.marginfiGroup,
      // admin: args.admin, // implied from group
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
    .instruction();

  return ix;
};

/**
 * * admin/feePayer - must sign
 * * bank - use a fresh keypair, must sign
 */
export type AddBankWithSeedArgs = {
  marginfiGroup: PublicKey;
  feePayer: PublicKey;
  bankMint: PublicKey;
  bank: PublicKey;
  config: BankConfig;
  seed?: BN;
};

export const addBankWithSeed = (
  program: Program<Marginfi>,
  args: AddBankWithSeedArgs
) => {
  const ix = program.methods
    .lendingPoolAddBankWithSeed(
      {
        assetWeightInit: args.config.assetWeightInit,
        assetWeightMaint: args.config.assetWeightMaint,
        liabilityWeightInit: args.config.liabilityWeightInit,
        liabilityWeightMaint: args.config.liabilityWeightMain,
        depositLimit: args.config.depositLimit,
        interestRateConfig: args.config.interestRateConfig,
        operationalState: args.config.operationalState,
        borrowLimit: args.config.borrowLimit,
        riskTier: args.config.riskTier,
        assetTag: args.config.assetTag,
        pad0: [0, 0, 0, 0, 0, 0],
        totalAssetValueInitLimit: args.config.totalAssetValueInitLimit,
        oracleMaxAge: args.config.oracleMaxAge,
      },
      args.seed ?? new BN(0)
    )
    .accounts({
      marginfiGroup: args.marginfiGroup,
      // admin: args.admin, // implied from group
      feePayer: args.feePayer,
      bankMint: args.bankMint,
      // bank: args.bank, // derived from seed
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
    .instruction();

  return ix;
};

/**
 * newAdmin - (Optional) pass null to keep current admin
 * newEModeAdmin - (Optional) pass null to keep current emode admin
 * marginfiGroup's admin - must sign
 * isArena - default false
 */
export type GroupConfigureArgs = {
  newAdmin?: PublicKey | null; // optional; pass null or leave undefined to keep current admin
  newEmodeAdmin?: PublicKey | null;
  marginfiGroup: PublicKey;
  isArena?: boolean; // optional; defaults to false if not provided
};

export const groupConfigure = async (
  program: Program<Marginfi>,
  args: GroupConfigureArgs
) => {
  const isArena = args.isArena ?? false;
  let newAdmin = args.newAdmin;
  if (newAdmin == null) {
    const group = await program.account.marginfiGroup.fetch(args.marginfiGroup);
    newAdmin = group.admin;
  }
  let newEmodeAdmin = args.newEmodeAdmin;
  if (newEmodeAdmin == null) {
    const group = await program.account.marginfiGroup.fetch(args.marginfiGroup);
    newEmodeAdmin = group.emodeAdmin;
  }
  const ix = program.methods
    .marginfiGroupConfigure(newAdmin, newEmodeAdmin, isArena)
    .accounts({
      marginfiGroup: args.marginfiGroup,
      // admin: // implied from group
    })
    .instruction();

  return ix;
};

export type GroupInitializeArgs = {
  marginfiGroup: PublicKey;
  admin: PublicKey;
  isArena?: boolean; // optional; defaults to false if not provided
};

export const groupInitialize = (
  program: Program<Marginfi>,
  args: GroupInitializeArgs
) => {
  const isArena = args.isArena ?? false;
  const ix = program.methods
    .marginfiGroupInitialize(isArena)
    .accounts({
      marginfiGroup: args.marginfiGroup,
      // feeState: deriveGlobalFeeState(id),
      admin: args.admin,
      // systemProgram: SystemProgram.programId,
    })
    .instruction();

  return ix;
};

export type ConfigureBankArgs = {
  bank: PublicKey;
  bankConfigOpt: BankConfigOptWithAssetTag; // BankConfigOptRaw + assetTag
};

export const configureBank = (
  program: Program<Marginfi>,
  args: ConfigureBankArgs
) => {
  const ix = program.methods
    .lendingPoolConfigureBank(args.bankConfigOpt)
    .accounts({
      bank: args.bank,
    })
    .instruction();
  return ix;
};

export type ConfigureBankOracleArgs = {
  bank: PublicKey;
  type: number;
  oracle: PublicKey;
  /** For Pyth Pull, pass the feed. For all others, ignore */
  feed?: PublicKey;
};

export const configureBankOracle = (
  program: Program<Marginfi>,
  args: ConfigureBankOracleArgs
) => {
  const metaKey = args.feed ?? args.oracle;
  const oracleMeta: AccountMeta = {
    pubkey: metaKey,
    isSigner: false,
    isWritable: false,
  };

  const ix = program.methods
    .lendingPoolConfigureBankOracle(args.type, args.oracle)
    .accounts({
      bank: args.bank,
    })
    .remainingAccounts([oracleMeta])
    .instruction();
  return ix;
};

export type SetupEmissionsArgs = {
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

// ************* Below this line, not yet included in package ****************

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
  newAdmin?: PublicKey;
};

// TODO add test for this
export const editGlobalFeeState = (
  program: Program<Marginfi>,
  args: EditGlobalFeeStateArgs
) => {
  const ix = program.methods
    .editGlobalFeeState(
      args.newAdmin ? args.newAdmin : args.admin,
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

// TODO propagate fee state and test

export type InitStakedSettingsArgs = {
  group: PublicKey;
  feePayer: PublicKey;
  settings: StakedSettingsConfig;
};

export const initStakedSettings = (
  program: Program<Marginfi>,
  args: InitStakedSettingsArgs
) => {
  const ix = program.methods
    .initStakedSettings(args.settings)
    .accounts({
      marginfiGroup: args.group,
      // admin: args.admin, // implied from group
      feePayer: args.feePayer,
      // staked_settings: deriveStakedSettings()
      // rent = SYSVAR_RENT_PUBKEY,
      // systemProgram: SystemProgram.programId,
    })
    .instruction();

  return ix;
};

export type EditStakedSettingsArgs = {
  settingsKey: PublicKey;
  settings: StakedSettingsEdit;
};

export const editStakedSettings = (
  program: Program<Marginfi>,
  args: EditStakedSettingsArgs
) => {
  const ix = program.methods
    .editStakedSettings(args.settings)
    .accounts({
      // marginfiGroup: args.group, // implied from stakedSettings
      // admin: args.admin, // implied from group
      stakedSettings: args.settingsKey,
      // rent = SYSVAR_RENT_PUBKEY,
      // systemProgram: SystemProgram.programId,
    })
    .instruction();

  return ix;
};

/**
 * oracle - required only if settings updates the oracle key
 */
export type PropagateStakedSettingsArgs = {
  settings: PublicKey;
  bank: PublicKey;
  oracle?: PublicKey;
};

export const propagateStakedSettings = (
  program: Program<Marginfi>,
  args: PropagateStakedSettingsArgs
) => {
  const remainingAccounts = args.oracle
    ? [
        {
          pubkey: args.oracle,
          isSigner: false,
          isWritable: false,
        } as AccountMeta,
      ]
    : [];

  const ix = program.methods
    .propagateStakedSettings()
    .accounts({
      // marginfiGroup: args.group, // implied from stakedSettings
      stakedSettings: args.settings,
      bank: args.bank,
    })
    .remainingAccounts(remainingAccounts)
    .instruction();

  return ix;
};

export type AddBankPermissionlessArgs = {
  marginfiGroup: PublicKey;
  feePayer: PublicKey;
  pythOracle: PublicKey;
  stakePool: PublicKey;
  seed: BN;
};

export const addBankPermissionless = (
  program: Program<Marginfi>,
  args: AddBankPermissionlessArgs
) => {
  const [settingsKey] = deriveStakedSettings(
    program.programId,
    args.marginfiGroup
  );
  const [lstMint] = PublicKey.findProgramAddressSync(
    [Buffer.from("mint"), args.stakePool.toBuffer()],
    SINGLE_POOL_PROGRAM_ID
  );
  const [solPool] = PublicKey.findProgramAddressSync(
    [Buffer.from("stake"), args.stakePool.toBuffer()],
    SINGLE_POOL_PROGRAM_ID
  );

  // Note: oracle and lst mint/pool are also passed in meta for validation
  const oracleMeta: AccountMeta = {
    pubkey: args.pythOracle,
    isSigner: false,
    isWritable: false,
  };
  const lstMeta: AccountMeta = {
    pubkey: lstMint,
    isSigner: false,
    isWritable: false,
  };
  const solPoolMeta: AccountMeta = {
    pubkey: solPool,
    isSigner: false,
    isWritable: false,
  };

  const ix = program.methods
    .lendingPoolAddBankPermissionless(args.seed)
    .accounts({
      // marginfiGroup: args.marginfiGroup, // implied from stakedSettings
      stakedSettings: settingsKey,
      feePayer: args.feePayer,
      bankMint: lstMint,
      solPool: solPool,
      stakePool: args.stakePool,
      // bank: bankKey, // deriveBankWithSeed
      // globalFeeState: deriveGlobalFeeState(id),
      // globalFeeWallet: // implied from globalFeeState,
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
    .remainingAccounts([oracleMeta, lstMeta, solPoolMeta])
    .instruction();

  return ix;
};
