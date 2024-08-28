import { workspace, Program, AnchorProvider, Wallet } from "@coral-xyz/anchor";
import {
  createSimpleMint,
  echoEcosystemInfo,
  Ecosystem,
  getGenericEcosystem,
  mockUser,
  Oracles,
  setupTestUser,
  SetupTestUserOptions,
} from "./utils/mocks";
import { Marginfi } from "../target/types/marginfi";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { setupPythOracles } from "./utils/pyth_mocks";
import { initGlobalFeeState } from "./utils/instructions";

export const ecosystem: Ecosystem = getGenericEcosystem();
export let oracles: Oracles = undefined;
export const verbose = true;
/** The program owner is also the provider wallet */
export let globalProgramAdmin: mockUser = undefined;
export let groupAdmin: mockUser = undefined;
export let globalFeeWallet: PublicKey = undefined;
export const users: mockUser[] = [];
export const numUsers = 2;

/** Lamports charged when creating any pool */
export const INIT_POOL_ORIGINATION_FEE = 1000;

/** Group used for all happy-path tests */
export const marginfiGroup = Keypair.generate();
/** Bank for USDC */
export const bankKeypairUsdc = Keypair.generate();
/** Bank for token A */
export const bankKeypairA = Keypair.generate();

export const mochaHooks = {
  beforeAll: async () => {
    const program = workspace.Marginfi as Program<Marginfi>;
    const provider = AnchorProvider.local();
    const wallet = provider.wallet as Wallet;

    if (verbose) {
      console.log("Global Ecosystem Information ");
      echoEcosystemInfo(ecosystem, {
        skipA: false,
        skipB: false,
        skipUsdc: false,
        skipWsol: true,
      });
      console.log("");
    }

    const { ixes: usdcIxes, mint: usdcMint } = await createSimpleMint(
      provider.publicKey,
      provider.connection,
      ecosystem.usdcDecimals,
      ecosystem.usdcMint
    );

    const { ixes: aIxes, mint: aMint } = await createSimpleMint(
      provider.publicKey,
      provider.connection,
      ecosystem.tokenADecimals,
      ecosystem.tokenAMint
    );
    const { ixes: bIxes, mint: bMint } = await createSimpleMint(
      provider.publicKey,
      provider.connection,
      ecosystem.tokenBDecimals,
      ecosystem.tokenBMint
    );
    const tx = new Transaction();
    tx.add(...usdcIxes);
    tx.add(...aIxes);
    tx.add(...bIxes);

    let globalFeeKeypair = Keypair.generate();
    globalFeeWallet = globalFeeKeypair.publicKey;
    // Send some sol to the global fee wallet for rent
    tx.add(
      SystemProgram.transfer({
        fromPubkey: wallet.publicKey,
        toPubkey: globalFeeWallet,
        lamports: 10 * LAMPORTS_PER_SOL,
      })
    );

    // Init the global fee state
    tx.add(
      await initGlobalFeeState(program, {
        payer: provider.publicKey,
        admin: wallet.payer.publicKey,
        wallet: globalFeeWallet,
        bankInitFlatSolFee: INIT_POOL_ORIGINATION_FEE,
      })
    );

    await provider.sendAndConfirm(tx, [usdcMint, aMint, bMint]);

    const setupUserOptions: SetupTestUserOptions = {
      marginProgram: program,
      forceWallet: undefined,
      // If mints are created, typically create the ATA too, otherwise pass undefined...
      wsolMint: undefined,
      tokenAMint: ecosystem.tokenAMint.publicKey,
      tokenBMint: ecosystem.tokenBMint.publicKey,
      usdcMint: ecosystem.usdcMint.publicKey,
    };

    groupAdmin = await setupTestUser(provider, wallet.payer, setupUserOptions);

    for (let i = 0; i < numUsers; i++) {
      const user = await setupTestUser(
        provider,
        wallet.payer,
        setupUserOptions
      );
      users.push(user);
    }

    // Global admin uses the payer wallet...
    setupUserOptions.forceWallet = wallet.payer;
    globalProgramAdmin = await setupTestUser(
      provider,
      wallet.payer,
      setupUserOptions
    );

    oracles = await setupPythOracles(
      wallet,
      150,
      ecosystem.wsolDecimals,
      1,
      ecosystem.usdcDecimals,
      10,
      ecosystem.tokenADecimals,
      20,
      ecosystem.tokenBDecimals,
      verbose
    );
  },
};
