import { BN, Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, TransactionInstruction } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { ExponentCore } from "../fixtures/exponent_core";

export type ExponentCpiInterfaceContext = {
  altIndex: number;
  isSigner: boolean;
  isWritable: boolean;
};

export type ExponentCpiAccounts = {
  getSyState: ExponentCpiInterfaceContext[];
  depositSy: ExponentCpiInterfaceContext[];
  withdrawSy: ExponentCpiInterfaceContext[];
  claimEmission: ExponentCpiInterfaceContext[][];
  getPositionState: ExponentCpiInterfaceContext[];
};

export const emptyCpiAccounts = (): ExponentCpiAccounts => ({
  getSyState: [],
  depositSy: [],
  withdrawSy: [],
  claimEmission: [],
  getPositionState: [],
});

export type ExponentNumber = {
  mantissa: BN | number | bigint;
  exp: number;
};

export type InitializeVaultArgs = {
  payer: PublicKey;
  admin: PublicKey;
  authority: PublicKey;
  vault: PublicKey;
  mintPt: PublicKey;
  mintYt: PublicKey;
  escrowYt: PublicKey;
  escrowSy: PublicKey;
  mintSy: PublicKey;
  treasuryTokenAccount: PublicKey;
  associatedTokenProgram: PublicKey;
  syProgram: PublicKey;
  addressLookupTable: PublicKey;
  yieldPosition: PublicKey;
  metadata: PublicKey;
  tokenMetadataProgram: PublicKey;
  startTimestamp: number;
  duration: number;
  interestBpsFee: number;
  cpiAccounts: ExponentCpiAccounts;
  minOpSizeStrip: BN | number | bigint;
  minOpSizeMerge: BN | number | bigint;
  ptMetadataName: string;
  ptMetadataSymbol: string;
  ptMetadataUri: string;
  remainingAccounts?: { pubkey: PublicKey; isWritable: boolean; isSigner: boolean }[];
};

export const initializeVaultIx = async (
  program: Program<ExponentCore>,
  args: InitializeVaultArgs
): Promise<TransactionInstruction> => {
  const builder = program.methods
    .initializeVault(
      args.startTimestamp,
      args.duration,
      args.interestBpsFee,
      args.cpiAccounts,
      args.minOpSizeStrip,
      args.minOpSizeMerge,
      args.ptMetadataName,
      args.ptMetadataSymbol,
      args.ptMetadataUri
    )
    .accounts({
      payer: args.payer,
      admin: args.admin,
      authority: args.authority,
      vault: args.vault,
      mintPt: args.mintPt,
      mintYt: args.mintYt,
      escrowYt: args.escrowYt,
      escrowSy: args.escrowSy,
      mintSy: args.mintSy,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      treasuryTokenAccount: args.treasuryTokenAccount,
      associatedTokenProgram: args.associatedTokenProgram,
      syProgram: args.syProgram,
      addressLookupTable: args.addressLookupTable,
      yieldPosition: args.yieldPosition,
      metadata: args.metadata,
      tokenMetadataProgram: args.tokenMetadataProgram,
    });

  if (args.remainingAccounts && args.remainingAccounts.length > 0) {
    builder.remainingAccounts(args.remainingAccounts);
  }

  return builder.instruction();
};

export type InitializeYieldPositionArgs = {
  owner: PublicKey;
  vault: PublicKey;
  yieldPosition: PublicKey;
};

export const initializeYieldPositionIx = async (
  program: Program<ExponentCore>,
  args: InitializeYieldPositionArgs
) => {
  return program.methods
    .initializeYieldPosition()
    .accounts({
      owner: args.owner,
      vault: args.vault,
      yieldPosition: args.yieldPosition,
      systemProgram: SystemProgram.programId,
    })
    .instruction();
};

export type DepositYtArgs = {
  depositor: PublicKey;
  vault: PublicKey;
  userYieldPosition: PublicKey;
  ytSrc: PublicKey;
  escrowYt: PublicKey;
  syProgram: PublicKey;
  addressLookupTable: PublicKey;
  yieldPosition: PublicKey;
  amount: BN | number | bigint;
};

export const depositYtIx = async (
  program: Program<ExponentCore>,
  args: DepositYtArgs
) => {
  return program.methods
    .depositYt(args.amount)
    .accounts({
      depositor: args.depositor,
      vault: args.vault,
      userYieldPosition: args.userYieldPosition,
      ytSrc: args.ytSrc,
      escrowYt: args.escrowYt,
      tokenProgram: TOKEN_PROGRAM_ID,
      syProgram: args.syProgram,
      addressLookupTable: args.addressLookupTable,
      yieldPosition: args.yieldPosition,
      systemProgram: SystemProgram.programId,
      eventAuthority: PublicKey.findProgramAddressSync(
        [Buffer.from("__event_authority")],
        program.programId
      )[0],
      program: program.programId,
    })
    .instruction();
};

export type InitMarketTwoArgs = {
  payer: PublicKey;
  adminSigner: PublicKey;
  market: PublicKey;
  vault: PublicKey;
  mintSy: PublicKey;
  mintPt: PublicKey;
  mintLp: PublicKey;
  escrowPt: PublicKey;
  escrowSy: PublicKey;
  escrowLp: PublicKey;
  ptSrc: PublicKey;
  sySrc: PublicKey;
  lpDst: PublicKey;
  tokenProgram: PublicKey;
  syProgram: PublicKey;
  associatedTokenProgram: PublicKey;
  addressLookupTable: PublicKey;
  admin: PublicKey;
  tokenTreasuryFeeSy: PublicKey;
  lnFeeRateRoot: number;
  rateScalarRoot: number;
  initRateAnchor: number;
  syExchangeRate: ExponentNumber;
  ptInit: BN | number | bigint;
  syInit: BN | number | bigint;
  feeTreasurySyBps: number;
  cpiAccounts: ExponentCpiAccounts;
  seedId: number;
};

export const initMarketTwoIx = async (
  program: Program<ExponentCore>,
  args: InitMarketTwoArgs
): Promise<TransactionInstruction> => {
  return program.methods
    .initMarketTwo(
      args.lnFeeRateRoot,
      args.rateScalarRoot,
      args.initRateAnchor,
      args.syExchangeRate,
      args.ptInit,
      args.syInit,
      args.feeTreasurySyBps,
      args.cpiAccounts,
      args.seedId
    )
    .accounts({
      payer: args.payer,
      adminSigner: args.adminSigner,
      market: args.market,
      vault: args.vault,
      mintSy: args.mintSy,
      mintPt: args.mintPt,
      mintLp: args.mintLp,
      escrowPt: args.escrowPt,
      escrowSy: args.escrowSy,
      escrowLp: args.escrowLp,
      ptSrc: args.ptSrc,
      sySrc: args.sySrc,
      lpDst: args.lpDst,
      tokenProgram: args.tokenProgram,
      systemProgram: SystemProgram.programId,
      syProgram: args.syProgram,
      associatedTokenProgram: args.associatedTokenProgram,
      addressLookupTable: args.addressLookupTable,
      admin: args.admin,
      tokenTreasuryFeeSy: args.tokenTreasuryFeeSy,
    })
    .instruction();
};

export type TradePtArgs = {
  trader: PublicKey;
  market: PublicKey;
  tokenSyTrader: PublicKey;
  tokenPtTrader: PublicKey;
  tokenSyEscrow: PublicKey;
  tokenPtEscrow: PublicKey;
  addressLookupTable: PublicKey;
  syProgram: PublicKey;
  tokenFeeTreasurySy: PublicKey;
  netTraderPt: BN | number | bigint;
  syConstraint: BN | number | bigint;
};

export const tradePtIx = async (
  program: Program<ExponentCore>,
  args: TradePtArgs
): Promise<TransactionInstruction> => {
  const [eventAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from("__event_authority")],
    program.programId
  );

  return program.methods
    .tradePt(args.netTraderPt, args.syConstraint)
    .accounts({
      trader: args.trader,
      market: args.market,
      tokenSyTrader: args.tokenSyTrader,
      tokenPtTrader: args.tokenPtTrader,
      tokenSyEscrow: args.tokenSyEscrow,
      tokenPtEscrow: args.tokenPtEscrow,
      addressLookupTable: args.addressLookupTable,
      tokenProgram: TOKEN_PROGRAM_ID,
      syProgram: args.syProgram,
      tokenFeeTreasurySy: args.tokenFeeTreasurySy,
      eventAuthority,
      program: program.programId,
    })
    .instruction();
};
