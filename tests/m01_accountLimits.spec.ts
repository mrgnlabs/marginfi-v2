import { BN, Wallet, getProvider, AnchorProvider } from "@coral-xyz/anchor";
import {
  AccountMeta,
  ComputeBudgetProgram,
  Keypair,
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import {
  groupAdmin,
  bankrunContext,
  banksClient,
  bankrunProgram,
  verbose,
  ecosystem,
  oracles,
  PYTH_ORACLE_SAMPLE,
  PYTH_ORACLE_FEED_SAMPLE,
  users,
} from "./rootHooks";
import {
  assertBankrunTxFailed,
  assertI80F48Approx,
  assertKeysEqual,
} from "./utils/genericTests";
import {
  addBankWithSeed,
  configureBank,
  groupConfigure,
  groupInitialize,
} from "./utils/group-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { assert } from "chai";
import {
  defaultBankConfig,
  ASSET_TAG_DEFAULT,
  ORACLE_SETUP_PYTH_PUSH,
  HEALTH_CACHE_ENGINE_OK,
  HEALTH_CACHE_HEALTHY,
  CONF_INTERVAL_MULTIPLE,
  HEALTH_CACHE_ORACLE_OK,
  I80F48_ZERO,
  ORACLE_SETUP_PYTH_LEGACY,
  defaultBankConfigOptRaw,
} from "./utils/types";
import { deriveBankWithSeed } from "./utils/pdas";
import { createMintToInstruction, getAccount } from "@solana/spl-token";
import { USER_ACCOUNT } from "./utils/mocks";
import {
  accountInit,
  borrowIx,
  composeRemainingAccounts,
  depositIx,
  healthPulse,
  liquidateIx,
} from "./utils/user-instructions";
import {
  bigNumberToWrappedI80F48,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import { bytesToF64, dumpAccBalances, dumpBankrunLogs } from "./utils/tools";

/** Banks in this test use a "random" seed so their key is non-deterministic. */
let startingSeed: number = Math.floor(Math.random() * 16000);

/** This is the program-enforced maximum enforced number of balances per account. */
const MAX_BALANCES = 16;
const USER_ACCOUNT_THROWAWAY = "throwaway_account";

let banks: PublicKey[] = [];

const throwawayGroup = Keypair.generate();
describe("Pyth pull oracles in localnet", () => {
  it("(admin) Init group - happy path", async () => {
    let tx = new Transaction();

    tx.add(
      await groupInitialize(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: throwawayGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, throwawayGroup);
    await banksClient.processTransaction(tx);

    let group = await bankrunProgram.account.marginfiGroup.fetch(
      throwawayGroup.publicKey
    );
    assertKeysEqual(group.admin, groupAdmin.wallet.publicKey);
    if (verbose) {
      console.log("*init group: " + throwawayGroup.publicKey);
      console.log(" group admin: " + group.admin);
    }
  });

  it("(admin) Set the emode admin - happy path", async () => {
    let tx = new Transaction().add(
      await groupConfigure(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: throwawayGroup.publicKey,
        newEmodeAdmin: groupAdmin.wallet.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    let group = await bankrunProgram.account.marginfiGroup.fetch(
      throwawayGroup.publicKey
    );
    assertKeysEqual(group.emodeAdmin, groupAdmin.wallet.publicKey);
  });

  it("(admin) Add 16 banks (asset doesn't matter)", async () => {
    const promises: Promise<any>[] = [];
    for (let i = 0; i < MAX_BALANCES; i++) {
      const seed = startingSeed + i;
      promises.push(
        addBankTest({
          bankMint: ecosystem.lstAlphaMint.publicKey,
          oracle: oracles.pythPullLstOracleFeed.publicKey,
          oracleMeta: {
            pubkey: oracles.pythPullLst.publicKey, // NOTE: Price V2 update
            isSigner: false,
            isWritable: false,
          },
          oracleSetup: "PULL",
          feedOracle: oracles.pythPullLstOracleFeed.publicKey,
          seed: new BN(seed),
          verboseMessage: `*init LST #${seed}:`,
        })
      );
      const [bank] = deriveBankWithSeed(
        bankrunProgram.programId,
        throwawayGroup.publicKey,
        ecosystem.lstAlphaMint.publicKey,
        new BN(seed)
      );
      banks.push(bank);
    }
    await Promise.all(promises);
  });

  it("(Fund users/admin USDC/WSOL/LST token accounts", async () => {
    const provider = getProvider() as AnchorProvider;
    const wallet = provider.wallet as Wallet;
    for (let i = 0; i < users.length; i++) {
      let tx = new Transaction();
      tx.add(
        createMintToInstruction(
          ecosystem.lstAlphaMint.publicKey,
          users[i].lstAlphaAccount,
          wallet.publicKey,
          10000 * 10 ** ecosystem.lstAlphaDecimals
        )
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(wallet.payer);
      await banksClient.processTransaction(tx);
    }

    // Seed the admin with funds as well
    let tx = new Transaction();
    tx.add(
      createMintToInstruction(
        ecosystem.lstAlphaMint.publicKey,
        groupAdmin.lstAlphaAccount,
        wallet.publicKey,
        10000 * 10 ** ecosystem.lstAlphaDecimals
      )
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(wallet.payer);
    await banksClient.processTransaction(tx);
  });

  it("Initialize user accounts (if needed)", async () => {
    for (let i = 0; i < users.length; i++) {
      const userAccKeypair = Keypair.generate();
      const userAccount = userAccKeypair.publicKey;
      if (users[i].accounts.get(USER_ACCOUNT_THROWAWAY)) {
        if (verbose) {
          console.log("Skipped creating user " + i);
        }
        continue;
      } else {
        if (verbose) {
          console.log("user [" + i + "]: " + userAccount);
        }
        users[i].accounts.set(USER_ACCOUNT_THROWAWAY, userAccount);
      }

      let userinitTx: Transaction = new Transaction();
      userinitTx.add(
        await accountInit(users[i].mrgnBankrunProgram, {
          marginfiGroup: throwawayGroup.publicKey,
          marginfiAccount: userAccount,
          authority: users[i].wallet.publicKey,
          feePayer: users[i].wallet.publicKey,
        })
      );
      userinitTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      userinitTx.sign(users[i].wallet, userAccKeypair);
      await banksClient.processTransaction(userinitTx);
    }

    if (groupAdmin.accounts.get(USER_ACCOUNT_THROWAWAY)) {
      console.log("Skipped creating admin account");
      return;
    }
    const userAccKeypair = Keypair.generate();
    const userAccount = userAccKeypair.publicKey;
    groupAdmin.accounts.set(USER_ACCOUNT_THROWAWAY, userAccount);

    let userinitTx: Transaction = new Transaction();
    userinitTx.add(
      await accountInit(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: throwawayGroup.publicKey,
        marginfiAccount: userAccount,
        authority: groupAdmin.wallet.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
      })
    );
    userinitTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    userinitTx.sign(groupAdmin.wallet, userAccKeypair);
    await banksClient.processTransaction(userinitTx);
  });

  it("(admin) Seeds liquidity in all banks - validates 16 deposits is possible", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const amount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
    // Note: This is about the max per TX without using LUTs.
    const depositsPerTx = 5;

    for (let i = 0; i < banks.length; i += depositsPerTx) {
      const chunk = banks.slice(i, i + depositsPerTx);
      const tx = new Transaction();
      for (const bank of chunk) {
        tx.add(
          await depositIx(user.mrgnBankrunProgram, {
            marginfiAccount: userAccount,
            bank,
            tokenAccount: user.lstAlphaAccount,
            amount,
            depositUpToLimit: false,
          })
        );
      }
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(user.wallet);
      await banksClient.tryProcessTransaction(tx);
    }
  });

  // TODO see if this still works with emode
  it("(user 0) Borrows 15 positions against 1 - validates max borrows possible", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const depositAmount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
    const borrowAmount = new BN(1 * 10 ** ecosystem.lstAlphaDecimals);
    let oomAt = MAX_BALANCES;

    const tx = new Transaction();
    tx.add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[0],
        tokenAccount: user.lstAlphaAccount,
        amount: depositAmount,
        depositUpToLimit: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.tryProcessTransaction(tx);

    for (let i = 1; i < banks.length; i += 1) {
      const remainingAccounts: PublicKey[][] = [];
      remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
      for (let k = 1; k <= i; k++) {
        remainingAccounts.push([banks[k], oracles.pythPullLst.publicKey]);
      }

      const tx = new Transaction();
      tx.add(
        ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
        ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }),
        await borrowIx(user.mrgnBankrunProgram, {
          marginfiAccount: userAccount,
          bank: banks[i],
          tokenAccount: user.lstAlphaAccount,
          remaining: composeRemainingAccounts(remainingAccounts),
          amount: borrowAmount,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(user.wallet);
      let result = await banksClient.tryProcessTransaction(tx);
      console.log("***********" + i + " ***********");
      //dumpBankrunLogs(result);

      // Throws if the error is not OOM.
      if (result.result) {
        const logs = result.meta.logMessages;
        const isOOM = logs.some((msg) =>
          msg.toLowerCase().includes("memory allocation failed, out of memory")
        );

        if (isOOM) {
          oomAt = i + 1;
          console.warn(`⚠️ \t OOM during borrow on bank ${i}: \n`, logs);
          console.log("MAXIMUM ACCOUNTS BEFORE MEMORY FAILURE: " + oomAt);
          assert.ok(false);
        } else {
          // anything other than OOM should blow up the test
          throw new Error(
            `Unexpected borrowIx failure on bank ${banks[i].toBase58()}: ` +
              logs.join("\n")
          );
        }
      }
    }
    console.log("No memory failures detected on " + MAX_BALANCES + " accounts");
  });

  it("(admin) vastly increase last bank liability ratio to make user 0 unhealthy", async () => {
    let config = defaultBankConfigOptRaw();
    config.liabilityWeightInit = bigNumberToWrappedI80F48(210); // 21000%
    config.liabilityWeightMaint = bigNumberToWrappedI80F48(200); // 20000%

    let tx = new Transaction().add(
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: banks[MAX_BALANCES - 1],
        bankConfigOpt: config,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
  });

  it("(user 1) Liquidates user 0", async () => {
    const liquidatee = users[0];
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_THROWAWAY);
    const liquidator = users[1];
    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT_THROWAWAY);
    const depositAmount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
    const liquidateAmount = new BN(0.01 * 10 ** ecosystem.lstAlphaDecimals);

    const remainingAccounts: PublicKey[][] = [];
    for (let i = 0; i < MAX_BALANCES; i++) {
      remainingAccounts.push([banks[i], oracles.pythPullLst.publicKey]);
      console.log("bank: " + banks[i]);
    }

    // Deposit some funds to operate as a liquidator...
    let tx = new Transaction();
    tx.add(
      await depositIx(liquidator.mrgnBankrunProgram, {
        marginfiAccount: liquidatorAccount,
        bank: banks[0],
        tokenAccount: liquidator.lstAlphaAccount,
        amount: depositAmount,
        depositUpToLimit: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(liquidator.wallet);
    await banksClient.tryProcessTransaction(tx);

    const liquidateeAcc = await bankrunProgram.account.marginfiAccount.fetch(
      liquidateeAccount
    );
    dumpAccBalances(liquidateeAcc);
    const liquidatorAcc = await bankrunProgram.account.marginfiAccount.fetch(
      liquidatorAccount
    );
    dumpAccBalances(liquidatorAcc);

    tx = new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
      await liquidateIx(liquidator.mrgnBankrunProgram, {
        assetBankKey: banks[0],
        liabilityBankKey: banks[MAX_BALANCES - 1],
        liquidatorMarginfiAccount: liquidatorAccount,
        liquidateeMarginfiAccount: liquidateeAccount,
        remaining: [
          oracles.pythPullLst.publicKey, // asset oracle
          oracles.pythPullLst.publicKey, // liab oracle

          ...composeRemainingAccounts([
            // liquidator accounts
            [banks[0], oracles.pythPullLst.publicKey],
            [banks[MAX_BALANCES - 1], oracles.pythPullLst.publicKey],
          ]),

          ...composeRemainingAccounts(
            // liquidatee accounts
            remainingAccounts
          ),
        ],
        amount: liquidateAmount,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(liquidator.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    //dumpBankrunLogs(result);

    // Throws if the error is not OOM.
    if (result.result) {
      const logs = result.meta.logMessages;
      const isOOM = logs.some((msg) =>
        msg.toLowerCase().includes("memory allocation failed, out of memory")
      );

      if (isOOM) {
        console.warn(`⚠️ \t OOM during liquidate: \n`, logs);
        assert.ok(false);
      } else {
        // anything other than OOM should blow up the test
        throw new Error(`Unexpected liquidate failure}: ` + logs.join("\n"));
      }
    }
  });

  // TODO try these with switchboard oracles.

  async function addBankTest(options: {
    assetTag?: number;
    bankMint: PublicKey;
    oracle: PublicKey;
    oracleMeta: AccountMeta;
    // For banks (like LST) that need a different oracle setup (pull vs legacy)
    oracleSetup?: "LEGACY" | "PULL";
    // Optional feed oracle in case the instruction requires it (i.e. for pull)
    feedOracle?: PublicKey;
    // Function to adjust the seed (for example, seed.addn(1))
    seed: BN;
    verboseMessage: string;
  }) {
    const {
      assetTag,
      bankMint,
      oracle,
      oracleMeta,
      oracleSetup = "LEGACY",
      feedOracle,
      seed,
      verboseMessage,
    } = options;

    const config = defaultBankConfig();
    config.assetWeightInit = bigNumberToWrappedI80F48(0.5);
    config.assetWeightMaint = bigNumberToWrappedI80F48(0.6);

    // The default limit is somewhat small for SOL/LST with 9 decimals, so we bump it here.
    config.depositLimit = new BN(100_000_000_000_000);
    config.borrowLimit = new BN(100_000_000_000_000);
    // We don't want origination fees messing with debt here
    config.interestRateConfig.protocolOriginationFee = I80F48_ZERO;
    if (assetTag) {
      config.assetTag = assetTag;
    }

    // Calculate bank key using the (optionally modified) seed
    const [bankKey] = deriveBankWithSeed(
      bankrunProgram.programId,
      throwawayGroup.publicKey,
      bankMint,
      seed
    );

    const setupType =
      oracleSetup === "PULL"
        ? ORACLE_SETUP_PYTH_PUSH
        : ORACLE_SETUP_PYTH_LEGACY;
    const targetOracle = feedOracle ?? oracle;
    const config_ix = await groupAdmin.mrgnProgram.methods
      .lendingPoolConfigureBankOracle(setupType, targetOracle)
      .accountsPartial({
        group: throwawayGroup.publicKey,
        bank: bankKey,
        admin: groupAdmin.wallet.publicKey,
      })
      .remainingAccounts([oracleMeta])
      .instruction();

    const addBankIx = await addBankWithSeed(groupAdmin.mrgnBankrunProgram, {
      marginfiGroup: throwawayGroup.publicKey,
      feePayer: groupAdmin.wallet.publicKey,
      bankMint: bankMint,
      bank: bankKey,
      config: config,
      seed,
    });

    const tx = new Transaction();
    tx.add(addBankIx, config_ix);
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    if (verbose) {
      console.log(`${verboseMessage} ${bankKey}`);
    }
  }
});
