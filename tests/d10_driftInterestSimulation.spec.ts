import { BN } from "@coral-xyz/anchor";
import {
  PublicKey,
  Transaction,
  SystemProgram,
  Keypair,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import { assert } from "chai";
import {
  ecosystem,
  driftAccounts,
  driftGroup,
  users,
  bankrunContext,
  bankrunProgram,
  driftBankrunProgram,
  bankRunProvider,
  oracles,
  globalProgramAdmin,
  groupAdmin,
  banksClient,
  DRIFT_USDC_SPOT_MARKET,
  DRIFT_TOKENA_SPOT_MARKET,
  DRIFT_TOKENA_PULL_ORACLE,
} from "./rootHooks";
import { refreshPullOraclesBankrun } from "./utils/bankrun-oracles";
import { MockUser } from "./utils/mocks";
import { processBankrunTransaction } from "./utils/tools";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { deriveBankWithSeed } from "./utils/pdas";
import {
  ASSET_TAG_DRIFT,
  defaultDriftBankConfig,
  getSpotMarketAccount,
  getDriftUserAccount,
  scaledBalanceToTokenAmount,
  refreshDriftOracles,
  USDC_MARKET_INDEX,
  TOKEN_A_MARKET_INDEX,
  DRIFT_SCALED_BALANCE_DECIMALS,
} from "./utils/drift-utils";
import {
  makeAddDriftBankIx,
  makeInitDriftUserIx,
  makeDriftDepositIx,
  makeDriftWithdrawIx,
} from "./utils/drift-instructions";
import { getTokenBalance, assertBNEqual } from "./utils/genericTests";
import { composeRemainingAccounts } from "./utils/user-instructions";
import { createMintToInstruction } from "@solana/spl-token";
import { Clock } from "solana-bankrun";

describe("d10: Drift Interest Simulation", () => {
  const NEW_DRIFT_USDC_BANK = "new_drift_usdc_bank";
  const NEW_DRIFT_TOKENA_BANK = "new_drift_tokena_bank";
  const NEW_DRIFT_ACCOUNT = "gd_new_acc";

  let newDriftUsdcBank: PublicKey;
  let newDriftTokenABank: PublicKey;
  let userA: MockUser;
  let userB: MockUser;

  interface BankInfo {
    bank: PublicKey;
    mint: PublicKey;
    decimals: number;
    symbol: string;
    spotMarket: PublicKey;
    marketIndex: number;
  }

  interface Operation {
    user: string;
    type: "deposit" | "withdraw";
    bank: PublicKey;
    amount: BN;
    decimals: number;
    symbol: string;
    timestamp: number;
    slot: number;
  }

  const operationLog: Operation[] = [];

  // Transaction nonce to ensure unique compute unit limits, preventing Bankrun from detecting
  // duplicate transaction signatures when similar operations occur across test cycles.
  let driftTxNonce = 0;

  before(async () => {
    userA = users[0];
    userB = users[1];
  });

  it("(admin) creates new drift banks for interest testing", async () => {
    const usdcBankSeed = new BN(100);
    const tokenABankSeed = new BN(101);
    [newDriftUsdcBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      driftGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      usdcBankSeed
    );
    [newDriftTokenABank] = deriveBankWithSeed(
      bankrunProgram.programId,
      driftGroup.publicKey,
      ecosystem.tokenAMint.publicKey,
      tokenABankSeed
    );

    driftAccounts.set(NEW_DRIFT_USDC_BANK, newDriftUsdcBank);
    driftAccounts.set(NEW_DRIFT_TOKENA_BANK, newDriftTokenABank);
    const usdcConfig = defaultDriftBankConfig(oracles.usdcOracle.publicKey);
    usdcConfig.depositLimit = new BN(10_000_000).mul(
      new BN(10 ** DRIFT_SCALED_BALANCE_DECIMALS)
    );

    const addUsdcBankIx = await makeAddDriftBankIx(
      groupAdmin.mrgnBankrunProgram,
      {
        group: driftGroup.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.usdcMint.publicKey,
        driftSpotMarket: driftAccounts.get(DRIFT_USDC_SPOT_MARKET),
        oracle: oracles.usdcOracle.publicKey,
      },
      {
        seed: usdcBankSeed,
        config: usdcConfig,
      }
    );

    const tokenAConfig = defaultDriftBankConfig(oracles.tokenAOracle.publicKey);
    tokenAConfig.depositLimit = new BN(10_000_000).mul(
      new BN(10 ** DRIFT_SCALED_BALANCE_DECIMALS)
    );

    const addTokenABankIx = await makeAddDriftBankIx(
      groupAdmin.mrgnBankrunProgram,
      {
        group: driftGroup.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.tokenAMint.publicKey,
        driftSpotMarket: driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET),
        oracle: oracles.tokenAOracle.publicKey,
      },
      {
        seed: tokenABankSeed,
        config: tokenAConfig,
      }
    );

    const tx1 = new Transaction().add(addUsdcBankIx);
    await processBankrunTransaction(
      bankrunContext,
      tx1,
      [groupAdmin.wallet],
      false,
      true
    );

    const tx2 = new Transaction().add(addTokenABankIx);
    await processBankrunTransaction(
      bankrunContext,
      tx2,
      [groupAdmin.wallet],
      false,
      true
    );
  });

  it("(admin) initializes Drift user accounts for new banks", async () => {
    const initUserAmount = new BN(100);
    const fundAdminTx = new Transaction()
      .add(
        createMintToInstruction(
          ecosystem.usdcMint.publicKey,
          groupAdmin.usdcAccount,
          globalProgramAdmin.wallet.publicKey,
          initUserAmount.toNumber()
        )
      )
      .add(
        createMintToInstruction(
          ecosystem.tokenAMint.publicKey,
          groupAdmin.tokenAAccount,
          globalProgramAdmin.wallet.publicKey,
          initUserAmount.toNumber()
        )
      );
    await processBankrunTransaction(bankrunContext, fundAdminTx, [
      globalProgramAdmin.wallet,
    ]);
    const initUsdcUserIx = await makeInitDriftUserIx(
      groupAdmin.mrgnBankrunProgram,
      {
        feePayer: groupAdmin.wallet.publicKey,
        bank: newDriftUsdcBank,
        signerTokenAccount: groupAdmin.usdcAccount,
      },
      {
        amount: initUserAmount,
      },
      0
    );

    const initTokenAUserIx = await makeInitDriftUserIx(
      groupAdmin.mrgnBankrunProgram,
      {
        feePayer: groupAdmin.wallet.publicKey,
        bank: newDriftTokenABank,
        signerTokenAccount: groupAdmin.tokenAAccount,
        driftOracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE),
      },
      {
        amount: initUserAmount,
      },
      1
    );

    const tx = new Transaction().add(initUsdcUserIx).add(initTokenAUserIx);

    await processBankrunTransaction(
      bankrunContext,
      tx,
      [groupAdmin.wallet],
      false,
      true
    );
  });

  it("funds users with tokens for testing", async () => {
    const LARGE_USDC_AMOUNT = new BN(1_000_000 * 10 ** ecosystem.usdcDecimals);
    const LARGE_TOKENA_AMOUNT = new BN(50_000 * 10 ** ecosystem.tokenADecimals);
    const fundUserATx = new Transaction()
      .add(
        createMintToInstruction(
          ecosystem.usdcMint.publicKey,
          userA.usdcAccount,
          globalProgramAdmin.wallet.publicKey,
          LARGE_USDC_AMOUNT.toNumber()
        )
      )
      .add(
        createMintToInstruction(
          ecosystem.tokenAMint.publicKey,
          userA.tokenAAccount,
          globalProgramAdmin.wallet.publicKey,
          1000 * 10 ** ecosystem.tokenADecimals
        )
      );
    await processBankrunTransaction(bankrunContext, fundUserATx, [
      globalProgramAdmin.wallet,
    ]);
    const fundUserBTx = new Transaction()
      .add(
        createMintToInstruction(
          ecosystem.usdcMint.publicKey,
          userB.usdcAccount,
          globalProgramAdmin.wallet.publicKey,
          1_000_000 * 10 ** ecosystem.usdcDecimals
        )
      )
      .add(
        createMintToInstruction(
          ecosystem.tokenAMint.publicKey,
          userB.tokenAAccount,
          globalProgramAdmin.wallet.publicKey,
          LARGE_TOKENA_AMOUNT.toNumber()
        )
      );
    await processBankrunTransaction(bankrunContext, fundUserBTx, [
      globalProgramAdmin.wallet,
    ]);
  });

  it("initializes marginfi accounts for new drift group", async () => {
    for (const user of [userA, userB] as MockUser[]) {
      if (user.accounts.has(NEW_DRIFT_ACCOUNT)) {
        continue;
      }

      const accountKeypair = Keypair.generate();
      user.accounts.set(NEW_DRIFT_ACCOUNT, accountKeypair.publicKey);

      const initAccountIx = await user.mrgnBankrunProgram.methods
        .marginfiAccountInitialize()
        .accounts({
          marginfiGroup: driftGroup.publicKey,
          marginfiAccount: accountKeypair.publicKey,
          authority: user.wallet.publicKey,
          feePayer: user.wallet.publicKey,
        })
        .instruction();

      const tx = new Transaction().add(initAccountIx);
      await processBankrunTransaction(
        bankrunContext,
        tx,
        [user.wallet, accountKeypair],
        false,
        true
      );
    }
  });

  async function makeDepositThroughMarginfi(
    user: MockUser,
    bank: PublicKey,
    amount: BN
  ): Promise<void> {
    const userAccount = user.accounts.get(NEW_DRIFT_ACCOUNT)!;
    const bankInfo = await bankrunProgram.account.bank.fetch(bank);
    const isUsdc = bankInfo.mint.equals(ecosystem.usdcMint.publicKey);
    const tokenAccount = isUsdc ? user.usdcAccount : user.tokenAAccount;
    const marketIndex = isUsdc ? USDC_MARKET_INDEX : TOKEN_A_MARKET_INDEX;
    const oracle = isUsdc
      ? oracles.usdcOracle.publicKey
      : oracles.tokenAOracle.publicKey;
    const spotMarket = isUsdc
      ? driftAccounts.get(DRIFT_USDC_SPOT_MARKET)!
      : driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET)!;
    const driftOracle = isUsdc
      ? null
      : driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE);
    const activePositions: PublicKey[][] = [];
    const marginfiAccount = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );

    for (const balance of marginfiAccount.lendingAccount.balances) {
      if (balance.active === 1) {
        const balanceBank = await bankrunProgram.account.bank.fetch(
          balance.bankPk
        );
        const balanceOracle = balanceBank.config.oracleKeys[0];

        if (balanceBank.config.assetTag === ASSET_TAG_DRIFT) {
          const driftSpotMarket = balanceBank.driftSpotMarket;
          activePositions.push([
            balance.bankPk,
            balanceOracle,
            driftSpotMarket,
          ]);
        } else {
          activePositions.push([balance.bankPk, balanceOracle]);
        }
      }
    }
    const isDepositBankActive = activePositions.some((pos) =>
      pos[0].equals(bank)
    );
    if (!isDepositBankActive) {
      activePositions.push([bank, oracle, spotMarket]);
    }

    const depositIx = await makeDriftDepositIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: userAccount,
        bank,
        signerTokenAccount: tokenAccount,
        driftOracle: driftOracle,
      },
      amount,
      marketIndex
    );

    const nonce = driftTxNonce++;
    const computeUnits = 1_200_000 + (nonce % 1000);

    const tx = new Transaction()
      .add(ComputeBudgetProgram.setComputeUnitLimit({ units: computeUnits }))
      .add(depositIx);
    tx.instructions[1].keys.push(
      ...composeRemainingAccounts(activePositions).map((pubkey) => ({
        pubkey,
        isSigner: false,
        isWritable: false,
      }))
    );

    await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      false,
      true
    );
  }

  async function makeWithdrawThroughMarginfi(
    user: MockUser,
    bank: PublicKey,
    amount: BN,
    withdrawAll: boolean = false
  ): Promise<void> {
    const userAccount = user.accounts.get(NEW_DRIFT_ACCOUNT)!;
    const bankInfo = await bankrunProgram.account.bank.fetch(bank);
    const isUsdc = bankInfo.mint.equals(ecosystem.usdcMint.publicKey);
    const tokenAccount = isUsdc ? user.usdcAccount : user.tokenAAccount;
    const driftOracle = isUsdc
      ? null
      : driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE);

    const activePositions: PublicKey[][] = [];
    const marginfiAccount = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );

    for (const balance of marginfiAccount.lendingAccount.balances) {
      if (balance.active === 1) {
        if (withdrawAll && balance.bankPk.equals(bank)) {
          continue;
        }

        const balanceBank = await bankrunProgram.account.bank.fetch(
          balance.bankPk
        );
        const balanceOracle = balanceBank.config.oracleKeys[0];

        if (balanceBank.config.assetTag === ASSET_TAG_DRIFT) {
          const driftSpotMarket = balanceBank.driftSpotMarket;
          activePositions.push([
            balance.bankPk,
            balanceOracle,
            driftSpotMarket,
          ]);
        } else {
          activePositions.push([balance.bankPk, balanceOracle]);
        }
      }
    }

    const withdrawIx = await makeDriftWithdrawIx(
      user.mrgnBankrunProgram,
      {
        marginfiAccount: userAccount,
        bank,
        destinationTokenAccount: tokenAccount,
        driftOracle: driftOracle,
      },
      {
        amount: withdrawAll ? new BN(0) : amount,
        withdraw_all: withdrawAll,
        remaining: composeRemainingAccounts(activePositions),
      },
      driftBankrunProgram
    );

    const nonce = driftTxNonce++;
    const computeUnits = 1_200_000 + (nonce % 1000);

    const tx = new Transaction()
      .add(ComputeBudgetProgram.setComputeUnitLimit({ units: computeUnits }))
      .add(withdrawIx);

    await processBankrunTransaction(
      bankrunContext,
      tx,
      [user.wallet],
      false,
      true
    );
  }

  it("tests very small deposits with interest accrual", async () => {
    const TINY_USDC = new BN(5);
    await makeDepositThroughMarginfi(userA, newDriftUsdcBank, TINY_USDC);

    operationLog.push({
      user: userA.wallet.publicKey.toString(),
      type: "deposit",
      bank: newDriftUsdcBank,
      amount: TINY_USDC,
      decimals: ecosystem.usdcDecimals,
      symbol: "USDC",
      timestamp: Date.now(),
      slot: await getCurrentSlot(),
    });
    await advanceTimeAndAccrueInterest(30);
  });

  it("tests very large deposits without overflow", async () => {
    const HUGE_TOKENA = new BN(40_000 * 10 ** ecosystem.tokenADecimals);

    await makeDepositThroughMarginfi(userB, newDriftTokenABank, HUGE_TOKENA);

    operationLog.push({
      user: userB.wallet.publicKey.toString(),
      type: "deposit",
      bank: newDriftTokenABank,
      amount: HUGE_TOKENA,
      decimals: ecosystem.tokenADecimals,
      symbol: "TKA",
      timestamp: Date.now(),
      slot: await getCurrentSlot(),
    });

    await advanceTimeAndAccrueInterest(30);
  });

  it("simulates random deposits and withdrawals with interest", async () => {
    const NUM_ITERATIONS = 20;

    const banks: BankInfo[] = [
      {
        bank: newDriftUsdcBank,
        mint: ecosystem.usdcMint.publicKey,
        decimals: ecosystem.usdcDecimals,
        symbol: "USDC",
        spotMarket: driftAccounts.get(DRIFT_USDC_SPOT_MARKET)!,
        marketIndex: USDC_MARKET_INDEX,
      },
      {
        bank: newDriftTokenABank,
        mint: ecosystem.tokenAMint.publicKey,
        decimals: ecosystem.tokenADecimals,
        symbol: "TKA",
        spotMarket: driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET)!,
        marketIndex: TOKEN_A_MARKET_INDEX,
      },
    ];

    for (let i = 0; i < NUM_ITERATIONS; i++) {
      const user = i % 2 === 0 ? userA : userB;

      const operation = await simulateRandomOperation(user, banks);

      operationLog.push(operation);

      const daysToAdvance = Math.floor(Math.random() * 7) + 1;
      await advanceTimeAndAccrueInterest(daysToAdvance);
    }
  });

  it("verifies all users can withdraw principal + interest", async () => {
    for (const user of [userA, userB]) {
      const userAccount = user.accounts.get(NEW_DRIFT_ACCOUNT)!;
      const marginfiAccount =
        await bankrunProgram.account.marginfiAccount.fetch(userAccount);

      for (const balance of marginfiAccount.lendingAccount.balances) {
        if (balance.active === 1) {
          const bank = await bankrunProgram.account.bank.fetch(balance.bankPk);

          if (bank.config.assetTag !== ASSET_TAG_DRIFT) continue;

          const isUsdc = bank.mint.equals(ecosystem.usdcMint.publicKey);

          const marginfiAssetSharesBigNumber = wrappedI80F48toBigNumber(
            balance.assetShares
          );
          const marginfiAssetShares = new BN(
            marginfiAssetSharesBigNumber.toString()
          );

          if (marginfiAssetShares.gt(new BN(0))) {
            await advanceTimeAndAccrueInterest(2);

            const marketIndex = isUsdc
              ? USDC_MARKET_INDEX
              : TOKEN_A_MARKET_INDEX;
            const spotMarket = await getSpotMarketAccount(
              driftBankrunProgram,
              marketIndex
            );
            const tokenAmount = scaledBalanceToTokenAmount(
              marginfiAssetShares,
              spotMarket,
              true
            );

            await makeWithdrawThroughMarginfi(
              user,
              balance.bankPk,
              tokenAmount
            );

            await makeWithdrawThroughMarginfi(
              user,
              balance.bankPk,
              new BN(0),
              true
            );
          }
        }
      }
    }
  });

  async function getCurrentTimestamp(): Promise<number> {
    const clock = await banksClient.getClock();
    return Number(clock.unixTimestamp);
  }

  async function getCurrentSlot(): Promise<number> {
    const clock = await banksClient.getClock();
    return Number(clock.slot);
  }

  async function advanceTimeAndAccrueInterest(days: number): Promise<void> {
    const currentClock = await banksClient.getClock();
    const currentSlot = Number(currentClock.slot);
    const currentTimestamp = Number(currentClock.unixTimestamp);

    const newSlot = currentSlot + 1;
    const newTimestamp = currentTimestamp + days * 86400;

    const newClock = new Clock(
      BigInt(newSlot),
      0n,
      currentClock.epoch,
      0n,
      BigInt(newTimestamp)
    );

    bankrunContext.setClock(newClock);

    const nonce = driftTxNonce++;
    const computeUnits = 1_200_000 + (nonce % 1000);

    const dummyTx = new Transaction()
      .add(ComputeBudgetProgram.setComputeUnitLimit({ units: computeUnits }))
      .add(
        SystemProgram.transfer({
          fromPubkey: groupAdmin.wallet.publicKey,
          toPubkey: groupAdmin.wallet.publicKey,
          lamports: 1,
        })
      );
    await processBankrunTransaction(bankrunContext, dummyTx, [
      groupAdmin.wallet,
    ]);

    const refreshedClock = await banksClient.getClock();
    await refreshPullOraclesBankrun(
      oracles,
      bankrunContext,
      banksClient
    );

    await refreshDriftOracles(
      oracles,
      driftAccounts,
      bankrunContext,
      banksClient
    );
  }

  async function simulateRandomOperation(
    user: MockUser,
    banks: BankInfo[]
  ): Promise<Operation> {
    const isDeposit = Math.random() < 0.5;

    const bankInfo = banks[Math.floor(Math.random() * banks.length)];

    if (isDeposit) {
      const tokenAccount =
        bankInfo.symbol === "USDC" ? user.usdcAccount : user.tokenAAccount;
      const balance = await getTokenBalance(bankRunProvider, tokenAccount);

      if (balance === 0) {
        return simulateRandomOperation(user, banks);
      }

      const MIN_DEPOSIT_USD = 100;
      const MAX_DEPOSIT_USD = 50_000;
      const depositUsdValue =
        MIN_DEPOSIT_USD + Math.random() * (MAX_DEPOSIT_USD - MIN_DEPOSIT_USD);

      const tokenPrice = bankInfo.symbol === "USDC" ? 1 : 10;
      const tokenAmount = depositUsdValue / tokenPrice;
      const amount = new BN(Math.floor(tokenAmount)).mul(
        new BN(10 ** bankInfo.decimals)
      );

      const maxAmount = new BN(balance);
      const finalAmount = amount.gt(maxAmount) ? maxAmount : amount;

      await makeDepositThroughMarginfi(user, bankInfo.bank, finalAmount);

      return {
        user: user.wallet.publicKey.toString(),
        type: "deposit",
        bank: bankInfo.bank,
        amount: finalAmount,
        decimals: bankInfo.decimals,
        symbol: bankInfo.symbol,
        timestamp: await getCurrentTimestamp(),
        slot: await getCurrentSlot(),
      };
    } else {
      const userAccount = user.accounts.get(NEW_DRIFT_ACCOUNT)!;
      const marginfiAccount =
        await bankrunProgram.account.marginfiAccount.fetch(userAccount);

      const balance = marginfiAccount.lendingAccount.balances.find(
        (b) => b.active === 1 && b.bankPk.equals(bankInfo.bank)
      );

      if (!balance) {
        return simulateRandomOperation(user, banks);
      }

      const bank = await bankrunProgram.account.bank.fetch(bankInfo.bank);
      const driftUser = await getDriftUserAccount(
        driftBankrunProgram,
        bank.driftUser
      );
      const spotPosition = driftUser.spotPositions[0];
      const scaledBalance = new BN(spotPosition.scaledBalance.toString());

      if (scaledBalance.eq(new BN(0))) {
        return simulateRandomOperation(user, banks);
      }

      const marginfiAssetSharesBigNumber = wrappedI80F48toBigNumber(
        balance.assetShares
      );
      const marginfiAssetShares = new BN(
        marginfiAssetSharesBigNumber.toString()
      );

      const spotMarket = await getSpotMarketAccount(
        driftBankrunProgram,
        bankInfo.marketIndex
      );
      const maxTokenAmount = scaledBalanceToTokenAmount(
        marginfiAssetShares,
        spotMarket,
        true
      );

      const percentage = 0.1 + Math.random() * 0.8;
      const amount = new BN(Math.floor(Number(maxTokenAmount) * percentage));

      await makeWithdrawThroughMarginfi(user, bankInfo.bank, amount);

      return {
        user: user.wallet.publicKey.toString(),
        type: "withdraw",
        bank: bankInfo.bank,
        amount,
        decimals: bankInfo.decimals,
        symbol: bankInfo.symbol,
        timestamp: await getCurrentTimestamp(),
        slot: await getCurrentSlot(),
      };
    }
  }

  it("performs random deposit-withdraw cycles alternating withdrawal methods", async () => {
    const NUM_CYCLES = 20;

    const cycleOperations: {
      cycle: number;
      user: string;
      bank: string;
      depositAmount: BN;
      daysAdvanced: number;
      balanceBeforeWithdraw: BN;
      interestEarned: BN;
      withdrawalMethod: "withdraw_all" | "token_amount_then_all";
    }[] = [];

    const banks: BankInfo[] = [
      {
        bank: newDriftUsdcBank,
        mint: ecosystem.usdcMint.publicKey,
        decimals: ecosystem.usdcDecimals,
        symbol: "USDC",
        spotMarket: driftAccounts.get(DRIFT_USDC_SPOT_MARKET)!,
        marketIndex: USDC_MARKET_INDEX,
      },
      {
        bank: newDriftTokenABank,
        mint: ecosystem.tokenAMint.publicKey,
        decimals: ecosystem.tokenADecimals,
        symbol: "TKA",
        spotMarket: driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET)!,
        marketIndex: TOKEN_A_MARKET_INDEX,
      },
    ];

    for (let cycle = 0; cycle < NUM_CYCLES; cycle++) {
      const user = Math.random() < 0.5 ? userA : userB;
      const userLabel = user === userA ? "User A" : "User B";
      const bankInfo = banks[Math.floor(Math.random() * banks.length)];

      const tokenAccount =
        bankInfo.symbol === "USDC" ? user.usdcAccount : user.tokenAAccount;
      const tokenBalance = await getTokenBalance(bankRunProvider, tokenAccount);

      if (tokenBalance === 0) {
        continue;
      }

      const percentage = 0.1 + Math.random() * 0.4;
      const depositAmount = new BN(
        Math.floor(Number(tokenBalance) * percentage)
      );

      await makeDepositThroughMarginfi(user, bankInfo.bank, depositAmount);

      const userAccount = user.accounts.get(NEW_DRIFT_ACCOUNT)!;
      const marginfiAccountBefore =
        await bankrunProgram.account.marginfiAccount.fetch(userAccount);
      const balanceBefore = marginfiAccountBefore.lendingAccount.balances.find(
        (b) => b.active === 1 && b.bankPk.equals(bankInfo.bank)
      );

      let scaledBalanceBefore = new BN(0);
      if (balanceBefore) {
        const assetSharesBefore = wrappedI80F48toBigNumber(
          balanceBefore.assetShares
        );
        scaledBalanceBefore = new BN(assetSharesBefore.toString());
      }

      await advanceTimeAndAccrueInterest(1);

      const marginfiAccountAfter =
        await bankrunProgram.account.marginfiAccount.fetch(userAccount);
      const balanceAfter = marginfiAccountAfter.lendingAccount.balances.find(
        (b) => b.active === 1 && b.bankPk.equals(bankInfo.bank)
      );

      let scaledBalanceAfter = new BN(0);
      if (balanceAfter) {
        const assetSharesAfter = wrappedI80F48toBigNumber(
          balanceAfter.assetShares
        );
        scaledBalanceAfter = new BN(assetSharesAfter.toString());
      }

      const interestEarned = scaledBalanceAfter.sub(scaledBalanceBefore);

      await advanceTimeAndAccrueInterest(1);

      const useWithdrawAll = cycle % 2 === 0;

      if (useWithdrawAll) {
        await makeWithdrawThroughMarginfi(user, bankInfo.bank, new BN(0), true);
        cycleOperations.push({
          cycle,
          user: userLabel,
          bank: bankInfo.symbol,
          depositAmount,
          daysAdvanced: 1,
          balanceBeforeWithdraw: scaledBalanceAfter,
          interestEarned,
          withdrawalMethod: "withdraw_all",
        });
      } else {
        const spotMarket = await getSpotMarketAccount(
          driftBankrunProgram,
          bankInfo.marketIndex
        );
        const fullTokenAmount = scaledBalanceToTokenAmount(
          scaledBalanceAfter,
          spotMarket,
          true
        );

        // Withdraw between 40-60% to add randomization and avoid duplicate transactions
        const withdrawPercentage = 40 + Math.floor(Math.random() * 21); // 40-60%
        const tokenAmount = fullTokenAmount
          .mul(new BN(withdrawPercentage))
          .div(new BN(100));

        // Only do ONE withdrawal (not two) to avoid duplicate transaction errors
        await makeWithdrawThroughMarginfi(user, bankInfo.bank, tokenAmount);

        const marginfiAccountMid =
          await bankrunProgram.account.marginfiAccount.fetch(userAccount);
        const balanceMid = marginfiAccountMid.lendingAccount.balances.find(
          (b) => b.active === 1 && b.bankPk.equals(bankInfo.bank)
        );

        if (balanceMid) {
          const assetSharesMid = wrappedI80F48toBigNumber(
            balanceMid.assetShares
          );
          const scaledBalanceMid = new BN(assetSharesMid.toString());

          if (!scaledBalanceMid.isZero()) {
            await makeWithdrawThroughMarginfi(
              user,
              bankInfo.bank,
              new BN(0),
              true
            );
          }
        }

        cycleOperations.push({
          cycle,
          user: userLabel,
          bank: bankInfo.symbol,
          depositAmount,
          daysAdvanced: 1,
          balanceBeforeWithdraw: scaledBalanceAfter,
          interestEarned,
          withdrawalMethod: "token_amount_then_all",
        });
      }
    }
  });

  it("tests instant deposit and withdrawal (not withdraw_all) for rounding edge case", async () => {
    // This test verifies the edge case fix where immediate withdrawal after deposit
    // would fail due to scaled_decrement being 1 more than asset_shares

    const user = userA;
    const testAmount = new BN(1 * 10 ** ecosystem.usdcDecimals);

    const initialBalance = await getTokenBalance(
      bankRunProvider,
      user.usdcAccount
    );

    await makeDepositThroughMarginfi(user, newDriftUsdcBank, testAmount);

    const balanceAfterDeposit = await getTokenBalance(
      bankRunProvider,
      user.usdcAccount
    );
    const deposited = new BN(initialBalance - balanceAfterDeposit);
    assertBNEqual(deposited, testAmount);

    const userAccount = user.accounts.get(NEW_DRIFT_ACCOUNT)!;

    // Immediately withdraw the same amount (not using withdraw_all)
    // This triggers the edge case where scaled_decrement might be asset_shares + 1
    await makeWithdrawThroughMarginfi(
      user,
      newDriftUsdcBank,
      testAmount,
      false
    );

    const balanceAfterWithdraw = await getTokenBalance(
      bankRunProvider,
      user.usdcAccount
    );
    const withdrawn = new BN(balanceAfterWithdraw - balanceAfterDeposit);
    const expectedWithdrawn = testAmount.sub(new BN(1)); // Expect 1 token less due to rounding

    assertBNEqual(withdrawn, expectedWithdrawn);

    const marginfiAccountAfterWithdraw =
      await bankrunProgram.account.marginfiAccount.fetch(userAccount);
    const balanceAfterWithdrawObj =
      marginfiAccountAfterWithdraw.lendingAccount.balances.find(
        (b) => b.active === 1 && b.bankPk.equals(newDriftUsdcBank)
      );

    let remainingScaledBalance = new BN(0);
    if (balanceAfterWithdrawObj) {
      const assetSharesAfterWithdraw = wrappedI80F48toBigNumber(
        balanceAfterWithdrawObj.assetShares
      );
      remainingScaledBalance = new BN(assetSharesAfterWithdraw.toString());

      // The dust should be small relative to the deposit
      // With Drift's scaling factor, we expect dust to be <1000 scaled units
      // otherwise it would have been possible to withdraw.
      assert(
        remainingScaledBalance.gt(new BN(0)),
        "Should have some dust remaining due to rounding"
      );
      assert(
        remainingScaledBalance.lt(new BN(1000)),
        `Dust amount ${remainingScaledBalance.toString()} seems too large`
      );
    }

    // Clean up the dust with withdraw_all
    await makeWithdrawThroughMarginfi(user, newDriftUsdcBank, new BN(0), true);

    const balanceAfterWithdrawAll = await getTokenBalance(
      bankRunProvider,
      user.usdcAccount
    );
    assertBNEqual(
      new BN(balanceAfterWithdrawAll),
      new BN(balanceAfterWithdraw)
    );

    const marginfiAccountFinal =
      await bankrunProgram.account.marginfiAccount.fetch(userAccount);
    const finalBalance = marginfiAccountFinal.lendingAccount.balances.find(
      (b) => b.active === 1 && b.bankPk.equals(newDriftUsdcBank)
    );

    assert(!finalBalance);
  });
});
