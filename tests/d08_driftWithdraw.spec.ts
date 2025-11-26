import { BN } from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import {
  ecosystem,
  driftAccounts,
  DRIFT_USDC_BANK,
  DRIFT_TOKENA_BANK,
  DRIFT_USDC_SPOT_MARKET,
  DRIFT_TOKENA_SPOT_MARKET,
  DRIFT_TOKENA_PULL_ORACLE,
  users,
  bankrunContext,
  bankrunProgram,
  driftBankrunProgram,
  bankRunProvider,
  oracles,
} from "./rootHooks";
import { assert } from "chai";
import { MockUser, USER_ACCOUNT_D } from "./utils/mocks";
import { processBankrunTransaction } from "./utils/tools";
import { makeDriftWithdrawIx } from "./utils/drift-instructions";
import {
  assertBNEqual,
  getTokenBalance,
  assertBankrunTxFailed,
  assertBNApproximately,
} from "./utils/genericTests";
import {
  getDriftScalingFactor,
  getDriftUserAccount,
  getSpotMarketAccount,
} from "./utils/drift-utils";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { composeRemainingAccounts } from "./utils/user-instructions";

describe("d08: Drift Withdraw Tests", () => {
  let driftUsdcBank: PublicKey;
  let driftTokenABank: PublicKey;

  before(async () => {
    driftUsdcBank = driftAccounts.get(DRIFT_USDC_BANK);
    driftTokenABank = driftAccounts.get(DRIFT_TOKENA_BANK);
  });

  async function assertDriftWithdrawSuccess(
    user: MockUser,
    bankPubkey: PublicKey
  ) {
    const marginfiAccount = user.accounts.get(USER_ACCOUNT_D);

    const userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      marginfiAccount
    );

    userAcc.lendingAccount.balances.find(
      (b) => b.bankPk.equals(bankPubkey) && b.active === 1
    );

    const bank = await bankrunProgram.account.bank.fetch(bankPubkey);
    const driftUserAfter = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );

    const spotPositionAfter = driftUserAfter.spotPositions[0];
    new BN(spotPositionAfter.scaledBalance.toString());
  }

  it("(user 0) Partial withdrawal from Drift - 50 USDC", async () => {
    const user = users[0];
    const amount = new BN(50 * 10 ** ecosystem.usdcDecimals);

    const bank = await bankrunProgram.account.bank.fetch(driftUsdcBank);
    const [userUsdcBefore, driftUserBefore] = await Promise.all([
      getTokenBalance(bankRunProvider, user.usdcAccount),
      getDriftUserAccount(driftBankrunProgram, bank.driftUser),
    ]);

    const spotPositionBefore = driftUserBefore.spotPositions[0];
    const scaledBalanceBefore = new BN(
      spotPositionBefore.scaledBalance.toString()
    );
    const marginfiAccountBefore =
      await bankrunProgram.account.marginfiAccount.fetch(
        user.accounts.get(USER_ACCOUNT_D)
      );
    const marginfiBalanceBefore =
      marginfiAccountBefore.lendingAccount.balances.find(
        (b) => b.bankPk.equals(driftUsdcBank) && b.active === 1
      );
    const sharesBefore = new BN(
      wrappedI80F48toBigNumber(marginfiBalanceBefore.assetShares).toString()
    );

    const withdrawIx = await makeDriftWithdrawIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftUsdcBank,
        destinationTokenAccount: user.usdcAccount,
        driftOracle: null,
      },
      {
        amount: amount,
        withdraw_all: false,
        remaining: composeRemainingAccounts([
          [
            driftUsdcBank,
            oracles.usdcOracle.publicKey,
            driftAccounts.get(DRIFT_USDC_SPOT_MARKET),
          ],
          [
            driftTokenABank,
            oracles.tokenAOracle.publicKey,
            driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET),
          ],
        ]),
      },
      driftBankrunProgram
    );

    const tx = new Transaction().add(withdrawIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      false,
      true
    );

    const userUsdcAfter = await getTokenBalance(
      bankRunProvider,
      user.usdcAccount
    );
    assertBNEqual(amount, userUsdcAfter - userUsdcBefore);

    const driftUserAfter = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );
    const spotPositionAfter = driftUserAfter.spotPositions[0];
    const scaledBalanceAfter = new BN(
      spotPositionAfter.scaledBalance.toString()
    );
    const scaledBalanceDecrease = scaledBalanceBefore.sub(scaledBalanceAfter);

    const marginfiAccountAfter =
      await bankrunProgram.account.marginfiAccount.fetch(
        user.accounts.get(USER_ACCOUNT_D)
      );
    const marginfiBalanceAfter =
      marginfiAccountAfter.lendingAccount.balances.find(
        (b) => b.bankPk.equals(driftUsdcBank) && b.active === 1
      );
    const sharesAfter = new BN(
      wrappedI80F48toBigNumber(marginfiBalanceAfter.assetShares).toString()
    );
    const sharesDecrease = sharesBefore.sub(sharesAfter);

    assertBNEqual(scaledBalanceDecrease, sharesDecrease);

    await assertDriftWithdrawSuccess(user, driftUsdcBank);
  });

  it("(user 0) Full withdrawal from Drift - remaining USDC balance", async () => {
    const user = users[0];

    const bank = await bankrunProgram.account.bank.fetch(driftUsdcBank);
    const [userUsdcBefore, driftUserBefore] = await Promise.all([
      getTokenBalance(bankRunProvider, user.usdcAccount),
      getDriftUserAccount(driftBankrunProgram, bank.driftUser),
    ]);

    const spotPositionBefore = driftUserBefore.spotPositions[0];
    const withdrawIx = await makeDriftWithdrawIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftUsdcBank,
        destinationTokenAccount: user.usdcAccount,
        driftOracle: null,
      },
      {
        amount: new BN(0),
        withdraw_all: true,
        remaining: composeRemainingAccounts([
          [
            driftTokenABank,
            oracles.tokenAOracle.publicKey,
            driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET),
          ],
        ]),
      },
      driftBankrunProgram
    );

    const tx = new Transaction().add(withdrawIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      false,
      true
    );
    const userUsdcAfter = await getTokenBalance(
      bankRunProvider,
      user.usdcAccount
    );
    const amountReceived = userUsdcAfter - userUsdcBefore;
    assert.ok(amountReceived > 0);
  });

  it("(user 1) Withdraw 100 USDC from Drift", async () => {
    const user = users[1];
    const amount = new BN(100 * 10 ** ecosystem.usdcDecimals);
    const withdrawIx = await makeDriftWithdrawIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftUsdcBank,
        destinationTokenAccount: user.usdcAccount,
        driftOracle: null,
      },
      {
        amount: amount,
        withdraw_all: false,
        remaining: composeRemainingAccounts([
          [
            driftUsdcBank,
            oracles.usdcOracle.publicKey,
            driftAccounts.get(DRIFT_USDC_SPOT_MARKET),
          ],
        ]),
      },
      driftBankrunProgram
    );

    const tx = new Transaction().add(withdrawIx);
    await processBankrunTransaction(bankrunContext, tx, [user.wallet]);

    const userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      user.accounts.get(USER_ACCOUNT_D)
    );
    const balance = userAcc.lendingAccount.balances.find(
      (b) => b.bankPk.equals(driftUsdcBank) && b.active === 1
    );
    assert(balance);
  });

  it("(user 0) Partial withdrawal from Drift - 2 Token A", async () => {
    const user = users[0];
    const amount = new BN(2 * 10 ** ecosystem.tokenADecimals);

    const bank = await bankrunProgram.account.bank.fetch(driftTokenABank);
    const [userTokenABefore, driftUserBefore] = await Promise.all([
      getTokenBalance(bankRunProvider, user.tokenAAccount),
      getDriftUserAccount(driftBankrunProgram, bank.driftUser),
    ]);

    const spotPositionBefore = driftUserBefore.spotPositions[0];
    const scaledBalanceBefore = new BN(
      spotPositionBefore.scaledBalance.toString()
    );
    const withdrawIx = await makeDriftWithdrawIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftTokenABank,
        destinationTokenAccount: user.tokenAAccount,
        driftOracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE),
      },
      {
        amount: amount,
        withdraw_all: false,
        remaining: composeRemainingAccounts([
          [
            driftTokenABank,
            oracles.tokenAOracle.publicKey,
            driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET),
          ],
        ]),
      },
      driftBankrunProgram
    );

    const tx = new Transaction().add(withdrawIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      false,
      true
    );
    const userTokenAAfter = await getTokenBalance(
      bankRunProvider,
      user.tokenAAccount
    );
    assertBNEqual(amount, userTokenAAfter - userTokenABefore);

    const driftUserAfter = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );
    const spotPositionAfter = driftUserAfter.spotPositions[0];
    const scaledBalanceAfter = new BN(
      spotPositionAfter.scaledBalance.toString()
    );
    const scaledBalanceDecrease = scaledBalanceBefore.sub(scaledBalanceAfter);

    await assertDriftWithdrawSuccess(user, driftTokenABank);
  });

  it("(user 0) Try to withdraw more than deposited - should fail", async () => {
    const user = users[0];
    const excessiveAmount = new BN(1000 * 10 ** ecosystem.tokenADecimals);

    const withdrawIx = await makeDriftWithdrawIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftTokenABank,
        destinationTokenAccount: user.tokenAAccount,
        driftOracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE),
      },
      {
        amount: excessiveAmount,
        withdraw_all: false,
        remaining: composeRemainingAccounts([
          [
            driftUsdcBank,
            oracles.usdcOracle.publicKey,
            driftAccounts.get(DRIFT_USDC_SPOT_MARKET),
          ],
          [
            driftTokenABank,
            oracles.tokenAOracle.publicKey,
            driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET),
          ],
        ]),
      },
      driftBankrunProgram
    );

    const tx = new Transaction().add(withdrawIx);
    const result = await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      true,
      false
    );

    assertBankrunTxFailed(result, 6020);
  });

  it("(user 0) Try to withdraw 0 tokens - should succeed with no transfer", async () => {
    const user = users[0];
    const zeroAmount = new BN(0);

    const bank = await bankrunProgram.account.bank.fetch(driftTokenABank);
    const [userTokenABefore, driftUserBefore] = await Promise.all([
      getTokenBalance(bankRunProvider, user.tokenAAccount),
      getDriftUserAccount(driftBankrunProgram, bank.driftUser),
    ]);

    const spotPositionBefore = driftUserBefore.spotPositions[0];
    const scaledBalanceBefore = new BN(
      spotPositionBefore.scaledBalance.toString()
    );
    const withdrawIx = await makeDriftWithdrawIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftTokenABank,
        destinationTokenAccount: user.tokenAAccount,
        driftOracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE),
      },
      {
        amount: zeroAmount,
        withdraw_all: false,
        remaining: composeRemainingAccounts([
          [
            driftTokenABank,
            oracles.tokenAOracle.publicKey,
            driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET),
          ],
        ]),
      },
      driftBankrunProgram
    );

    const tx = new Transaction().add(withdrawIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      false,
      true
    );

    const userTokenAAfter = await getTokenBalance(
      bankRunProvider,
      user.tokenAAccount
    );
    assertBNEqual(new BN(userTokenAAfter), new BN(userTokenABefore));
    const driftUserAfter = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );
    const spotPositionAfter = driftUserAfter.spotPositions[0];
    const scaledBalanceAfter = new BN(
      spotPositionAfter.scaledBalance.toString()
    );
    assertBNEqual(scaledBalanceAfter, scaledBalanceBefore);
  });

  it("(user 0) Full withdrawal from Drift - remaining Token A balance", async () => {
    const user = users[0];

    const bank = await bankrunProgram.account.bank.fetch(driftTokenABank);
    const [userTokenABefore, driftUserBefore] = await Promise.all([
      getTokenBalance(bankRunProvider, user.tokenAAccount),
      getDriftUserAccount(driftBankrunProgram, bank.driftUser),
    ]);

    const spotPositionBefore = driftUserBefore.spotPositions[0];
    const scaledBalanceBefore = new BN(
      spotPositionBefore.scaledBalance.toString()
    );
    const withdrawIx = await makeDriftWithdrawIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftTokenABank,
        destinationTokenAccount: user.tokenAAccount,
        driftOracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE),
      },
      {
        amount: new BN(0),
        withdraw_all: true,
        remaining: composeRemainingAccounts([
          [
            driftUsdcBank,
            oracles.usdcOracle.publicKey,
            driftAccounts.get(DRIFT_USDC_SPOT_MARKET),
          ],
          [
            driftTokenABank,
            oracles.tokenAOracle.publicKey,
            driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET),
          ],
        ]),
      },
      driftBankrunProgram
    );

    const tx = new Transaction().add(withdrawIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      false,
      true
    );
    const userTokenAAfter = await getTokenBalance(
      bankRunProvider,
      user.tokenAAccount
    );
    const amountReceived = userTokenAAfter - userTokenABefore;
    assert.ok(amountReceived > 0);

    const driftUserAfter = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );
    const spotPositionAfter = driftUserAfter.spotPositions[0];
    const scaledBalanceAfter = new BN(
      spotPositionAfter.scaledBalance.toString()
    );
    assertBNEqual(scaledBalanceAfter, new BN(0));

    const marginfiAccountAfter =
      await bankrunProgram.account.marginfiAccount.fetch(
        user.accounts.get(USER_ACCOUNT_D)
      );
    const marginfiBalanceAfter =
      marginfiAccountAfter.lendingAccount.balances.find((b) =>
        b.bankPk.equals(driftTokenABank)
      );
    assert.isUndefined(marginfiBalanceAfter);
  });
});
