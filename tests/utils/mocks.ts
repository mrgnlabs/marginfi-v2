import { AnchorProvider, Program, Wallet } from "@coral-xyz/anchor";
import {
  createAssociatedTokenAccountInstruction,
  createInitializeMintInstruction,
  getAssociatedTokenAddressSync,
  MintLayout,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import type { Connection, TransactionInstruction } from "@solana/web3.js";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { Marginfi } from "../../target/types/marginfi";

export type Ecosystem = {
  /** A generic wsol mint with 9 decimals (same as native) */
  wsolMint: Keypair;
  /** A generic spl token mint */
  tokenAMint: Keypair;
  /** A generic spl token mint */
  tokenBMint: Keypair;
  /** A generic spl token mint like USDC (6 decimals) */
  usdcMint: Keypair;
  /** 9 */
  wsolDecimals: number;
  /** Decimals for token A (default 8) */
  tokenADecimals: number;
  /** Decimals for token B (default 6)*/
  tokenBDecimals: number;
  /** 6 */
  usdcDecimals: number;
};

/**
 * Random keypairs for all mints.
 *
 * 6 Decimals for usdc. 9 decimals for sol. 8 decimals to token A, 6 for token B
 * @returns
 */
export const getGenericEcosystem = () => {
  const ecosystem: Ecosystem = {
    wsolMint: Keypair.generate(),
    tokenAMint: Keypair.generate(),
    tokenBMint: Keypair.generate(),
    usdcMint: Keypair.generate(),
    wsolDecimals: 9,
    tokenADecimals: 8,
    tokenBDecimals: 6,
    usdcDecimals: 6,
  };
  return ecosystem;
};

/**
 * Print ecosystem info to console
 * @param ecosystem
 */
export const echoEcosystemInfo = (
  ecosystem: Ecosystem,
  { skipWsol = false, skipUsdc = false, skipA = false, skipB = false }
) => {
  if (!skipWsol) {
    console.log("wsol mint:........... " + ecosystem.wsolMint.publicKey);
    console.log("  wsol decimals...... " + ecosystem.wsolDecimals);
  }
  if (!skipUsdc) {
    console.log("usdc mint:........... " + ecosystem.usdcMint.publicKey);
    console.log("  usdc decimals:..... " + ecosystem.usdcDecimals);
  }
  if (!skipA) {
    console.log("token a mint:........ " + ecosystem.tokenAMint.publicKey);
    console.log("  token a decimals:.. " + ecosystem.tokenADecimals);
  }
  if (!skipB) {
    console.log("token b mint:........ " + ecosystem.tokenBMint.publicKey);
    console.log("  token b decimals:.. " + ecosystem.tokenBDecimals);
  }
};

/**
 *  A typical user, with a wallet, ATAs for mock tokens, and a program to sign/send txes with.
 */
export type mockUser = {
  wallet: Keypair;
  /** Users's ATA for wsol*/
  wsolAccount: PublicKey;
  /** Users's ATA for token A */
  tokenAAccount: PublicKey;
  /** Users's ATA for token B */
  tokenBAccount: PublicKey;
  /** Users's ATA for USDC */
  usdcAccount: PublicKey;
  /** A program that uses the user's wallet */
  userMarginProgram: Program<Marginfi> | undefined;
};

/**
 * Options to skip various parts of mock user setup
 */
export interface SetupTestUserOptions {
  marginProgram: Program<Marginfi>;
  /** Force the mock user to use this keypair */
  forceWallet: Keypair;
  wsolMint: PublicKey;
  tokenAMint: PublicKey;
  tokenBMint: PublicKey;
  usdcMint: PublicKey;
}

/**
 * Creates and funds a user by transfering some SOL from a given wallet.
 *
 * Opens ATA for the user on all ecosystem mints
 *
 * Initializes a mock program to sign transactions as the user
 * @param provider
 * @param wallet - provider wallet, pays init and tx fees
 * @param options - skip parts of setup or force a keypair as the wallet
 * @returns
 */
export const setupTestUser = async (
  provider: AnchorProvider,
  wallet: Keypair,
  options?: SetupTestUserOptions
) => {
  // Creates a user wallet with some SOL in it to pay tx fees
  const userWalletKeypair = options.forceWallet || Keypair.generate();
  const userWallet = userWalletKeypair.publicKey;
  const tx: Transaction = new Transaction();
  tx.add(
    SystemProgram.transfer({
      fromPubkey: wallet.publicKey,
      toPubkey: userWallet,
      lamports: 1000 * LAMPORTS_PER_SOL,
    })
  );

  let wsolAccount: PublicKey = PublicKey.default;
  if (options.wsolMint) {
    wsolAccount = getAssociatedTokenAddressSync(options.wsolMint, userWallet);
    tx.add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        wsolAccount,
        userWallet,
        options.wsolMint
      )
    );
  }

  let usdcAccount: PublicKey = PublicKey.default;
  if (options.usdcMint) {
    usdcAccount = getAssociatedTokenAddressSync(options.usdcMint, userWallet);
    tx.add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        usdcAccount,
        userWallet,
        options.usdcMint
      )
    );
  }

  let tokenAAccount: PublicKey = PublicKey.default;
  if (options.tokenAMint) {
    tokenAAccount = getAssociatedTokenAddressSync(
      options.tokenAMint,
      userWallet
    );
    tx.add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        tokenAAccount,
        userWallet,
        options.tokenAMint
      )
    );
  }

  let tokenBAccount: PublicKey = PublicKey.default;
  if (options.tokenBMint) {
    tokenBAccount = getAssociatedTokenAddressSync(
      options.tokenBMint,
      userWallet
    );
    tx.add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        tokenBAccount,
        userWallet,
        options.tokenBMint
      )
    );
  }

  await provider.sendAndConfirm(tx, [wallet]);

  const user: mockUser = {
    wallet: userWalletKeypair,
    wsolAccount: wsolAccount,
    tokenAAccount: tokenAAccount,
    tokenBAccount: tokenBAccount,
    usdcAccount: usdcAccount,

    userMarginProgram: options.marginProgram
      ? getUserMarginfiProgram(options.marginProgram, userWalletKeypair)
      : undefined,
  };
  return user;
};

/**
 * Generates a mock program that can sign transactions as the user's wallet
 * @param program
 * @param userWallet
 * @returns
 */
export const getUserMarginfiProgram = (
  program: Program<Marginfi>,
  userWallet: Keypair | Wallet
) => {
  const wallet =
    userWallet instanceof Keypair ? new Wallet(userWallet) : userWallet;
  const provider = new AnchorProvider(program.provider.connection, wallet, {});
  const userProgram = new Program(program.idl, provider);
  return userProgram;
};

/**
 * Ixes to create a mint, the payer gains the Mint Tokens/Freeze authority
 * @param payer - pays account init fees, must sign, gains mint/freeze authority
 * @param provider
 * @param decimals
 * @param mintKeypair - (optional) generates random keypair if not provided, must sign
 * @param lamps - (optional) lamports to pay for created acc, fetches minimum for Mint exemption if
 * not provided
 * @returns ixes, and keypair of new mint
 */
export const createSimpleMint = async (
  payer: PublicKey,
  connection: Connection,
  decimals: number,
  mintKeypair?: Keypair,
  lamps?: number
) => {
  const mint = mintKeypair ? mintKeypair : Keypair.generate();
  const ixes: TransactionInstruction[] = [];
  const lamports = lamps
    ? lamps
    : await connection.getMinimumBalanceForRentExemption(MintLayout.span);
  ixes.push(
    SystemProgram.createAccount({
      fromPubkey: payer,
      newAccountPubkey: mint.publicKey,
      space: MintLayout.span,
      lamports: lamports,
      programId: TOKEN_PROGRAM_ID,
    })
  );
  ixes.push(
    createInitializeMintInstruction(
      mint.publicKey,
      decimals,
      payer,
      payer,
      TOKEN_PROGRAM_ID
    )
  );

  return { ixes, mint };
};
