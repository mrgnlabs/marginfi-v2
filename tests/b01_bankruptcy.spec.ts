import { AnchorProvider, BN, Wallet } from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  PublicKey,
  SystemProgram,
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
  verbose,
} from "./rootHooks";
import { accrueInterest, configureBank } from "./utils/group-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { assert } from "chai";
import { emptyBankConfigOptRaw, ORACLE_CONF_INTERVAL } from "./utils/types";
import {
  borrowIx,
  composeRemainingAccounts,
  depositIx,
  liquidateIx,
  withdrawIx,
} from "./utils/user-instructions";
import {
  bigNumberToWrappedI80F48,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import { Clock } from "solana-bankrun";
import { genericMultiBankTestSetup } from "./genericSetups";
import { assertBankrunTxFailed } from "./utils/genericTests";
import { initOrUpdatePriceUpdateV2 } from "./utils/pyth-pull-mocks";

const USER_ACCOUNT_THROWAWAY = "throwaway_account1";
const ONE_YEAR_IN_SECONDS = 2 * 365 * 24 * 60 * 60;

let banks: PublicKey[] = [];

describe("Bank bankruptcy tests", () => {
  it("init group, init banks, and fund banks", async () => {
    const result = await genericMultiBankTestSetup(2, USER_ACCOUNT_THROWAWAY);
    banks = result.banks;
  });

  it("(admin) Seeds liquidity in both banks", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const amount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);

    const tx = new Transaction();
    for (const bank of banks) {
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
    await banksClient.processTransaction(tx);
  });

  it("(admin) Sets both banks' asset weights to 0.9", async () => {
    let config = emptyBankConfigOptRaw();
    banks[0];
    config.assetWeightInit = bigNumberToWrappedI80F48(0.9); // 90%
    config.assetWeightMaint = bigNumberToWrappedI80F48(0.9); // 90%

    let tx = new Transaction().add(
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: banks[0],
        bankConfigOpt: config,
      }),
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: banks[1],
        bankConfigOpt: config,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
  });

  it("(user 0) Borrows B (bank 1) against A (bank 0)", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const depositAmount = new BN(100 * 10 ** ecosystem.lstAlphaDecimals);
    const borrowAmount = new BN(85 * 10 ** ecosystem.lstAlphaDecimals);

    const depositTx = new Transaction();
    depositTx.add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[0],
        tokenAccount: user.lstAlphaAccount,
        amount: depositAmount,
        depositUpToLimit: false,
      })
    );
    depositTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    depositTx.sign(user.wallet);
    await banksClient.processTransaction(depositTx);

    const remainingAccounts: PublicKey[][] = [];
    remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
    remainingAccounts.push([banks[1], oracles.pythPullLst.publicKey]);

    const borrowTx = new Transaction();
    borrowTx.add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }),
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[1],
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts(remainingAccounts),
        amount: borrowAmount,
      })
    );
    borrowTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    borrowTx.sign(user.wallet);
    await banksClient.processTransaction(borrowTx);
  });

  it("One year elapses", async () => {
    let now = Math.floor(Date.now() / 1000);
    const targetUnix = BigInt(now + ONE_YEAR_IN_SECONDS);

    // Construct a new Clock; we only care about the unixTimestamp field here.
    const newClock = new Clock(
      0n, // slot
      0n, // epochStartTimestamp
      0n, // epoch
      0n, // leaderScheduleEpoch
      targetUnix
    );

    bankrunContext.setClock(newClock);

    // Crank oracles so that the prices are not stale
    const provider = AnchorProvider.local();
      const wallet = provider.wallet as Wallet;
    let priceAlpha = ecosystem.lstAlphaPrice * 10 ** ecosystem.lstAlphaDecimals;
    let confAlpha = priceAlpha * ORACLE_CONF_INTERVAL;
    await initOrUpdatePriceUpdateV2(
      wallet,
      oracles.pythPullLstOracleFeed.publicKey,
      new BN(priceAlpha),
      new BN(confAlpha),
      now + ONE_YEAR_IN_SECONDS,
      -ecosystem.lstAlphaDecimals,
      oracles.pythPullLst,
      bankrunContext
    );
  });

  it("(user 0 - permissionless) Accrues interest on bank 1", async () => {
    const user = users[0];

    const tx = new Transaction();
    tx.add(
      await accrueInterest(user.mrgnBankrunProgram, {
        bank: banks[1],
      }),
      // dummy ix to trick bankrun
      SystemProgram.transfer({
        fromPubkey: users[0].wallet.publicKey,
        toPubkey: bankrunProgram.provider.publicKey,
        lamports: 11,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const bankAcc = await bankrunProgram.account.bank.fetch(banks[1]);
    let bankValues = {
      asset: wrappedI80F48toBigNumber(bankAcc.assetShareValue).toNumber(),
      liability: wrappedI80F48toBigNumber(
        bankAcc.liabilityShareValue
      ).toNumber(),
    };

    if (verbose) {
      console.log("Value per share after one year:");
      console.log(
        `  asset: ${bankValues.asset}, liab: ${bankValues.liability}`
      );
      console.log("total assets: " + wrappedI80F48toBigNumber(bankAcc.totalAssetShares).toNumber());
      console.log("total liabs: " + wrappedI80F48toBigNumber(bankAcc.totalLiabilityShares).toNumber());
    }

    assert.isAbove(bankValues.liability, bankValues.asset);
  });

  it("(admin) Tries to withdraw some B (bank 1 - now bankrupt) - should fail", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const amount = new BN(0.01 * 10 ** ecosystem.lstAlphaDecimals);

    const remainingAccounts: PublicKey[][] = [];
    remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
    remainingAccounts.push([banks[1], oracles.pythPullLst.publicKey]);

    const tx = new Transaction();
    tx.add(
      await withdrawIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[1],
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts(remainingAccounts),
        amount,
        withdrawAll: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    // IllegalUtilizationRatio
    assertBankrunTxFailed(result, "0x178a");
  });

  it("(user 0) Tries to borrow a little more B (bank 1 - now bankrupt) against A (bank 0) - should fail", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const borrowAmount = new BN(0.01 * 10 ** ecosystem.lstAlphaDecimals);

    const remainingAccounts: PublicKey[][] = [];
    remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
    remainingAccounts.push([banks[1], oracles.pythPullLst.publicKey]);

    const borrowTx = new Transaction();
    borrowTx.add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }),
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[1],
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts(remainingAccounts),
        amount: borrowAmount,
      })
    );
    borrowTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    borrowTx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(borrowTx);
    // IllegalUtilizationRatio
    assertBankrunTxFailed(result, "0x178a");
  });

  it("(admin) Liquidates user 0 and brings bank 1 bank to life - happy path", async () => {
    const liquidator = groupAdmin;
    const liquidatorMarginfiAccount = liquidator.accounts.get(
      USER_ACCOUNT_THROWAWAY
    );
    const liquidatee = users[0];
    const liquidateeMarginfiAccount = liquidatee.accounts.get(
      USER_ACCOUNT_THROWAWAY
    );
    const liquidateAmount = new BN(85 * 10 ** ecosystem.lstAlphaDecimals);

    const remainingAccounts: PublicKey[][] = [];
    remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
    remainingAccounts.push([banks[1], oracles.pythPullLst.publicKey]);

    const liquidateTx = new Transaction();
    liquidateTx.add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }),
      await liquidateIx(liquidator.mrgnBankrunProgram, {
        assetBankKey: banks[0],
        liabilityBankKey: banks[1],
        liquidatorMarginfiAccount,
        liquidateeMarginfiAccount,
        remaining: [
          oracles.pythPullLst.publicKey,
          oracles.pythPullLst.publicKey,
          ...composeRemainingAccounts(remainingAccounts),
          ...composeRemainingAccounts(remainingAccounts),
        ],
        amount: liquidateAmount,
      })
    );
    liquidateTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    liquidateTx.sign(liquidator.wallet);
    await banksClient.processTransaction(liquidateTx);
  });

    it("(admin) Retries to withdraw some B (bank 1 - is no longer bankrupt) - happy path", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_THROWAWAY);
    const amount = new BN(0.01 * 10 ** ecosystem.lstAlphaDecimals);

    const remainingAccounts: PublicKey[][] = [];
    remainingAccounts.push([banks[0], oracles.pythPullLst.publicKey]);
    remainingAccounts.push([banks[1], oracles.pythPullLst.publicKey]);

    const tx = new Transaction();
    tx.add(
      await withdrawIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: banks[1],
        tokenAccount: user.lstAlphaAccount,
        remaining: composeRemainingAccounts(remainingAccounts),
        amount,
        withdrawAll: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    console.log("LOGS: ", result.meta.logMessages);
    // IllegalUtilizationRatio
    assertBankrunTxFailed(result, "0x178b");
  });
});
