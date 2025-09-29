import { BN, Program } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import { assert } from "chai";
import BigNumber from "bignumber.js";
import Decimal from "decimal.js";
import { Clock } from "solana-bankrun";
import { createMintToInstruction, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import {
  bankRunProvider,
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  globalProgramAdmin,
  groupAdmin,
  kaminoAccounts,
  kaminoGroup,
  klendBankrunProgram,
  MARKET,
  oracles,
  users,
} from "./rootHooks";
import {
  defaultKaminoBankConfig,
  getLiquidityExchangeRate,
  simpleRefreshObligation,
  simpleRefreshReserve,
  RESERVE_SIZE,
} from "./utils/kamino-utils";
import { deriveBankWithSeed, deriveObligation, deriveUserMetadata } from "./utils/pdas";
import { processBankrunTransaction } from "./utils/tools";
import { makeAddKaminoBankIx, makeKaminoDepositIx } from "./utils/kamino-instructions";
import { accountInit } from "./utils/user-instructions";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { assertBNApproximately } from "./utils/genericTests";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { Marginfi } from "../target/types/marginfi";
import {
  lendingMarketAuthPda,
  reserveLiqSupplyPda,
  reserveFeeVaultPda,
  reserveCollateralMintPda,
  reserveCollateralSupplyPda,
  LendingMarket,
  MarketWithAddress,
  Reserve,
  BorrowRateCurve,
  BorrowRateCurveFields,
  CurvePoint,
  PriceFeed,
  AssetReserveConfig,
  updateEntireReserveConfigIx,
} from "@kamino-finance/klend-sdk";
import { InitObligationArgs } from "@kamino-finance/klend-sdk/dist/idl_codegen/types";

const NEW_RESERVE_LABEL = "k17_tokenb_reserve";
const NEW_BANK_LABEL = "k17_tokenb_bank";
const BANK_SEED = new BN(1701);
const SECONDS_PER_DAY = 86_400;

const bn10 = new BN(10);

const bnFromBigNumber = (value: BigNumber): BN =>
  new BN(value.integerValue(BigNumber.ROUND_FLOOR).toFixed(0));

const calcTolerance = (target: BN): BN => {
  const tolerance = target.div(new BN(1000));
  return tolerance.gt(new BN(0)) ? tolerance : new BN(1);
};

const makePulseHealthIx = async (
  program: Program<Marginfi>,
  marginfiAccount: PublicKey,
  remainingAccounts: { pubkey: PublicKey; isSigner: boolean; isWritable: boolean }[]
) =>
  program.methods
    .lendingAccountPulseHealth()
    .accounts({
      marginfiAccount,
    })
    .remainingAccounts(remainingAccounts)
    .instruction();

async function createReserve(
  mint: PublicKey,
  decimals: number,
  oracle: PublicKey,
  liquiditySource: PublicKey
): Promise<PublicKey> {
  const reserve = Keypair.generate();
  const market = kaminoAccounts.get(MARKET)!;
  const programId = klendBankrunProgram.programId;

  const [lendingMarketAuthority] = lendingMarketAuthPda(market, programId);
  const [reserveLiquiditySupply] = reserveLiqSupplyPda(market, mint, programId);
  const [reserveFeeVault] = reserveFeeVaultPda(market, mint, programId);
  const [collateralMint] = reserveCollateralMintPda(market, mint, programId);
  const [collateralSupply] = reserveCollateralSupplyPda(market, mint, programId);

  const initTx = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: groupAdmin.wallet.publicKey,
      newAccountPubkey: reserve.publicKey,
      space: RESERVE_SIZE + 8,
      lamports:
        await bankRunProvider.connection.getMinimumBalanceForRentExemption(
          RESERVE_SIZE + 8
        ),
      programId,
    }),
    await klendBankrunProgram.methods
      .initReserve()
      .accounts({
        lendingMarketOwner: groupAdmin.wallet.publicKey,
        lendingMarket: market,
        lendingMarketAuthority,
        reserve: reserve.publicKey,
        reserveLiquidityMint: mint,
        reserveLiquiditySupply,
        feeReceiver: reserveFeeVault,
        reserveCollateralMint: collateralMint,
        reserveCollateralSupply: collateralSupply,
        initialLiquiditySource: liquiditySource,
        rent: SYSVAR_RENT_PUBKEY,
        liquidityTokenProgram: TOKEN_PROGRAM_ID,
        collateralTokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction()
  );

  await processBankrunTransaction(bankrunContext, initTx, [groupAdmin.wallet, reserve]);

  const marketAccount: LendingMarket = LendingMarket.decode(
    (await bankRunProvider.connection.getAccountInfo(market))!.data
  );
  const marketWithAddress: MarketWithAddress = {
    address: market,
    state: marketAccount,
  };

  const borrowRateCurve = new BorrowRateCurve({
    points: [
      new CurvePoint({ utilizationRateBps: 0, borrowRateBps: 10_000 }),
      new CurvePoint({ utilizationRateBps: 5_000, borrowRateBps: 80_000 }),
      new CurvePoint({ utilizationRateBps: 8_000, borrowRateBps: 160_000 }),
      new CurvePoint({ utilizationRateBps: 10_000, borrowRateBps: 240_000 }),
      ...Array(7).fill(
        new CurvePoint({ utilizationRateBps: 10_000, borrowRateBps: 240_000 })
      ),
    ],
  } as BorrowRateCurveFields);

  const assetReserveConfig = new AssetReserveConfig({
    mint,
    mintTokenProgram: TOKEN_PROGRAM_ID,
    tokenName: NEW_RESERVE_LABEL,
    mintDecimals: decimals,
    priceFeed: { pythPrice: oracle } as PriceFeed,
    loanToValuePct: 80,
    liquidationThresholdPct: 87,
    borrowRateCurve,
    depositLimit: new Decimal(1_000_000_000),
    borrowLimit: new Decimal(1_000_000_000),
  }).getReserveConfig();

  const updateIx = updateEntireReserveConfigIx(
    marketWithAddress,
    reserve.publicKey,
    assetReserveConfig,
    programId
  );

  await processBankrunTransaction(bankrunContext, new Transaction().add(updateIx), [
    groupAdmin.wallet,
  ]);

  kaminoAccounts.set(NEW_RESERVE_LABEL, reserve.publicKey);
  return reserve.publicKey;
}

async function ensureKaminoUser(
  owner: Keypair,
  obligationId: number
): Promise<PublicKey> {
  const [metadata] = deriveUserMetadata(klendBankrunProgram.programId, owner.publicKey);
  const metadataInfo = await bankRunProvider.connection.getAccountInfo(metadata);
  if (!metadataInfo) {
    const dummyLookup = new PublicKey(Buffer.alloc(32));
    const metadataTx = new Transaction().add(
      await klendBankrunProgram.methods
        .initUserMetadata(dummyLookup)
        .accounts({
          owner: owner.publicKey,
          feePayer: owner.publicKey,
          userMetadata: metadata,
          referrerUserMetadata: null,
          rent: SYSVAR_RENT_PUBKEY,
          systemProgram: SystemProgram.programId,
        })
        .instruction()
    );
    await processBankrunTransaction(bankrunContext, metadataTx, [owner]);
  }

  const initArgs = new InitObligationArgs({ tag: 0, id: obligationId });
  const [obligation] = deriveObligation(
    klendBankrunProgram.programId,
    initArgs.tag,
    initArgs.id,
    owner.publicKey,
    kaminoAccounts.get(MARKET)!,
    PublicKey.default,
    PublicKey.default
  );
  const obligationInfo = await bankRunProvider.connection.getAccountInfo(obligation);
  if (!obligationInfo) {
    const obligationTx = new Transaction().add(
      await klendBankrunProgram.methods
        .initObligation(initArgs)
        .accounts({
          obligationOwner: owner.publicKey,
          feePayer: owner.publicKey,
          obligation,
          lendingMarket: kaminoAccounts.get(MARKET)!,
          seed1Account: PublicKey.default,
          seed2Account: PublicKey.default,
          ownerUserMetadata: metadata,
          rent: SYSVAR_RENT_PUBKEY,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .instruction()
    );
    await processBankrunTransaction(bankrunContext, obligationTx, [owner]);
  }

  return obligation;
}

async function mintToAccount(amount: BN, mint: PublicKey, destination: PublicKey) {
  const ix = createMintToInstruction(
    mint,
    destination,
    globalProgramAdmin.wallet.publicKey,
    amount.toNumber()
  );
  await processBankrunTransaction(
    bankrunContext,
    new Transaction().add(ix),
    [globalProgramAdmin.wallet]
  );
}

describe("k17: Kamino interest after borrow", () => {
  it("deposits, borrows, and accrues interest correctly", async () => {
    const tokenBMint = ecosystem.tokenBMint.publicKey;
    const tokenBDecimals = ecosystem.tokenBDecimals;
    const tokenBOracle = oracles.tokenBOracle.publicKey;
    const market = kaminoAccounts.get(MARKET)!;

    const baseUnit = bn10.pow(new BN(tokenBDecimals));
    const reserveDepositAmount = new BN(1_000).mul(baseUnit);
    const borrowAmount = reserveDepositAmount.div(new BN(2));
    const marginfiDepositAmount = new BN(150).mul(baseUnit);

    await mintToAccount(reserveDepositAmount, tokenBMint, groupAdmin.tokenBAccount);

    const reservePubkey = await createReserve(
      tokenBMint,
      tokenBDecimals,
      tokenBOracle,
      groupAdmin.tokenBAccount
    );

    await processBankrunTransaction(
      bankrunContext,
      new Transaction().add(
        await simpleRefreshReserve(klendBankrunProgram, reservePubkey, market, tokenBOracle)
      ),
      [groupAdmin.wallet]
    );

    const depositUser = users[2];
    const depositObligation = await ensureKaminoUser(depositUser.wallet, 91);
    await mintToAccount(reserveDepositAmount, tokenBMint, depositUser.tokenBAccount);

    const reserveBeforeDeposit = await klendBankrunProgram.account.reserve.fetch(reservePubkey);
    const liquidityMint = reserveBeforeDeposit.liquidity.mintPubkey;
    const liquiditySupply = reserveBeforeDeposit.liquidity.supplyVault;
    const collateralMint = reserveBeforeDeposit.collateral.mintPubkey;
    const collateralSupply = reserveBeforeDeposit.collateral.supplyVault;
    const [lendingMarketAuthority] = lendingMarketAuthPda(
      market,
      klendBankrunProgram.programId
    );

    const depositTx = new Transaction().add(
      await simpleRefreshReserve(klendBankrunProgram, reservePubkey, market, tokenBOracle),
      await simpleRefreshObligation(klendBankrunProgram, market, depositObligation),
      await klendBankrunProgram.methods
        .depositReserveLiquidityAndObligationCollateral(reserveDepositAmount)
        .accounts({
          owner: depositUser.wallet.publicKey,
          obligation: depositObligation,
          lendingMarket: market,
          lendingMarketAuthority,
          reserve: reservePubkey,
          reserveLiquidityMint: liquidityMint,
          reserveLiquiditySupply: liquiditySupply,
          reserveCollateralMint: collateralMint,
          reserveDestinationDepositCollateral: collateralSupply,
          userSourceLiquidity: depositUser.tokenBAccount,
          placeholderUserDestinationCollateral: null,
          collateralTokenProgram: TOKEN_PROGRAM_ID,
          liquidityTokenProgram: TOKEN_PROGRAM_ID,
          instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
        })
        .instruction()
    );
    await processBankrunTransaction(bankrunContext, depositTx, [depositUser.wallet]);

    const reserveAfterDeposit = await klendBankrunProgram.account.reserve.fetch(reservePubkey);
    assertBNApproximately(
      reserveAfterDeposit.liquidity.availableAmount,
      reserveDepositAmount,
      new BN(5)
    );

    const borrowTx = new Transaction().add(
      await simpleRefreshReserve(klendBankrunProgram, reservePubkey, market, tokenBOracle),
      await simpleRefreshObligation(klendBankrunProgram, market, depositObligation, [reservePubkey]),
      await klendBankrunProgram.methods
        .borrowObligationLiquidity(borrowAmount)
        .accounts({
          owner: depositUser.wallet.publicKey,
          obligation: depositObligation,
          lendingMarket: market,
          lendingMarketAuthority,
          borrowReserve: reservePubkey,
          borrowReserveLiquidityMint: liquidityMint,
          reserveSourceLiquidity: liquiditySupply,
          userDestinationLiquidity: depositUser.tokenBAccount,
          borrowReserveLiquidityFeeReceiver: reserveAfterDeposit.liquidity.feeVault,
          referrerTokenState: klendBankrunProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
        })
        .instruction()
    );
    await processBankrunTransaction(bankrunContext, borrowTx, [depositUser.wallet]);

    const reserveAfterBorrow = await klendBankrunProgram.account.reserve.fetch(reservePubkey);
    const expectedAvailableAfterBorrow = reserveDepositAmount.sub(borrowAmount);
    assertBNApproximately(
      reserveAfterBorrow.liquidity.availableAmount,
      expectedAvailableAfterBorrow,
      new BN(5)
    );

    const addBankTx = new Transaction().add(
      await makeAddKaminoBankIx(
        groupAdmin.mrgnBankrunProgram!,
        {
          group: kaminoGroup.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: tokenBMint,
          kaminoReserve: reservePubkey,
          kaminoMarket: market,
          oracle: tokenBOracle,
        },
        {
          config: defaultKaminoBankConfig(tokenBOracle),
          seed: BANK_SEED,
        }
      )
    );
    await processBankrunTransaction(bankrunContext, addBankTx, [groupAdmin.wallet]);

    const [bankPk] = deriveBankWithSeed(
      bankrunProgram.programId,
      kaminoGroup.publicKey,
      tokenBMint,
      BANK_SEED
    );
    kaminoAccounts.set(NEW_BANK_LABEL, bankPk);

    const marginfiUser = users[1];
    const marginfiAccountKeypair = Keypair.generate();
    const initAccountTx = new Transaction().add(
      await accountInit(marginfiUser.mrgnBankrunProgram!, {
        marginfiGroup: kaminoGroup.publicKey,
        marginfiAccount: marginfiAccountKeypair.publicKey,
        authority: marginfiUser.wallet.publicKey,
        feePayer: marginfiUser.wallet.publicKey,
      })
    );
    await processBankrunTransaction(bankrunContext, initAccountTx, [
      marginfiUser.wallet,
      marginfiAccountKeypair,
    ]);

    await mintToAccount(marginfiDepositAmount.mul(new BN(2)), tokenBMint, marginfiUser.tokenBAccount);

    const kaminoDepositTx = new Transaction().add(
      await simpleRefreshReserve(klendBankrunProgram, reservePubkey, market, tokenBOracle),
      await makeKaminoDepositIx(
        marginfiUser.mrgnBankrunProgram!,
        {
          marginfiAccount: marginfiAccountKeypair.publicKey,
          bank: bankPk,
          signerTokenAccount: marginfiUser.tokenBAccount,
          lendingMarket: market,
          reserveLiquidityMint: tokenBMint,
        },
        marginfiDepositAmount
      )
    );
    await processBankrunTransaction(bankrunContext, kaminoDepositTx, [marginfiUser.wallet]);

    const bankBeforePulse = await bankrunProgram.account.bank.fetch(bankPk);
    const pulseIx = await makePulseHealthIx(
      marginfiUser.mrgnBankrunProgram!,
      marginfiAccountKeypair.publicKey,
      [
        { pubkey: bankPk, isSigner: false, isWritable: false },
        { pubkey: tokenBOracle, isSigner: false, isWritable: false },
        { pubkey: reservePubkey, isSigner: false, isWritable: false },
      ]
    );

    const pulseTx = new Transaction().add(
      await simpleRefreshReserve(klendBankrunProgram, reservePubkey, market, tokenBOracle),
      await simpleRefreshObligation(
        klendBankrunProgram,
        market,
        bankBeforePulse.kaminoObligation,
        [reservePubkey]
      ),
      pulseIx
    );
    await processBankrunTransaction(bankrunContext, pulseTx, [marginfiUser.wallet], false, true);

    const marginfiAccountAfterPulse = await bankrunProgram.account.marginfiAccount.fetch(
      marginfiAccountKeypair.publicKey
    );
    const bankAfterPulse = await bankrunProgram.account.bank.fetch(bankPk);
    const reserveForInitialValuation = await klendBankrunProgram.account.reserve.fetch(
      reservePubkey
    );

    const balanceRecord = marginfiAccountAfterPulse.lendingAccount.balances.find(
      (balance) => balance.bankPk.equals(bankPk) && balance.active === 1
    );
    assert.isDefined(balanceRecord, "marginfi position should exist after deposit");

    const assetShares = wrappedI80F48toBigNumber(balanceRecord!.assetShares);
    const bankShareValueInitial = wrappedI80F48toBigNumber(bankAfterPulse.assetShareValue);
    const exchangeRateInitial = new BigNumber(
      getLiquidityExchangeRate(reserveForInitialValuation as Reserve).toString()
    );

    const expectedInitialUnderlying = bnFromBigNumber(
      assetShares.multipliedBy(exchangeRateInitial)
    );
    const actualInitialUnderlying = bnFromBigNumber(
      assetShares.multipliedBy(bankShareValueInitial)
    );
    assertBNApproximately(
      actualInitialUnderlying,
      expectedInitialUnderlying,
      calcTolerance(expectedInitialUnderlying)
    );

    const secondsToAdvance = 30 * SECONDS_PER_DAY;
    const slotsToAdvance = Math.floor(secondsToAdvance * 0.4);
    const currentClock = await banksClient.getClock();
    const newClock = new Clock(
      currentClock.slot + BigInt(slotsToAdvance),
      currentClock.epochStartTimestamp,
      currentClock.epoch,
      currentClock.leaderScheduleEpoch,
      currentClock.unixTimestamp + BigInt(secondsToAdvance)
    );
    bankrunContext.setClock(newClock);

    const refreshAfterWarp = new Transaction().add(
      await simpleRefreshReserve(klendBankrunProgram, reservePubkey, market, tokenBOracle),
      await simpleRefreshObligation(
        klendBankrunProgram,
        market,
        depositObligation,
        [reservePubkey]
      ),
      await simpleRefreshObligation(
        klendBankrunProgram,
        market,
        bankBeforePulse.kaminoObligation,
        [reservePubkey]
      )
    );
    await processBankrunTransaction(bankrunContext, refreshAfterWarp, [groupAdmin.wallet]);

    const pulseAfterWarpIx = await makePulseHealthIx(
      marginfiUser.mrgnBankrunProgram!,
      marginfiAccountKeypair.publicKey,
      [
        { pubkey: bankPk, isSigner: false, isWritable: false },
        { pubkey: tokenBOracle, isSigner: false, isWritable: false },
        { pubkey: reservePubkey, isSigner: false, isWritable: false },
      ]
    );
    await processBankrunTransaction(
      bankrunContext,
      new Transaction().add(pulseAfterWarpIx),
      [marginfiUser.wallet],
      false,
      true
    );

    const reserveAfterInterest = await klendBankrunProgram.account.reserve.fetch(reservePubkey);
    const bankAfterInterest = await bankrunProgram.account.bank.fetch(bankPk);
    const bankShareValueAfter = wrappedI80F48toBigNumber(bankAfterInterest.assetShareValue);
    const exchangeRateAfter = new BigNumber(
      getLiquidityExchangeRate(reserveAfterInterest as Reserve).toString()
    );

    const actualUnderlyingAfter = bnFromBigNumber(
      assetShares.multipliedBy(bankShareValueAfter)
    );
    const expectedUnderlyingAfter = bnFromBigNumber(
      assetShares.multipliedBy(exchangeRateAfter)
    );

    assertBNApproximately(
      actualUnderlyingAfter,
      expectedUnderlyingAfter,
      calcTolerance(expectedUnderlyingAfter)
    );
    assert.isTrue(
      actualUnderlyingAfter.gt(actualInitialUnderlying),
      "deposit value should increase after interest accrual"
    );
  });
});
