import { assert } from "chai";
import { Transaction, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { createMintToInstruction } from "@solana/spl-token";
import {
  ecosystem,
  globalProgramAdmin,
  users,
  oracles,
  bankrunContext,
  bankRunProvider,
  driftBankrunProgram,
  driftAccounts,
  DRIFT_TOKENA_PULL_ORACLE,
} from "./rootHooks";
import { processBankrunTransaction } from "./utils/tools";
import {
  getTokenBalance,
  assertBNEqual,
  assertBNApproximately,
} from "./utils/genericTests";
import { refreshPullOraclesBankrun } from "./utils/bankrun-oracles";
import { makeDepositIx, makeWithdrawIx } from "./utils/drift-sdk";
import { deriveSpotMarketPDA } from "./utils/pdas";
import {
  getSpotMarketAccount,
  getUserPositions,
  getUserAccount,
  calculateUtilizationRate,
  calculateInterestRate,
  getDriftScalingFactor,
  formatTokenAmount,
  formatDepositAmount,
  formatRawAmount,
  isDriftPositionBorrow,
  isDriftPositionDeposit,
  USDC_MARKET_INDEX,
  TOKEN_A_MARKET_INDEX,
} from "./utils/drift-utils";

describe("d04: Drift - User Deposits and Borrows", () => {
  let userA: (typeof users)[0];
  let userB: (typeof users)[0];

  // Balanced deposit/borrow amounts for cross-asset lending
  // User A: Deposits Token A, borrows USDC
  const USER_A_TOKEN_A_DEPOSIT = new BN(5_000 * 10 ** ecosystem.tokenADecimals); // 5,000 Token A = $50,000 @ $10/TKA
  const USER_A_USDC_BORROW = new BN(10_000 * 10 ** ecosystem.usdcDecimals); // 10,000 USDC = $10,000 @ $1/USDC

  // User B: Deposits USDC, borrows Token A
  const USER_B_USDC_DEPOSIT = new BN(100_000 * 10 ** ecosystem.usdcDecimals); // 100,000 USDC = $100,000 @ $1/USDC
  const USER_B_TOKEN_A_BORROW = new BN(500 * 10 ** ecosystem.tokenADecimals); // 500 Token A = $5,000 @ $10/TKA

  // Drift uses token-specific scaling factors: 10^(9 - token_decimals)
  const USDC_SCALING_FACTOR = getDriftScalingFactor(ecosystem.usdcDecimals); // 10^3 = 1,000
  const TOKEN_A_SCALING_FACTOR = getDriftScalingFactor(
    ecosystem.tokenADecimals
  ); // 10^1 = 10

  before(async () => {
    userA = users[0];
    userB = users[1];

    const fundUserATokenAIx = createMintToInstruction(
      ecosystem.tokenAMint.publicKey,
      userA.tokenAAccount,
      globalProgramAdmin.wallet.publicKey,
      USER_A_TOKEN_A_DEPOSIT.mul(new BN(2)).toNumber()
    );

    const fundUserAUsdcIx = createMintToInstruction(
      ecosystem.usdcMint.publicKey,
      userA.usdcAccount,
      globalProgramAdmin.wallet.publicKey,
      1 * 10 ** ecosystem.usdcDecimals
    );

    const fundUserATx = new Transaction().add(
      fundUserATokenAIx,
      fundUserAUsdcIx
    );
    await processBankrunTransaction(
      bankrunContext,
      fundUserATx,
      [globalProgramAdmin.wallet],
      false,
      true
    );

    const fundUserBUsdcIx = createMintToInstruction(
      ecosystem.usdcMint.publicKey,
      userB.usdcAccount,
      globalProgramAdmin.wallet.publicKey,
      USER_B_USDC_DEPOSIT.mul(new BN(2)).toNumber()
    );

    const fundUserBTokenAIx = createMintToInstruction(
      ecosystem.tokenAMint.publicKey,
      userB.tokenAAccount,
      globalProgramAdmin.wallet.publicKey,
      1 * 10 ** ecosystem.tokenADecimals
    );

    const fundUserBTx = new Transaction().add(
      fundUserBUsdcIx,
      fundUserBTokenAIx
    );
    await processBankrunTransaction(
      bankrunContext,
      fundUserBTx,
      [globalProgramAdmin.wallet],
      false,
      true
    );

    const { banksClient } = bankrunContext;
    await refreshPullOraclesBankrun(oracles, bankrunContext, banksClient);
  });

  it("(user A) deposits Token A to spot market", async () => {
    const userBalanceBefore = await getTokenBalance(
      bankRunProvider,
      userA.tokenAAccount
    );
    const spotMarketBefore = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );

    const [tokenASpotMarket] = deriveSpotMarketPDA(
      driftBankrunProgram.programId,
      TOKEN_A_MARKET_INDEX
    );
    const depositIx = await makeDepositIx(
      driftBankrunProgram,
      {
        authority: userA.wallet.publicKey,
        userTokenAccount: userA.tokenAAccount,
      },
      {
        marketIndex: TOKEN_A_MARKET_INDEX,
        amount: USER_A_TOKEN_A_DEPOSIT,
        subAccountId: 0,
        reduceOnly: false,
        remainingOracles: [driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE)!],
        remainingMarkets: [tokenASpotMarket],
      }
    );

    const tx = new Transaction().add(depositIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [userA.wallet],
      false,
      true
    );
    const userBalanceAfter = await getTokenBalance(
      bankRunProvider,
      userA.tokenAAccount
    );
    const spotMarketAfter = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );

    assert.equal(
      userBalanceBefore - userBalanceAfter,
      USER_A_TOKEN_A_DEPOSIT.toNumber()
    );

    const depositIncrease = spotMarketAfter.depositBalance.sub(
      spotMarketBefore.depositBalance
    );

    // Drift uses a scaling factor for deposit balances
    const expectedScaledAmount = USER_A_TOKEN_A_DEPOSIT.mul(
      TOKEN_A_SCALING_FACTOR
    );

    assert.ok(depositIncrease.eq(expectedScaledAmount));

    const positions = await getUserPositions(
      driftBankrunProgram,
      userA.wallet.publicKey,
      0
    );
    const tokenAPosition = positions.find(
      (p: any) => p.marketIndex === TOKEN_A_MARKET_INDEX
    );
    assert.ok(tokenAPosition);
  });

  it("(user B) deposits USDC to spot market", async () => {
    const userBalanceBefore = await getTokenBalance(
      bankRunProvider,
      userB.usdcAccount
    );
    const spotMarketBefore = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );

    const [usdcSpotMarket] = deriveSpotMarketPDA(
      driftBankrunProgram.programId,
      USDC_MARKET_INDEX
    );
    const depositIx = await makeDepositIx(
      driftBankrunProgram,
      {
        authority: userB.wallet.publicKey,
        userTokenAccount: userB.usdcAccount,
      },
      {
        marketIndex: USDC_MARKET_INDEX,
        amount: USER_B_USDC_DEPOSIT,
        subAccountId: 0,
        reduceOnly: false,
        remainingOracles: [],
        remainingMarkets: [usdcSpotMarket],
      }
    );

    const tx = new Transaction().add(depositIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [userB.wallet],
      false,
      true
    );

    const userBalanceAfter = await getTokenBalance(
      bankRunProvider,
      userB.usdcAccount
    );
    const spotMarketAfter = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );

    assert.equal(
      userBalanceBefore - userBalanceAfter,
      USER_B_USDC_DEPOSIT.toNumber()
    );

    const depositIncrease = spotMarketAfter.depositBalance.sub(
      spotMarketBefore.depositBalance
    );
    const expectedScaledAmount = USER_B_USDC_DEPOSIT.mul(USDC_SCALING_FACTOR);

    assert.ok(depositIncrease.eq(expectedScaledAmount));

    const positions = await getUserPositions(
      driftBankrunProgram,
      userB.wallet.publicKey,
      0
    );
    const usdcPosition = positions.find(
      (p: any) => p.marketIndex === USDC_MARKET_INDEX
    );
    assert.ok(usdcPosition);
  });

  it("(user A) borrows USDC against Token A collateral", async () => {
    // User A borrows USDC using Token A as collateral
    // Amount: 10,000 USDC = $10,000 @ $1/USDC
    // Source: User B's USDC liquidity pool
    // LTV: 20% ($10k debt / $50k collateral)

    const userUsdcBalanceBefore = await getTokenBalance(
      bankRunProvider,
      userA.usdcAccount
    );
    const usdcMarketBefore = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );

    const [usdcSpotMarket] = deriveSpotMarketPDA(
      driftBankrunProgram.programId,
      USDC_MARKET_INDEX
    );
    const [tokenASpotMarket] = deriveSpotMarketPDA(
      driftBankrunProgram.programId,
      TOKEN_A_MARKET_INDEX
    );
    const borrowIx = await makeWithdrawIx(
      driftBankrunProgram,
      {
        authority: userA.wallet.publicKey,
        userTokenAccount: userA.usdcAccount,
      },
      {
        marketIndex: USDC_MARKET_INDEX,
        amount: USER_A_USDC_BORROW,
        subAccountId: 0,
        reduceOnly: false,
        remainingOracles: [driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE)!],
        remainingMarkets: [usdcSpotMarket, tokenASpotMarket],
      }
    );

    const tx = new Transaction().add(borrowIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [userA.wallet],
      false,
      true
    );

    const userUsdcBalanceAfter = await getTokenBalance(
      bankRunProvider,
      userA.usdcAccount
    );
    const usdcMarketAfter = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );

    assert.equal(
      userUsdcBalanceAfter - userUsdcBalanceBefore,
      USER_A_USDC_BORROW.toNumber()
    );

    const borrowIncrease = usdcMarketAfter.borrowBalance.sub(
      usdcMarketBefore.borrowBalance
    );
    const expectedScaledBorrowAmount =
      USER_A_USDC_BORROW.mul(USDC_SCALING_FACTOR);

    // Allow for small rounding differences due to Drift's interest accrual
    assertBNApproximately(
      borrowIncrease,
      expectedScaledBorrowAmount,
      new BN(1)
    );

    const positions = await getUserPositions(
      driftBankrunProgram,
      userA.wallet.publicKey,
      0
    );
    assert.equal(positions.length, 2);

    const usdcPosition = positions.find(
      (p: any) => p.marketIndex === USDC_MARKET_INDEX
    );
    const tokenAPosition = positions.find(
      (p: any) => p.marketIndex === TOKEN_A_MARKET_INDEX
    );

    assert.ok(usdcPosition);
    assert.ok(isDriftPositionBorrow(usdcPosition));

    assert.ok(tokenAPosition);
    assert.ok(isDriftPositionDeposit(tokenAPosition));
  });

  it("(user B) borrows Token A against USDC collateral", async () => {
    // User B borrows Token A using USDC as collateral
    // Amount: 500 TKA = $5,000 @ $10/TKA
    // Source: User A's Token A liquidity pool
    // LTV: 5% ($5k debt / $100k collateral)

    const userTokenABalanceBefore = await getTokenBalance(
      bankRunProvider,
      userB.tokenAAccount
    );
    const tokenAMarketBefore = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );

    const [usdcSpotMarket] = deriveSpotMarketPDA(
      driftBankrunProgram.programId,
      USDC_MARKET_INDEX
    );
    const [tokenASpotMarket] = deriveSpotMarketPDA(
      driftBankrunProgram.programId,
      TOKEN_A_MARKET_INDEX
    );
    const borrowIx = await makeWithdrawIx(
      driftBankrunProgram,
      {
        authority: userB.wallet.publicKey,
        userTokenAccount: userB.tokenAAccount,
      },
      {
        marketIndex: TOKEN_A_MARKET_INDEX,
        amount: USER_B_TOKEN_A_BORROW,
        subAccountId: 0,
        reduceOnly: false,
        remainingOracles: [driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE)!],
        remainingMarkets: [tokenASpotMarket, usdcSpotMarket],
      }
    );

    const tx = new Transaction().add(borrowIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [userB.wallet],
      false,
      true
    );

    const userTokenABalanceAfter = await getTokenBalance(
      bankRunProvider,
      userB.tokenAAccount
    );
    const tokenAMarketAfter = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );

    assert.equal(
      userTokenABalanceAfter - userTokenABalanceBefore,
      USER_B_TOKEN_A_BORROW.toNumber()
    );

    const borrowIncrease = tokenAMarketAfter.borrowBalance.sub(
      tokenAMarketBefore.borrowBalance
    );
    const expectedScaledBorrowAmount = USER_B_TOKEN_A_BORROW.mul(
      TOKEN_A_SCALING_FACTOR
    );

    // Allow for small rounding differences
    assertBNApproximately(
      borrowIncrease,
      expectedScaledBorrowAmount,
      new BN(1)
    );

    const positions = await getUserPositions(
      driftBankrunProgram,
      userB.wallet.publicKey,
      0
    );
    assert.equal(positions.length, 2);

    const usdcPosition = positions.find(
      (p: any) => p.marketIndex === USDC_MARKET_INDEX
    );
    const tokenAPosition = positions.find(
      (p: any) => p.marketIndex === TOKEN_A_MARKET_INDEX
    );

    assert.ok(usdcPosition);
    assert.ok(isDriftPositionDeposit(usdcPosition));

    assert.ok(tokenAPosition);
    assert.ok(isDriftPositionBorrow(tokenAPosition));
  });

  it("Verify spot markets are configured for borrowing", async () => {
    const usdcMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );

    assert.ok(usdcMarket.initialAssetWeight > 0);
    assert.ok(usdcMarket.initialLiabilityWeight > 0);
    assert.ok(usdcMarket.optimalBorrowRate > 0);
    assert.ok(usdcMarket.maxBorrowRate > 0);

    const tokenAMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );

    assert.ok(tokenAMarket.initialAssetWeight > 0);
    assert.ok(tokenAMarket.initialLiabilityWeight > 0);
    assert.ok(tokenAMarket.optimalBorrowRate > 0);
    assert.ok(tokenAMarket.maxBorrowRate > 0);

    const tokenAOracle = driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE);
    assert.ok(tokenAOracle);
  });

  it("Verify both markets have active deposits and borrows", async () => {
    const usdcMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );

    assert.ok(usdcMarket.depositBalance.gt(new BN(0)));
    assert.ok(usdcMarket.borrowBalance.gt(new BN(0)));
    assert.ok(usdcMarket.depositBalance.gt(usdcMarket.borrowBalance));

    const tokenAMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );

    assert.ok(tokenAMarket.depositBalance.gt(new BN(0)));
    assert.ok(tokenAMarket.borrowBalance.gt(new BN(0)));
    assert.ok(tokenAMarket.depositBalance.gt(tokenAMarket.borrowBalance));
  });

  it("Verify interest accumulation on both markets", async () => {
    const usdcMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      USDC_MARKET_INDEX
    );
    const usdcUtilization = calculateUtilizationRate(
      usdcMarket.depositBalance,
      usdcMarket.borrowBalance
    );

    assert.ok(usdcUtilization > 0 && usdcUtilization <= 1000000);

    const usdcInterestRate = calculateInterestRate(
      usdcUtilization,
      usdcMarket.optimalUtilization,
      usdcMarket.optimalBorrowRate,
      usdcMarket.maxBorrowRate
    );

    assert.ok(usdcInterestRate > 0);

    const tokenAMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_A_MARKET_INDEX
    );
    const tokenAUtilization = calculateUtilizationRate(
      tokenAMarket.depositBalance,
      tokenAMarket.borrowBalance
    );

    assert.ok(tokenAUtilization > 0 && tokenAUtilization <= 1000000);

    const tokenAInterestRate = calculateInterestRate(
      tokenAUtilization,
      tokenAMarket.optimalUtilization,
      tokenAMarket.optimalBorrowRate,
      tokenAMarket.maxBorrowRate
    );

    assert.ok(tokenAInterestRate > 0);

    assert.ok(usdcMarket.cumulativeDepositInterest.gt(new BN(0)));
    assert.ok(usdcMarket.cumulativeBorrowInterest.gt(new BN(0)));
    assert.ok(tokenAMarket.cumulativeDepositInterest.gt(new BN(0)));
    assert.ok(tokenAMarket.cumulativeBorrowInterest.gt(new BN(0)));
  });
});
