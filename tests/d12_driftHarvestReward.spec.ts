import {
  PublicKey,
  Transaction,
  Keypair,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import {
  ecosystem,
  driftAccounts,
  DRIFT_TOKEN_A_BANK,
  DRIFT_USDC_BANK,
  DRIFT_TOKEN_A_SPOT_MARKET,
  users,
  bankrunContext,
  bankrunProgram,
  driftBankrunProgram,
  oracles,
  groupAdmin,
  globalProgramAdmin,
  banksClient,
  driftGroup,
  DRIFT_TOKEN_A_PULL_ORACLE,
  globalFeeWallet,
  bankRunProvider,
} from "./rootHooks";
import { assert } from "chai";
import { processBankrunTransaction } from "./utils/tools";
import {
  makeDriftDepositIx,
  makeDriftHarvestRewardIx,
  makeDriftWithdrawIx,
} from "./utils/drift-instructions";
import {
  assertBankrunTxFailed,
  assertBNEqual,
  assertKeysEqual,
  getTokenBalance,
} from "./utils/genericTests";
import {
  ORACLE_CONF_INTERVAL,
  DRIFT_ORACLE_RECEIVER_PROGRAM_ID,
} from "./utils/types";
import {
  makeInitializeSpotMarketIx,
  makeAdminDepositIx,
} from "./utils/drift-sdk";
import { createBankrunPythOracleAccount } from "./utils/bankrun-oracles";
import { deriveSpotMarketPDA } from "./utils/pdas";
import {
  getSpotMarketAccount,
  getDriftStateAccount,
  defaultSpotMarketConfig,
  DriftOracleSourceValues,
  TOKEN_A_MARKET_INDEX,
  getDriftUserAccount,
  TOKEN_B_SCALING_FACTOR,
} from "./utils/drift-utils";
import { setPythPullOraclePrice } from "./utils/bankrun-oracles";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
  createMintToInstruction,
} from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
import {
  composeRemainingAccounts,
  accountInit,
} from "./utils/user-instructions";

const DRIFT_TOKEN_B_SPOT_MARKET = "drift_token_b_spot_market";
const DRIFT_TOKEN_B_PULL_ORACLE = "drift_token_b_pull_oracle";
const DRIFT_TOKEN_B_PULL_FEED = "drift_token_b_pull_feed";
const depositBAmount = new BN(50 * 10 ** ecosystem.tokenBDecimals);

describe("d12: Drift Harvest Reward", () => {
  let driftTokenABank: PublicKey;
  let driftUsdcBank: PublicKey;

  // New for this test
  let driftTokenBSpotMarket: PublicKey;
  let driftTokenBPullOracle: Keypair;
  let driftTokenBPullFeed: Keypair;
  let TOKEN_B_MARKET_INDEX: number;

  before(async () => {
    driftTokenABank = driftAccounts.get(DRIFT_TOKEN_A_BANK);
    driftUsdcBank = driftAccounts.get(DRIFT_USDC_BANK);

    const driftState = await getDriftStateAccount(driftBankrunProgram);
    TOKEN_B_MARKET_INDEX = driftState.numberOfSpotMarkets;

    await createIntermediaryTokenAccountIfNeeded(
      driftTokenABank,
      ecosystem.tokenBMint.publicKey
    );
    await createGlobalFeeWalletTokenAccount(ecosystem.tokenBMint.publicKey);
  });

  it("Setup: Initialize Token B spot market", async () => {
    const config = defaultSpotMarketConfig();

    const [spotMarketPDA] = deriveSpotMarketPDA(
      driftBankrunProgram.programId,
      TOKEN_B_MARKET_INDEX
    );

    driftTokenBSpotMarket = spotMarketPDA;
    driftAccounts.set(DRIFT_TOKEN_B_SPOT_MARKET, spotMarketPDA);

    driftTokenBPullOracle = Keypair.generate();
    driftTokenBPullFeed = Keypair.generate();

    await createBankrunPythOracleAccount(
      bankrunContext,
      banksClient,
      driftTokenBPullOracle,
      DRIFT_ORACLE_RECEIVER_PROGRAM_ID
    );

    driftAccounts.set(
      DRIFT_TOKEN_B_PULL_ORACLE,
      driftTokenBPullOracle.publicKey
    );
    driftAccounts.set(DRIFT_TOKEN_B_PULL_FEED, driftTokenBPullFeed.publicKey);

    await setPythPullOraclePrice(
      bankrunContext,
      banksClient,
      driftTokenBPullOracle.publicKey,
      driftTokenBPullFeed.publicKey,
      oracles.tokenBPrice,
      ecosystem.tokenBDecimals,
      ORACLE_CONF_INTERVAL,
      new PublicKey("FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH")
    );

    const initTokenBMarketIx = await makeInitializeSpotMarketIx(
      driftBankrunProgram,
      {
        admin: groupAdmin.wallet.publicKey,
        spotMarketMint: ecosystem.tokenBMint.publicKey,
        oracle: driftTokenBPullOracle.publicKey,
      },
      {
        optimalUtilization: config.optimalUtilization,
        optimalRate: config.optimalRate,
        maxRate: config.maxRate,
        oracleSource: DriftOracleSourceValues.pythPull,
        initialAssetWeight: config.initialAssetWeight,
        maintenanceAssetWeight: config.maintenanceAssetWeight,
        initialLiabilityWeight: config.initialLiabilityWeight,
        maintenanceLiabilityWeight: config.maintenanceLiabilityWeight,
        marketIndex: TOKEN_B_MARKET_INDEX,
      }
    );

    const tx = new Transaction().add(initTokenBMarketIx);
    await processBankrunTransaction(
      bankrunContext,
      tx,
      [groupAdmin.wallet],
      false,
      true
    );

    const tokenBMarket = await getSpotMarketAccount(
      driftBankrunProgram,
      TOKEN_B_MARKET_INDEX
    );
    assert.ok(tokenBMarket);
    assert.equal(tokenBMarket.marketIndex, TOKEN_B_MARKET_INDEX);
    assertKeysEqual(tokenBMarket.mint, ecosystem.tokenBMint.publicKey);
    assertKeysEqual(tokenBMarket.oracle, driftTokenBPullOracle.publicKey);
    assert.deepStrictEqual(
      tokenBMarket.oracleSource,
      DriftOracleSourceValues.pythPull
    );

    const state = await getDriftStateAccount(driftBankrunProgram);
    assert.equal(state.numberOfSpotMarkets, TOKEN_B_MARKET_INDEX + 1);
  });

  it("Admin: Add deposits to Token A bank's drift user", async () => {
    const tokenABank = await bankrunProgram.account.bank.fetch(driftTokenABank);
    const driftUser = tokenABank.driftUser;

    const driftAdmin = groupAdmin.wallet;

    const adminTokenBAccount = getAssociatedTokenAddressSync(
      ecosystem.tokenBMint.publicKey,
      driftAdmin.publicKey
    );

    const createAdminAtaIx = createAssociatedTokenAccountIdempotentInstruction(
      globalProgramAdmin.wallet.publicKey,
      adminTokenBAccount,
      driftAdmin.publicKey,
      ecosystem.tokenBMint.publicKey
    );

    const mintAmount = new BN(100 * 10 ** ecosystem.tokenBDecimals);
    const mintToAdminIx = createMintToInstruction(
      ecosystem.tokenBMint.publicKey,
      adminTokenBAccount,
      globalProgramAdmin.wallet.publicKey,
      mintAmount.toNumber()
    );

    const fundTx = new Transaction().add(createAdminAtaIx).add(mintToAdminIx);

    await processBankrunTransaction(
      bankrunContext,
      fundTx,
      [globalProgramAdmin.wallet],
      false,
      true
    );

    const tokenBOracle = driftAccounts.get(DRIFT_TOKEN_B_PULL_ORACLE);
    const tokenBSpotMarket = driftAccounts.get(DRIFT_TOKEN_B_SPOT_MARKET);

    const remainingAccounts: PublicKey[] = [];
    if (tokenBOracle) {
      remainingAccounts.push(tokenBOracle);
    }
    if (tokenBSpotMarket) {
      remainingAccounts.push(tokenBSpotMarket);
    }

    const adminDepositIx = await makeAdminDepositIx(
      driftBankrunProgram,
      {
        admin: driftAdmin.publicKey,
        driftUser: driftUser,
        adminTokenAccount: adminTokenBAccount,
      },
      {
        marketIndex: TOKEN_B_MARKET_INDEX,
        amount: depositBAmount,
        remainingAccounts,
      }
    );

    const depositTx = new Transaction().add(adminDepositIx);

    await processBankrunTransaction(
      bankrunContext,
      depositTx,
      [driftAdmin],
      false,
      true
    );

    const driftUserAccount = await getDriftUserAccount(
      driftBankrunProgram,
      driftUser
    );
    const tokenBPosition = driftUserAccount.spotPositions[2];
    assertBNEqual(
      tokenBPosition.scaledBalance,
      depositBAmount.mul(TOKEN_B_SCALING_FACTOR)
    );
    assert.equal(tokenBPosition.marketIndex, TOKEN_B_MARKET_INDEX);
  });

  it("User: Can deposit and withdraw normally after admin deposit", async () => {
    const user = users[0];

    const throwawayAccount = Keypair.generate();

    const initAccountIx = await accountInit(user.mrgnBankrunProgram, {
      marginfiGroup: driftGroup.publicKey,
      marginfiAccount: throwawayAccount.publicKey,
      authority: user.wallet.publicKey,
      feePayer: user.wallet.publicKey,
    });

    const initTx = new Transaction().add(initAccountIx);
    await processBankrunTransaction(
      bankrunContext,
      initTx,
      [user.wallet, throwawayAccount],
      false,
      true
    );

    const tokenAOracle = driftAccounts.get(DRIFT_TOKEN_A_PULL_ORACLE);
    const tokenASpotMarket = driftAccounts.get(DRIFT_TOKEN_A_SPOT_MARKET);
    const tokenBOracle = driftAccounts.get(DRIFT_TOKEN_B_PULL_ORACLE);
    const tokenBSpotMarket = driftAccounts.get(DRIFT_TOKEN_B_SPOT_MARKET);

    const depositAmount = new BN(10 * 10 ** ecosystem.tokenADecimals);

    const balanceBefore = await getTokenBalance(
      bankRunProvider,
      user.tokenAAccount
    );

    // STEP 1: Deposit should work normally even with admin deposits present
    const depositIx = await makeDriftDepositIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: throwawayAccount.publicKey,
        bank: driftTokenABank,
        signerTokenAccount: user.tokenAAccount,
        driftOracle: tokenAOracle,
      },
      depositAmount,
      TOKEN_A_MARKET_INDEX
    );

    const depositTx = new Transaction()
      .add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }))
      .add(depositIx);

    await processBankrunTransaction(
      bankrunContext,
      depositTx,
      [user.wallet],
      false,
      true
    );

    const balanceAfterDeposit = await getTokenBalance(
      bankRunProvider,
      user.tokenAAccount
    );
    const deposited = new BN(balanceBefore - balanceAfterDeposit);
    assertBNEqual(deposited, depositAmount);

    // STEP 2: Try to withdraw without reward accounts - should fail
    const withdrawWithoutRewardsIx = await makeDriftWithdrawIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: throwawayAccount.publicKey,
        bank: driftTokenABank,
        destinationTokenAccount: user.tokenAAccount,
        driftOracle: tokenAOracle,
        // Missing reward accounts - this should cause failure
      },
      {
        amount: depositAmount,
        withdraw_all: false,
        remaining: composeRemainingAccounts([
          [driftTokenABank, tokenAOracle, tokenASpotMarket],
        ]),
      },
      driftBankrunProgram
    );

    const withdrawWithoutRewardsTx = new Transaction()
      .add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }))
      .add(withdrawWithoutRewardsIx);

    const withdrawWithoutRewardsResult = await processBankrunTransaction(
      bankrunContext,
      withdrawWithoutRewardsTx,
      [user.wallet],
      true,
      false
    );

    assertBankrunTxFailed(withdrawWithoutRewardsResult, 0x18b1); // DriftMissingRewardAccounts

    // STEP 3: Withdraw all with reward accounts - should succeed
    const withdrawAllIx = await makeDriftWithdrawIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: throwawayAccount.publicKey,
        bank: driftTokenABank,
        destinationTokenAccount: user.tokenAAccount,
        driftOracle: tokenAOracle,
        driftRewardOracle: tokenBOracle,
        driftRewardSpotMarket: tokenBSpotMarket,
      },
      {
        amount: new BN(0),
        withdraw_all: true,
        remaining: composeRemainingAccounts([
          [driftTokenABank, tokenAOracle, tokenASpotMarket],
        ]),
      },
      driftBankrunProgram
    );

    const withdrawAllTx = new Transaction()
      .add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }))
      .add(withdrawAllIx);

    await processBankrunTransaction(
      bankrunContext,
      withdrawAllTx,
      [user.wallet],
      false,
      true
    );

    // Verify we got back N-1 tokens due to rounding
    const balanceAfterWithdraw = await getTokenBalance(
      bankRunProvider,
      user.tokenAAccount
    );
    const withdrawn = new BN(balanceAfterWithdraw - balanceAfterDeposit);
    const expectedWithdrawn = depositAmount.sub(new BN(1));

    assertBNEqual(withdrawn, expectedWithdrawn);
  });

  it("Error: Try to harvest same market as bank's main market", async () => {
    const user = users[0];

    await createIntermediaryTokenAccountIfNeeded(
      driftTokenABank,
      ecosystem.tokenAMint.publicKey
    );
    await createGlobalFeeWalletTokenAccount(ecosystem.tokenAMint.publicKey);

    // Try to harvest Token A from Token A bank (harvest market same as bank's drift spot market)
    const tokenASpotMarket = driftAccounts.get(DRIFT_TOKEN_A_SPOT_MARKET);

    const remainingAccounts = [];

    const tokenAOracle = driftAccounts.get(DRIFT_TOKEN_A_PULL_ORACLE);
    if (tokenAOracle) {
      remainingAccounts.push({
        pubkey: tokenAOracle,
        isSigner: false,
        isWritable: false,
      });
    }

    if (tokenASpotMarket) {
      remainingAccounts.push({
        pubkey: tokenASpotMarket,
        isSigner: false,
        isWritable: true,
      });
    }

    const harvestIx = await makeDriftHarvestRewardIx(
      user.mrgnBankrunProgram,
      driftBankrunProgram,
      {
        bank: driftTokenABank,
        harvestDriftSpotMarket: tokenASpotMarket, // Same as bank's drift spot market!
      },
      remainingAccounts
    );

    const tx = new Transaction()
      .add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }))
      .add(harvestIx);

    const result = await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      true,
      true
    );

    assertBankrunTxFailed(result, 0x18ac); // DriftNoAdminDeposit - no admin deposits for Token A harvest
  });

  it("User 0: Harvest reward from drift position", async () => {
    const user = users[0];

    const tokenBOracle = driftAccounts.get(DRIFT_TOKEN_B_PULL_ORACLE);
    const tokenAOracle = driftAccounts.get(DRIFT_TOKEN_A_PULL_ORACLE);
    const tokenASpotMarket = driftAccounts.get(DRIFT_TOKEN_A_SPOT_MARKET);

    const remainingAccounts = [];

    if (tokenAOracle) {
      remainingAccounts.push({
        pubkey: tokenAOracle,
        isSigner: false,
        isWritable: false,
      });
    }

    if (tokenBOracle) {
      remainingAccounts.push({
        pubkey: tokenBOracle,
        isSigner: false,
        isWritable: false,
      });
    }

    if (tokenASpotMarket) {
      remainingAccounts.push({
        pubkey: tokenASpotMarket,
        isSigner: false,
        isWritable: true,
      });
    }

    remainingAccounts.push({
      pubkey: driftTokenBSpotMarket,
      isSigner: false,
      isWritable: true,
    });

    const harvestIx = await makeDriftHarvestRewardIx(
      user.mrgnBankrunProgram,
      driftBankrunProgram,
      {
        bank: driftTokenABank,
        harvestDriftSpotMarket: driftTokenBSpotMarket,
      },
      remainingAccounts
    );
    const destinationTokenAccount = harvestIx.keys.at(4).pubkey;

    const tx = new Transaction()
      .add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }))
      .add(harvestIx);

    await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      false,
      false
    );

    const userTokenBAfter = await getTokenBalance(bankRunProvider, destinationTokenAccount);
    assertBNEqual(depositBAmount, userTokenBAfter);
  });

  it("Error: Try to harvest from USDC bank (no admin deposits)", async () => {
    const user = users[0];

    await createIntermediaryTokenAccountIfNeeded(
      driftUsdcBank,
      ecosystem.tokenBMint.publicKey
    );

    const remainingAccounts = [];

    const usdcOracle = oracles.usdcOracle.publicKey;
    if (usdcOracle) {
      remainingAccounts.push({
        pubkey: usdcOracle,
        isSigner: false,
        isWritable: false,
      });
    }

    const tokenBOracle = driftAccounts.get(DRIFT_TOKEN_B_PULL_ORACLE);
    if (tokenBOracle) {
      remainingAccounts.push({
        pubkey: tokenBOracle,
        isSigner: false,
        isWritable: false,
      });
    }

    remainingAccounts.push({
      pubkey: driftTokenBSpotMarket,
      isSigner: false,
      isWritable: true,
    });

    const harvestIx = await makeDriftHarvestRewardIx(
      user.mrgnBankrunProgram,
      driftBankrunProgram,
      {
        bank: driftUsdcBank, // USDC bank has no admin deposits
        harvestDriftSpotMarket: driftTokenBSpotMarket,
      },
      remainingAccounts
    );

    const tx = new Transaction()
      .add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }))
      .add(harvestIx);

    const result = await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      true,
      false
    );

    // Should fail with DriftNoAdminDeposit since USDC bank has no admin deposits to harvest
    assertBankrunTxFailed(result, 0x18ac); // DriftNoAdminDeposit
  });
});

const createIntermediaryTokenAccountIfNeeded = async (
  bank: PublicKey,
  mint: PublicKey
) => {
  const [liquidityVaultAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from("liquidity_vault_auth"), bank.toBuffer()],
    bankrunProgram.programId
  );

  const intermediaryTokenAccount = getAssociatedTokenAddressSync(
    mint,
    liquidityVaultAuthority,
    true
  );

  const createAtaIx = createAssociatedTokenAccountIdempotentInstruction(
    groupAdmin.wallet.publicKey,
    intermediaryTokenAccount,
    liquidityVaultAuthority,
    mint
  );

  const tx = new Transaction().add(createAtaIx);
  await processBankrunTransaction(
    bankrunContext,
    tx,
    [groupAdmin.wallet],
    false,
    true
  );
};

const createGlobalFeeWalletTokenAccount = async (mint: PublicKey) => {
  const destinationAta = getAssociatedTokenAddressSync(mint, globalFeeWallet);

  const tx = new Transaction().add(
    createAssociatedTokenAccountIdempotentInstruction(
      groupAdmin.wallet.publicKey,
      destinationAta,
      globalFeeWallet,
      mint
    )
  );

  await processBankrunTransaction(
    bankrunContext,
    tx,
    [groupAdmin.wallet],
    false,
    true
  );
};

/*
TODO: Add tests for second reward account and bricked account scenarios

Helper Functions to Create:
1. createDriftSpotMarketWithOracle(tokenMint, tokenSymbol, marketIndex, price, decimals)
   - Create oracle keypairs
   - Set up Pyth pull oracle with price
   - Initialize spot market with default config
   - Store oracle/market in driftAccounts map
   - Return spot market pubkey

2. fundAndDepositAdminReward(driftAdmin, tokenMint, marketIndex, amount)
   - Create/get admin's token account
   - Mint tokens to admin
   - Build remaining accounts (oracle + spot market)
   - Execute admin deposit
   - Return success

3. createThrowawayMarginfiAccount(user, group)
   - Generate new keypair
   - Initialize marginfi account
   - Return account pubkey

Tests to Add:
1. "User: Cannot withdraw with 3 active deposits without second reward accounts"
   - Setup Token C as second admin deposit
   - Try withdraw with only first reward accounts
   - Should fail with DriftMissingRewardAccounts (0x18b1)

2. "User: Can withdraw with 3 active deposits with both reward accounts"
   - Withdraw with both sets of reward accounts provided
   - Should succeed with N-1 tokens due to rounding

3. "User: Account bricked with 4 active deposits"
   - Setup Token D as third admin deposit
   - Try withdraw even with both reward accounts
   - Should fail with DriftBrickedAccount (0x18b2)

This would reduce setup from ~90 lines per token to ~10 lines using helpers.
Current setup is sufficient for main use case (2 active deposits).
*/
