import { workspace, Program, AnchorProvider, Wallet } from "@coral-xyz/anchor";
import {
  createSimpleMint,
  echoEcosystemInfo,
  Ecosystem,
  getGenericEcosystem,
  MockUser as MockUser,
  Oracles,
  setupTestUser,
  SetupTestUserOptions,
  Validator,
} from "./utils/mocks";
import { Marginfi } from "../target/types/marginfi";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  SYSVAR_STAKE_HISTORY_PUBKEY,
  Transaction,
  VoteInit,
  VoteProgram,
} from "@solana/web3.js";
import { setupPythOracles } from "./utils/pyth_mocks";
import { BankrunProvider } from "anchor-bankrun";
import { BanksClient, ProgramTestContext, startAnchor } from "solana-bankrun";
import path from "path";
import {
  findPoolAddress,
  findPoolMintAddress,
  findPoolStakeAddress,
  findPoolStakeAuthorityAddress,
  SinglePoolProgram,
} from "@solana/spl-single-pool-classic";
import { SINGLE_POOL_PROGRAM_ID } from "./utils/types";
import { assertKeysEqual } from "./utils/genericTests";
import { assert } from "chai";
import { decodeSinglePool } from "./utils/spl-staking-utils";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { initGlobalFeeState } from "./utils/group-instructions";
import { deriveGlobalFeeState } from "./utils/pdas";

export const ecosystem: Ecosystem = getGenericEcosystem();
export let oracles: Oracles = undefined;
/** Show various information about accounts and tests */
export const verbose = true;
/** Show the raw buffer printout of various structs */
export const printBuffers = false;
/** The program owner is also the provider wallet */
export let globalProgramAdmin: MockUser = undefined;
/** Administers the mrgnlend group and/or stake holder accounts */
export let groupAdmin: MockUser = undefined;
/** Administers valiator votes and withdraws */
export let validatorAdmin: MockUser = undefined;
export const users: MockUser[] = [];
export const numUsers = 4;

export const validators: Validator[] = [];
export const numValidators = 2;
export let globalFeeWallet: PublicKey = undefined;

/** Lamports charged when creating any pool */
export const INIT_POOL_ORIGINATION_FEE = 1000;

export const PROGRAM_FEE_FIXED = 0.01;
export const PROGRAM_FEE_RATE = 0.02;

/** Group used for all happy-path tests */
export const marginfiGroup = Keypair.generate();
/** Bank for USDC */
export const bankKeypairUsdc = Keypair.generate();
/** Bank for token A */
export const bankKeypairA = Keypair.generate();
/** Bank for "WSOL", which is treated the same as SOL */
export const bankKeypairSol = Keypair.generate();

export let bankrunContext: ProgramTestContext;
export let bankRunProvider: BankrunProvider;
export let bankrunProgram: Program<Marginfi>;
export let banksClient: BanksClient;
/** keys copied into the bankrun instance */
let copyKeys: PublicKey[] = [];

export const mochaHooks = {
  beforeAll: async () => {
    // If this is false, you are in the wrong environment to run this test suite, try polyfill.
    console.log("Environment supports crypto: ", !!global.crypto?.subtle);

    const mrgnProgram = workspace.Marginfi as Program<Marginfi>;
    const provider = AnchorProvider.local();
    const wallet = provider.wallet as Wallet;

    copyKeys.push(wallet.publicKey);

    if (verbose) {
      console.log("Global Ecosystem Information ");
      echoEcosystemInfo(ecosystem, {
        skipA: false,
        skipB: false,
        skipUsdc: false,
        skipWsol: false,
      });
      console.log("");
    }

    const { ixes: wsolIxes, mint: wsolMint } = await createSimpleMint(
      provider.publicKey,
      provider.connection,
      ecosystem.wsolDecimals,
      ecosystem.wsolMint
    );
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
    const initMintsTx = new Transaction();
    initMintsTx.add(...wsolIxes);
    initMintsTx.add(...usdcIxes);
    initMintsTx.add(...aIxes);
    initMintsTx.add(...bIxes);

    await provider.sendAndConfirm(initMintsTx, [
      wsolMint,
      usdcMint,
      aMint,
      bMint,
    ]);
    copyKeys.push(
      wsolMint.publicKey,
      usdcMint.publicKey,
      aMint.publicKey,
      bMint.publicKey
    );

    let miscSetupTx = new Transaction();

    let globalFeeKeypair = Keypair.generate();
    globalFeeWallet = globalFeeKeypair.publicKey;
    // Send some sol to the global fee wallet for rent
    miscSetupTx.add(
      SystemProgram.transfer({
        fromPubkey: wallet.publicKey,
        toPubkey: globalFeeWallet,
        lamports: 10 * LAMPORTS_PER_SOL,
      })
    );

    // Init the global fee state
    miscSetupTx.add(
      await initGlobalFeeState(mrgnProgram, {
        payer: provider.publicKey,
        admin: wallet.payer.publicKey,
        wallet: globalFeeWallet,
        bankInitFlatSolFee: INIT_POOL_ORIGINATION_FEE,
        programFeeFixed: bigNumberToWrappedI80F48(PROGRAM_FEE_FIXED),
        programFeeRate: bigNumberToWrappedI80F48(PROGRAM_FEE_RATE),
      })
    );

    await provider.sendAndConfirm(miscSetupTx);
    copyKeys.push(
      globalFeeWallet,
      deriveGlobalFeeState(mrgnProgram.programId)[0]
    );

    const setupUserOptions: SetupTestUserOptions = {
      marginProgram: mrgnProgram,
      forceWallet: undefined,
      // If mints are created, typically create the ATA too, otherwise pass undefined...
      wsolMint: ecosystem.wsolMint.publicKey,
      tokenAMint: ecosystem.tokenAMint.publicKey,
      tokenBMint: ecosystem.tokenBMint.publicKey,
      usdcMint: ecosystem.usdcMint.publicKey,
    };

    groupAdmin = await setupTestUser(provider, wallet.payer, setupUserOptions);
    validatorAdmin = await setupTestUser(
      provider,
      wallet.payer,
      setupUserOptions
    );
    copyKeys.push(groupAdmin.usdcAccount);
    copyKeys.push(groupAdmin.wallet.publicKey);

    for (let i = 0; i < numUsers; i++) {
      const user = await setupTestUser(
        provider,
        wallet.payer,
        setupUserOptions
      );
      addUser(user);
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
    copyKeys.push(oracles.wsolOracle.publicKey);
    copyKeys.push(oracles.usdcOracle.publicKey);
    copyKeys.push(oracles.tokenAOracle.publicKey);

    for (let i = 0; i < numValidators; i++) {
      const validator = await createValidator(
        provider,
        validatorAdmin.wallet,
        validatorAdmin.wallet.publicKey
      );
      if (verbose) {
        console.log("Validator vote acc [" + i + "]: " + validator.voteAccount);
      }
      addValidator(validator);

      let { poolKey, poolMintKey, poolAuthority, poolStake } =
        await createSplStakePool(provider, validator);
      if (verbose) {
        console.log(" pool..... " + poolKey);
        console.log(" mint..... " + poolMintKey);
        console.log(" auth..... " + poolAuthority);
        console.log(" stake.... " + poolStake);
      }
    }

    // copyKeys.push(StakeProgram.programId);
    copyKeys.push(SYSVAR_STAKE_HISTORY_PUBKEY);

    const accountKeys = copyKeys;

    const accounts = await provider.connection.getMultipleAccountsInfo(
      accountKeys
    );
    const addedAccounts = accountKeys.map((address, index) => ({
      address,
      info: accounts[index],
    }));

    bankrunContext = await startAnchor(path.resolve(), [], addedAccounts);
    bankRunProvider = new BankrunProvider(bankrunContext);
    bankrunProgram = new Program(mrgnProgram.idl, bankRunProvider);
    for (let i = 0; i < numUsers; i++) {
      const wal = new Wallet(users[i].wallet);
      const prov = new AnchorProvider(bankRunProvider.connection, wal, {});
      users[i].mrgnBankrunProgram = new Program(mrgnProgram.idl, prov);
    }
    banksClient = bankrunContext.banksClient;

    if (verbose) {
      console.log("---End ecosystem setup---");
      console.log("");
    }
  },
};

const addValidator = (validator: Validator) => {
  validators.push(validator);
  // copyKeys.push(validator.authorizedVoter);
  // copyKeys.push(validator.authorizedWithdrawer);
  // copyKeys.push(validator.node);
  copyKeys.push(validator.voteAccount);
};

const addUser = (user: MockUser) => {
  users.push(user);
  copyKeys.push(user.tokenAAccount);
  // copyKeys.push(user.tokenBAccount);
  copyKeys.push(user.usdcAccount);
  copyKeys.push(user.wallet.publicKey);
  copyKeys.push(user.wsolAccount);
};

/**
 * Create a mock validator with given vote/withdraw authority
 * * Note: Spl Pool fields (splPool, mint, authority, stake) are initialized to pubkey default.
 * @param provider
 * @param authorizedVoter - also pays init fees
 * @param authorizedWithdrawer - also pays init fees
 * @param comission - defaults to 0
 */
export const createValidator = async (
  provider: AnchorProvider,
  authorizedVoter: Keypair,
  authorizedWithdrawer: PublicKey,
  commission: number = 0 // Commission rate from 0 to 100
) => {
  const voteAccount = Keypair.generate();
  const node = Keypair.generate();

  const tx = new Transaction().add(
    // Create the vote account
    SystemProgram.createAccount({
      fromPubkey: authorizedVoter.publicKey,
      newAccountPubkey: voteAccount.publicKey,
      lamports: await provider.connection.getMinimumBalanceForRentExemption(
        VoteProgram.space
      ),
      space: VoteProgram.space,
      programId: VoteProgram.programId,
    }),
    // Initialize the vote account
    VoteProgram.initializeAccount({
      votePubkey: voteAccount.publicKey,
      nodePubkey: node.publicKey,
      voteInit: new VoteInit(
        node.publicKey,
        authorizedVoter.publicKey,
        authorizedWithdrawer,
        commission
      ),
    })
  );

  await provider.sendAndConfirm(tx, [voteAccount, authorizedVoter, node]);

  const validator: Validator = {
    node: node.publicKey,
    authorizedVoter: authorizedVoter.publicKey,
    authorizedWithdrawer: authorizedWithdrawer,
    voteAccount: voteAccount.publicKey,
    splPool: PublicKey.default,
    splMint: PublicKey.default,
    splAuthority: PublicKey.default,
    splSolPool: PublicKey.default,
    bank: PublicKey.default,
  };

  return validator;
};

/**
 * Create a single-validator spl stake pool. Copys the pool, mint, authority, and stake accounts to
 * the copyKeys slice to be deployed to bankrun
 * @param provider
 * @param validator - mutated, adds the spl keys (pool, mint, authority, stake)
 */
export const createSplStakePool = async (
  provider: AnchorProvider,
  validator: Validator
) => {
  let tx = await SinglePoolProgram.initialize(
    // @ts-ignore // Doesn't matter
    provider.connection,
    validator.voteAccount,
    users[0].wallet.publicKey,
    true
  );

  // @ts-ignore // Doesn't matter
  await provider.sendAndConfirm(tx, [users[0].wallet]);

  // Note: import the id from @solana/spl-single-pool (the classic version doesn't have it)
  const poolKey = await findPoolAddress(
    SINGLE_POOL_PROGRAM_ID,
    validator.voteAccount
  );
  validator.splPool = poolKey;
  copyKeys.push(poolKey);

  const poolAcc = await provider.connection.getAccountInfo(poolKey);
  // Rudimentary validation that this account now exists and is owned by the single pool program
  assertKeysEqual(poolAcc.owner, SINGLE_POOL_PROGRAM_ID);
  assert.equal(poolAcc.executable, false);

  const pool = decodeSinglePool(poolAcc.data);
  assertKeysEqual(pool.voteAccountAddress, validator.voteAccount);

  const poolMintKey = await findPoolMintAddress(
    SINGLE_POOL_PROGRAM_ID,
    poolKey
  );
  validator.splMint = poolMintKey;
  copyKeys.push(poolMintKey);

  const poolStake = await findPoolStakeAddress(SINGLE_POOL_PROGRAM_ID, poolKey);
  validator.splSolPool = poolStake;
  copyKeys.push(poolStake);

  const poolAuthority = await findPoolStakeAuthorityAddress(
    SINGLE_POOL_PROGRAM_ID,
    poolKey
  );
  validator.splAuthority = poolAuthority;
  // Note: accounts that do not exist (blank PDAs) cannot be pushed to bankrun
  // copyKeys.push(poolAuthority);

  return { poolKey, poolMintKey, poolAuthority, poolStake };
};
