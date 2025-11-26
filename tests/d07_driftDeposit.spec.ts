import { BN } from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import {
  ecosystem,
  driftAccounts,
  DRIFT_USDC_BANK,
  DRIFT_TOKENA_BANK,
  DRIFT_TOKENA_PULL_ORACLE,
  users,
  verbose,
  bankrunContext,
  bankrunProgram,
  driftBankrunProgram,
  bankRunProvider,
  oracles,
} from "./rootHooks";
import { assert } from "chai";
import { MockUser, USER_ACCOUNT_D } from "./utils/mocks";
import { processBankrunTransaction } from "./utils/tools";
import { makeDriftDepositIx } from "./utils/drift-instructions";
import {
  assertBNEqual,
  getTokenBalance,
  assertBankrunTxFailed,
} from "./utils/genericTests";
import {
  getDriftUserAccount,
  getSpotMarketAccount,
  USDC_MARKET_INDEX,
  TOKEN_A_MARKET_INDEX,
} from "./utils/drift-utils";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";

describe("d07: Drift Deposit Tests", () => {
  let driftUsdcBank: PublicKey;
  let driftTokenABank: PublicKey;

  before(async () => {
    driftUsdcBank = driftAccounts.get(DRIFT_USDC_BANK);
    driftTokenABank = driftAccounts.get(DRIFT_TOKENA_BANK);
  });

  async function assertDriftDepositSuccess(
    user: MockUser,
    bankPubkey: PublicKey,
    scaledBalanceChange: BN,
    expectedTotalBalance?: BN
  ) {
    const marginfiAccount = user.accounts.get(USER_ACCOUNT_D);

    const userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      marginfiAccount
    );

    const balance = userAcc.lendingAccount.balances.find(
      (b) => b.bankPk.equals(bankPubkey) && b.active === 1
    );

    assert(balance);

    const assetSharesBN = new BN(
      wrappedI80F48toBigNumber(balance.assetShares).toString()
    );
    const expectedBalance = expectedTotalBalance || scaledBalanceChange;
    assertBNEqual(assetSharesBN, expectedBalance);
  }

  it("(user 0) First deposit to Drift - 100 USDC", async () => {
    const user = users[0];
    const amount = new BN(100 * 10 ** ecosystem.usdcDecimals);

    const bank = await bankrunProgram.account.bank.fetch(driftUsdcBank);
    const [userUsdcBefore, driftUserBefore] = await Promise.all([
      getTokenBalance(bankRunProvider, user.usdcAccount),
      getDriftUserAccount(driftBankrunProgram, bank.driftUser),
    ]);

    const spotPositionBefore = driftUserBefore.spotPositions[0];
    const scaledBalanceBefore = new BN(
      spotPositionBefore.scaledBalance.toString()
    );

    const depositIx = await makeDriftDepositIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftUsdcBank,
        signerTokenAccount: user.usdcAccount,
      },
      amount,
      USDC_MARKET_INDEX
    );

    const tx = new Transaction().add(depositIx);
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
    assertBNEqual(amount, userUsdcBefore - userUsdcAfter);

    const driftUserAfter = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );
    const spotPositionAfter = driftUserAfter.spotPositions[0];
    const scaledBalanceAfter = new BN(
      spotPositionAfter.scaledBalance.toString()
    );
    const scaledBalanceChange = scaledBalanceAfter.sub(scaledBalanceBefore);

    await assertDriftDepositSuccess(user, driftUsdcBank, scaledBalanceChange);
  });

  it("(user 0) Second deposit to Drift - 50 USDC", async () => {
    const user = users[0];
    const amount = new BN(50 * 10 ** ecosystem.usdcDecimals);

    const bank = await bankrunProgram.account.bank.fetch(driftUsdcBank);
    const driftUserBefore = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );
    const scaledBalanceBefore = new BN(
      driftUserBefore.spotPositions[0].scaledBalance.toString()
    );

    const depositIx = await makeDriftDepositIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftUsdcBank,
        signerTokenAccount: user.usdcAccount,
      },
      amount,
      USDC_MARKET_INDEX
    );

    const tx = new Transaction().add(depositIx);
    await processBankrunTransaction(bankrunContext, tx, [user.wallet]);

    const driftUserAfter = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );
    // USDC (market 0) uses position[0]
    const scaledBalanceAfter = new BN(
      driftUserAfter.spotPositions[0].scaledBalance.toString()
    );
    const scaledBalanceChange = scaledBalanceAfter.sub(scaledBalanceBefore);

    // The marginfi account tracks user deposits (150 USDC), not the initial 100 units
    const bankInitDepositAmountInScaledUnits = new BN(100_000);
    const expectedMarginfiBalance = scaledBalanceAfter.sub(
      bankInitDepositAmountInScaledUnits
    );
    await assertDriftDepositSuccess(
      user,
      driftUsdcBank,
      scaledBalanceChange,
      expectedMarginfiBalance
    );

    const userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      user.accounts.get(USER_ACCOUNT_D)
    );
    const balance = userAcc.lendingAccount.balances.find(
      (b) => b.bankPk.equals(driftUsdcBank) && b.active === 1
    );

    const totalShares = new BN(
      wrappedI80F48toBigNumber(balance.assetShares).toString()
    );
    assertBNEqual(totalShares, expectedMarginfiBalance);
  });

  it("(user 1) First deposit to Drift - 200 USDC", async () => {
    const user = users[1];
    const amount = new BN(200 * 10 ** ecosystem.usdcDecimals);

    const depositIx = await makeDriftDepositIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftUsdcBank,
        signerTokenAccount: user.usdcAccount,
      },
      amount,
      USDC_MARKET_INDEX
    );

    const tx = new Transaction().add(depositIx);
    await processBankrunTransaction(bankrunContext, tx, [user.wallet]);

    const userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      user.accounts.get(USER_ACCOUNT_D)
    );
    const balance = userAcc.lendingAccount.balances.find(
      (b) => b.bankPk.equals(driftUsdcBank) && b.active === 1
    );
    assert(balance);
  });

  it("(user 0) Deposit zero amount - should fail", async () => {
    const user = users[0];
    const amount = new BN(0);

    const depositIx = await makeDriftDepositIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftUsdcBank,
        signerTokenAccount: user.usdcAccount,
      },
      amount,
      USDC_MARKET_INDEX
    );

    const tx = new Transaction().add(depositIx);
    const result = await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      true
    );

    // Drift Error Code: InsufficientDeposit. Error Number: 6002. custom program error: 0x1772
    assertBankrunTxFailed(result, "0x1772");
  });

  it("(user 0) Deposit Token A to Drift - 5 Token A", async () => {
    const user = users[0];
    const amount = new BN(5 * 10 ** ecosystem.tokenADecimals);

    const bank = await bankrunProgram.account.bank.fetch(driftTokenABank);
    const [userTokenABefore, driftUserBefore] = await Promise.all([
      getTokenBalance(bankRunProvider, user.tokenAAccount),
      getDriftUserAccount(driftBankrunProgram, bank.driftUser),
    ]);

    const spotPositionBefore = driftUserBefore.spotPositions[1];
    const scaledBalanceBefore = new BN(
      spotPositionBefore.scaledBalance.toString()
    );

    const depositIx = await makeDriftDepositIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: user.accounts.get(USER_ACCOUNT_D),
        bank: driftTokenABank,
        signerTokenAccount: user.tokenAAccount,
        driftOracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE),
      },
      amount,
      TOKEN_A_MARKET_INDEX
    );

    const tx = new Transaction().add(depositIx);
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
    assertBNEqual(amount, userTokenABefore - userTokenAAfter);

    const driftUserAfter = await getDriftUserAccount(
      driftBankrunProgram,
      bank.driftUser
    );
    // Token A (market 1) uses position[1]
    const spotPositionAfter = driftUserAfter.spotPositions[1];
    const scaledBalanceAfter = new BN(
      spotPositionAfter.scaledBalance.toString()
    );
    const scaledBalanceChange = scaledBalanceAfter.sub(scaledBalanceBefore);

    await assertDriftDepositSuccess(user, driftTokenABank, scaledBalanceChange);
  });

  it("Verify position indices are correct", async () => {
    const usdcBank = await bankrunProgram.account.bank.fetch(driftUsdcBank);
    const tokenABank = await bankrunProgram.account.bank.fetch(driftTokenABank);

    const usdcDriftUser = await getDriftUserAccount(
      driftBankrunProgram,
      usdcBank.driftUser
    );
    const tokenADriftUser = await getDriftUserAccount(
      driftBankrunProgram,
      tokenABank.driftUser
    );

    const usdcPosition = usdcDriftUser.spotPositions[0];
    assert(usdcPosition.scaledBalance.gt(new BN(0)));
    assert.equal(usdcPosition.marketIndex, USDC_MARKET_INDEX);
    const usdcPosition1 = usdcDriftUser.spotPositions[1];
    assertBNEqual(usdcPosition1.scaledBalance, new BN(0));

    const tokenAPosition = tokenADriftUser.spotPositions[1];
    assert(tokenAPosition.scaledBalance.gt(new BN(0)));
    assert.equal(tokenAPosition.marketIndex, TOKEN_A_MARKET_INDEX);
    const tokenAPosition0 = tokenADriftUser.spotPositions[0];
    assertBNEqual(tokenAPosition0.scaledBalance, new BN(0));
  });

  it("Verify spot market totals", async () => {
    const usdcSpotMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );
    assert(usdcSpotMarket.depositBalance.gt(new BN(0)));

    const tokenASpotMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );
    assert(tokenASpotMarket.depositBalance.gt(new BN(0)));
  });
});
