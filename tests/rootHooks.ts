import { workspace, Program, AnchorProvider, Wallet } from "@coral-xyz/anchor";
import {
  createSimpleMint,
  echoEcosystemInfo,
  Ecosystem,
  getGenericEcosystem,
  mockUser as MockUser,
  Oracles,
  setupTestUser,
  SetupTestUserOptions,
  Validator,
} from "./utils/mocks";
import { Marginfi } from "../target/types/marginfi";
import {
  Keypair,
  PublicKey,
  StakeProgram,
  SystemProgram,
  SYSVAR_EPOCH_SCHEDULE_PUBKEY,
  SYSVAR_STAKE_HISTORY_PUBKEY,
  Transaction,
  VoteInit,
  VoteProgram,
} from "@solana/web3.js";
import { setupPythOracles } from "./utils/pyth_mocks";
import { StakingCollatizer } from "../target/types/staking_collatizer";
import { BankrunProvider } from "anchor-bankrun";
import { BanksClient, ProgramTestContext, startAnchor } from "solana-bankrun";
import path from "path";

export const ecosystem: Ecosystem = getGenericEcosystem();
export let oracles: Oracles = undefined;
export const verbose = true;
/** The program owner is also the provider wallet */
export let globalProgramAdmin: MockUser = undefined;
/** Administers the mrgnlend group and/or stake holder accounts */
export let groupAdmin: MockUser = undefined;
/** Administers valiator votes and withdraws */
export let validatorAdmin: MockUser = undefined;
export const users: MockUser[] = [];
export const numUsers = 2;

export const validators: Validator[] = [];
export const numValidators = 2;

/** Group used for all happy-path tests */
export const marginfiGroup = Keypair.generate();
/** Bank for USDC */
export const bankKeypairUsdc = Keypair.generate();
/** Bank for token A */
export const bankKeypairA = Keypair.generate();

export let bankrunContext: ProgramTestContext;
export let bankRunProvider: BankrunProvider;
export let bankrunProgram: Program<StakingCollatizer>;
export let banksClient: BanksClient;
/** keys copied into the bankrun instance */
let copyKeys: PublicKey[] = [];

export const mochaHooks = {
  beforeAll: async () => {
    const mrgnProgram = workspace.Marginfi as Program<Marginfi>;
    const collatProgram =
      workspace.StakingCollatizer as Program<StakingCollatizer>;
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

    await provider.sendAndConfirm(tx, [usdcMint, aMint, bMint]);
    copyKeys.push(usdcMint.publicKey, aMint.publicKey, bMint.publicKey);

    const setupUserOptions: SetupTestUserOptions = {
      marginProgram: mrgnProgram,
      collatizerProgram: collatProgram,
      forceWallet: undefined,
      // If mints are created, typically create the ATA too, otherwise pass undefined...
      wsolMint: undefined,
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
    }

    copyKeys.push(StakeProgram.programId);
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
    bankrunProgram = new Program(collatProgram.idl, bankRunProvider);
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
  // copyKeys.push(user.tokenAAccount);
  // copyKeys.push(user.tokenBAccount);
  copyKeys.push(user.usdcAccount);
  copyKeys.push(user.wallet.publicKey);
  // copyKeys.push(user.wsolAccount);
};

/**
 * Create a mock validator with given vote/withdraw authority
 *
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
  };

  return validator;
};
