import { BN } from "@coral-xyz/anchor";
import {
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
  ecosystem,
  oracles,
  users,
  klendBankrunProgram,
  TOKEN_A_RESERVE,
  kaminoAccounts,
  MARKET,
  globalProgramAdmin,
  FARMS_PROGRAM_ID,
  A_FARM_STATE,
  farmAccounts,
} from "./rootHooks";
import {
  configBankEmode,
  configureBank,
  groupConfigure,
} from "./utils/group-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { assert } from "chai";
import { defaultBankConfigOptRaw, newEmodeEntry } from "./utils/types";
import {
  borrowIx,
  composeRemainingAccounts,
  depositIx,
  liquidateIx,
} from "./utils/user-instructions";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import {
  dumpAccBalances,
  dumpBankrunLogs,
  processBankrunTransaction,
} from "./utils/tools";
import { genericKaminoMultiBankTestSetup } from "./genericSetups";
import { makeKaminoDepositIx } from "./utils/kamino-instructions";
import {
  simpleRefreshObligation,
  simpleRefreshReserve,
} from "./utils/kamino-utils";
import {
  deriveBaseObligation,
  deriveLiquidityVaultAuthority,
  deriveUserState,
} from "./utils/pdas";
import { refreshPullOracles } from "./utils/pyth-pull-mocks";
import { refreshPullOraclesBankrun } from "./utils/bankrun-oracles";

const startingSeed: number = 499;
const groupBuff = Buffer.from("MARGINFI_GROUP_SEED_123400000K14");

/** This is the program-enforced maximum enforced number of balances per account. */
const MAX_BALANCES = 16;
const USER_ACCOUNT_THROWAWAY = "throwaway_account_k14";

let kaminoBanks: PublicKey[] = [];
let regularBanks: PublicKey[] = [];
let throwawayGroup: Keypair;

describe("k14: Limits on number of accounts, with Kamino and emode", () => {
  it("Refresh oracles", async () => {
    // Update all oracles using the aggregator function to avoid "Account in use" errors
    // This test creates many banks in parallel that all reference tokenAPull oracle
    await refreshPullOraclesBankrun(oracles, bankrunContext, banksClient);
  });

  it("init group, init banks, and fund banks", async () => {
    const result = await genericKaminoMultiBankTestSetup(
      MAX_BALANCES,
      USER_ACCOUNT_THROWAWAY,
      groupBuff,
      startingSeed
    );
    kaminoBanks = result.kaminoBanks;
    regularBanks = result.regularBanks;
    throwawayGroup = result.throwawayGroup;
  });

  it("(admin) set the group admin as the emode admin too", async () => {
    const tx = new Transaction().add(
      await groupConfigure(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: throwawayGroup.publicKey,
        newAdmin: groupAdmin.wallet.publicKey,
        newEmodeAdmin: groupAdmin.wallet.publicKey,
        isArena: false,
      })
    );
    await processBankrunTransaction(bankrunContext, tx, [groupAdmin.wallet]);
  });

  it("(emode admin) Configures bank emodes - happy path", async () => {
    for (let bankIndex = 0; bankIndex < kaminoBanks.length; bankIndex++) {
      const kaminoBank = kaminoBanks[bankIndex];
      const regularBank = regularBanks[bankIndex];

      // pick 10 unique, random tags from 0..MAX_BALANCES-1 (excluding the last bank)
      const entryTags = [...Array(MAX_BALANCES - 1).keys()] // [0,1,2,…,14]
        .sort(() => Math.random() - 0.5) // shuffle
        .slice(0, 10); // take first 10

      // build the 10 entries for this bank with random tags and values
      const entries = entryTags.map((entryTag) =>
        newEmodeEntry(
          entryTag,
          1, // applies to isolated doesn't matter here
          bigNumberToWrappedI80F48(Math.random() * 0.3 + 0.6), // random 0.6–0.9
          bigNumberToWrappedI80F48(Math.random() * 0.1 + 0.9) // random 0.9–1.0
        )
      );

      // construct & send the tx for this bank
      const tx = new Transaction().add(
        await configBankEmode(groupAdmin.mrgnBankrunProgram, {
          bank: kaminoBank,
          tag: bankIndex, // bank’s own tag = its index
          entries,
        }),
        // For simplicity we just echo the same setting on the regular bank. This makes no sense in
        // practice it's just to simplify setup.
        await configBankEmode(groupAdmin.mrgnBankrunProgram, {
          bank: regularBank,
          tag: bankIndex, // bank’s own tag = its index
          entries,
        })
      );
      await processBankrunTransaction(bankrunContext, tx, [groupAdmin.wallet]);
    }
  });

  it("(user 1) Seeds liquidity in all regular banks", async () => {
    const user = users[1];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const amount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
    // Note: This is about the max per TX without using LUTs with regular deposits
    const depositsPerTx = 5;

    for (let i = 0; i < regularBanks.length; i += depositsPerTx) {
      const chunk = regularBanks.slice(i, i + depositsPerTx);
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

  it("(admin) Seeds liquidity in all banks - validates 16 deposits is possible", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const market = kaminoAccounts.get(MARKET);
    const tokenAReserve = kaminoAccounts.get(TOKEN_A_RESERVE);
    const farmState = farmAccounts.get(A_FARM_STATE);

    const amount = new BN(10 * 10 ** ecosystem.tokenADecimals);
    // Note: Kamino deposits sans-LUT are severely limited.
    const depositsPerTx = 2;

    for (let i = 0; i < kaminoBanks.length; i += depositsPerTx) {
      const chunk = kaminoBanks.slice(i, i + depositsPerTx);
      const depositTx: Transaction = new Transaction();
      for (const bank of chunk) {
        const [liquidityVaultAuthority] = deriveLiquidityVaultAuthority(
          bankrunProgram.programId,
          bank
        );
        const [kaminoObligation] = deriveBaseObligation(
          liquidityVaultAuthority,
          market
        );
        const [userState] = deriveUserState(
          FARMS_PROGRAM_ID,
          farmState,
          kaminoObligation
        );

        depositTx.add(
          await simpleRefreshReserve(
            klendBankrunProgram,
            tokenAReserve,
            market,
            oracles.tokenAOracle.publicKey
          ),
          await simpleRefreshObligation(
            klendBankrunProgram,
            market,
            kaminoObligation,
            [tokenAReserve]
          ),
          await makeKaminoDepositIx(
            user.mrgnBankrunProgram,
            {
              marginfiAccount: userAccount,
              bank,
              signerTokenAccount: user.tokenAAccount,
              lendingMarket: market,
              reserveLiquidityMint: ecosystem.tokenAMint.publicKey,
              obligationFarmUserState: userState,
              reserveFarmState: farmState,
            },
            amount
          )
        );
      }
      await processBankrunTransaction(bankrunContext, depositTx, [user.wallet]);
    }
  });

  // Note: the memory performance of 15 deposits and one borrow should be similar; packing that into
  // one TX without a LUT is probably not posssible.
  it("(user 0) Borrows 15 positions against 1 - validates max borrows possible", async () => {
    const user = users[0];
    const depositBank = kaminoBanks[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const market = kaminoAccounts.get(MARKET);
    const tokenAReserve = kaminoAccounts.get(TOKEN_A_RESERVE);
    const farmState = farmAccounts.get(A_FARM_STATE);
    const [liquidityVaultAuthority] = deriveLiquidityVaultAuthority(
      bankrunProgram.programId,
      depositBank
    );
    const [kaminoObligation] = deriveBaseObligation(
      liquidityVaultAuthority,
      market
    );
    const [userState] = deriveUserState(
      FARMS_PROGRAM_ID,
      farmState,
      kaminoObligation
    );

    const depositAmount = new BN(100 * 10 ** ecosystem.tokenADecimals);
    const borrowAmount = new BN(1 * 10 ** ecosystem.tokenADecimals);
    let oomAt = MAX_BALANCES;

    const depositTx = new Transaction().add(
      await simpleRefreshReserve(
        klendBankrunProgram,
        tokenAReserve,
        market,
        oracles.tokenAOracle.publicKey
      ),
      await simpleRefreshObligation(
        klendBankrunProgram,
        market,
        kaminoObligation,
        [tokenAReserve]
      ),
      await makeKaminoDepositIx(
        user.mrgnBankrunProgram,
        {
          marginfiAccount: userAccount,
          bank: depositBank,
          signerTokenAccount: user.tokenAAccount,
          lendingMarket: market,
          reserveLiquidityMint: ecosystem.tokenAMint.publicKey,
          obligationFarmUserState: userState,
          reserveFarmState: farmState,
        },
        depositAmount
      )
    );
    await processBankrunTransaction(bankrunContext, depositTx, [user.wallet]);

    for (let i = 1; i < regularBanks.length; i += 1) {
      // The kamino bank we deposited into
      const remainingAccounts: PublicKey[][] = [];
      remainingAccounts.push([
        kaminoBanks[0],
        oracles.tokenAOracle.publicKey,
        tokenAReserve,
      ]);
      // the regular bank(s) we are borrowing from
      for (let k = 1; k <= i; k++) {
        remainingAccounts.push([regularBanks[k], oracles.pythPullLst.publicKey]);
      }

      const tx = new Transaction();
      tx.add(
        ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
        ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }),
        await simpleRefreshReserve(
          klendBankrunProgram,
          tokenAReserve,
          market,
          oracles.tokenAOracle.publicKey
        ),
        await simpleRefreshObligation(
          klendBankrunProgram,
          market,
          kaminoObligation,
          [tokenAReserve]
        ),
        await borrowIx(user.mrgnBankrunProgram, {
          marginfiAccount: userAccount,
          bank: regularBanks[i],
          tokenAccount: user.lstAlphaAccount,
          remaining: composeRemainingAccounts(remainingAccounts),
          amount: borrowAmount,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(user.wallet);
      let result = await banksClient.tryProcessTransaction(tx);
      console.log("***********" + i + " ***********");
      // dumpBankrunLogs(result);

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
            `Unexpected borrowIx failure on bank ${kaminoBanks[
              i
            ].toBase58()}: ` + logs.join("\n")
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
        bank: regularBanks[MAX_BALANCES - 1],
        bankConfigOpt: config,
      })
    );
    await processBankrunTransaction(bankrunContext, tx, [groupAdmin.wallet]);
  });

  it("(user 2) Liquidates user 0", async () => {
    const liquidatee = users[0];
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT_THROWAWAY);
    const liquidator = users[2];
    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT_THROWAWAY);
    const depositAmount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
    const liquidateAmount = new BN(0.01 * 10 ** ecosystem.lstAlphaDecimals);
    const tokenAReserve = kaminoAccounts.get(TOKEN_A_RESERVE);

    const remainingAccounts: PublicKey[][] = [];
    remainingAccounts.push([
      kaminoBanks[0],
      oracles.tokenAOracle.publicKey,
      tokenAReserve,
    ]);
    for (let i = 1; i < MAX_BALANCES; i++) {
      remainingAccounts.push([regularBanks[i], oracles.pythPullLst.publicKey]);
      // console.log("bank: " + banks[i]);
    }

    // Deposit some funds to operate as a liquidator...
    let tx = new Transaction();
    tx.add(
      await depositIx(liquidator.mrgnBankrunProgram, {
        marginfiAccount: liquidatorAccount,
        bank: regularBanks[0],
        tokenAccount: liquidator.lstAlphaAccount,
        amount: depositAmount,
        depositUpToLimit: false,
      })
    );
    await processBankrunTransaction(bankrunContext, tx, [liquidator.wallet]);

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
        assetBankKey: kaminoBanks[0],
        liabilityBankKey: regularBanks[MAX_BALANCES - 1],
        liquidatorMarginfiAccount: liquidatorAccount,
        liquidateeMarginfiAccount: liquidateeAccount,
        remaining: [
          oracles.tokenAOracle.publicKey, // asset oracle
          tokenAReserve,
          oracles.pythPullLst.publicKey, // liab oracle

          ...composeRemainingAccounts([
            // liquidator accounts
            [regularBanks[0], oracles.pythPullLst.publicKey],
            // We pick this up as an asset
            [kaminoBanks[0], oracles.tokenAOracle.publicKey, tokenAReserve],
            // We pick this up as a liability
            [regularBanks[MAX_BALANCES - 1], oracles.pythPullLst.publicKey],
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
    // dumpBankrunLogs(result);

    const liquidateeAccAfter =
      await bankrunProgram.account.marginfiAccount.fetch(liquidateeAccount);
    dumpAccBalances(liquidateeAccAfter);
    const liquidatorAccAfter =
      await bankrunProgram.account.marginfiAccount.fetch(liquidatorAccount);
    dumpAccBalances(liquidatorAccAfter);

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
});
