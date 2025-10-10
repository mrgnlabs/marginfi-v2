import {
  ComputeBudgetProgram,
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import {
  bankrunContext,
  bankRunProvider,
  ecosystem,
  globalProgramAdmin,
  groupAdmin,
  kaminoAccounts,
  klendBankrunProgram,
  MARKET,
  oracles,
  TOKEN_A_RESERVE,
  USDC_RESERVE,
  users,
  verbose,
} from "./rootHooks";
import { processBankrunTransaction } from "./utils/tools";
import { assert } from "chai";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assertBNApproximately, assertKeysEqual } from "./utils/genericTests";
import Decimal from "decimal.js";
import { Fraction } from "@kamino-finance/klend-sdk/dist/classes/fraction";
import {
  LENDING_MARKET_SIZE,
  RESERVE_SIZE,
  simpleRefreshReserve,
} from "./utils/kamino-utils";
// Note: there's some glitch in Kamino's lib based on a Raydium static init, it's currently patch-package hacked...
import {
  lendingMarketAuthPda,
  LendingMarket,
  reserveLiqSupplyPda,
  reserveFeeVaultPda,
  reserveCollateralMintPda,
  reserveCollateralSupplyPda,
  Reserve,
  MarketWithAddress,
  BorrowRateCurve,
  CurvePoint,
  BorrowRateCurveFields,
  PriceFeed,
  AssetReserveConfig,
  updateEntireReserveConfigIx,
} from "@kamino-finance/klend-sdk";
import { createMintToInstruction } from "@solana/spl-token";
import { ProgramTestContext } from "solana-bankrun";

let ctx: ProgramTestContext;

describe("k01: Init Kamino instance", () => {
  before(async () => {
    ctx = bankrunContext;
  });

  // Note: We use the same admins for Kamino as for mrgn, but in practice the Kamino program is
  // adminstrated by a different organization
  it("(admin) Init Kamino Market - happy path", async () => {
    const usdcString = "USDC";
    const quoteCurrency = Array.from(usdcString.padEnd(32, "\0")).map((c) =>
      c.charCodeAt(0)
    );

    const lendingMarket = Keypair.generate();
    const [lendingMarketAuthority] = lendingMarketAuthPda(
      lendingMarket.publicKey,
      klendBankrunProgram.programId
    );

    let tx = new Transaction();
    tx.add(
      // Create a zeroed account that's large enough to hold the lending market
      SystemProgram.createAccount({
        fromPubkey: groupAdmin.wallet.publicKey,
        newAccountPubkey: lendingMarket.publicKey,
        space: LENDING_MARKET_SIZE + 8,
        lamports:
          await bankRunProvider.connection.getMinimumBalanceForRentExemption(
            LENDING_MARKET_SIZE + 8
          ),
        programId: klendBankrunProgram.programId,
      }),
      // Init lending market
      await klendBankrunProgram.methods
        .initLendingMarket(quoteCurrency)
        .accounts({
          lendingMarketOwner: groupAdmin.wallet.publicKey,
          lendingMarket: lendingMarket.publicKey,
          lendingMarketAuthority: lendingMarketAuthority,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .instruction()
    );

    await processBankrunTransaction(ctx, tx, [
      groupAdmin.wallet,
      lendingMarket,
    ]);
    kaminoAccounts.set(MARKET, lendingMarket.publicKey);
    if (verbose) {
      console.log("Kamino market: " + lendingMarket.publicKey);
    }

    const marketAcc: LendingMarket = LendingMarket.decode(
      (await bankRunProvider.connection.getAccountInfo(lendingMarket.publicKey))
        .data
    );
    assertKeysEqual(marketAcc.lendingMarketOwner, groupAdmin.wallet.publicKey);
  });

  it("(admin) create USDC reserve", async () => {
    // We need to mint some USDC to the admin's account first
    const tx = new Transaction().add(
      createMintToInstruction(
        ecosystem.usdcMint.publicKey,
        groupAdmin.usdcAccount,
        globalProgramAdmin.wallet.publicKey,
        1000 * 10 ** ecosystem.usdcDecimals
      )
    );
    await processBankrunTransaction(ctx, tx, [globalProgramAdmin.wallet]);

    await createReserve(
      ecosystem.usdcMint.publicKey,
      USDC_RESERVE,
      ecosystem.usdcDecimals,
      // Note: Kamino performs zero oracle validation, it is happy to accept the mock program here
      // instead of Pyth, or any other spoof of Pyth with the same account structure, so be wary!
      // Using Pyth Pull oracle instead of legacy Pyth oracle
      oracles.usdcOracle.publicKey,
      groupAdmin.usdcAccount
    );
  });

  it("(admin) create token A reserve", async () => {
    // We need to mint some Token A to the admin's account first
    const tx = new Transaction().add(
      createMintToInstruction(
        ecosystem.tokenAMint.publicKey,
        groupAdmin.tokenAAccount,
        globalProgramAdmin.wallet.publicKey,
        1000 * 10 ** ecosystem.tokenADecimals
      )
    );
    await processBankrunTransaction(ctx, tx, [globalProgramAdmin.wallet]);

    await createReserve(
      ecosystem.tokenAMint.publicKey,
      TOKEN_A_RESERVE,
      ecosystem.tokenADecimals,
      // Using Pyth Pull oracle instead of legacy Pyth oracle
      oracles.tokenAOracle.publicKey,
      groupAdmin.tokenAAccount
    );
  });

  it("(user 0 - permissionless) refresh USDC reserve price with Pyth Pull oracle", async () => {
    let marketKey = kaminoAccounts.get(MARKET);
    let reserveKey = kaminoAccounts.get(USDC_RESERVE);

    const tx = new Transaction().add(
      await simpleRefreshReserve(
        klendBankrunProgram,
        reserveKey,
        marketKey,
        oracles.usdcOracle.publicKey
      )
    );

    await processBankrunTransaction(ctx, tx, [users[0].wallet]);

    const reserveAcc: Reserve = Reserve.decode(
      (await bankRunProvider.connection.getAccountInfo(reserveKey)).data
    );

    // Note: prices are stored as scaled fraction (multiply price by 2^60)
    // E.g. the price is 10 so 10 * 2^60 ~= 1.15292e+19
    let expected = Fraction.fromDecimal(new Decimal(oracles.usdcPrice));
    assertBNApproximately(
      reserveAcc.liquidity.marketPriceSf,
      expected.valueSf,
      100_000
    );
  });

  it("(admin - permissionless) refresh token A reserve price with Pyth Pull oracle", async () => {
    let marketKey = kaminoAccounts.get(MARKET);
    let reserveKey = kaminoAccounts.get(TOKEN_A_RESERVE);

    const tx = new Transaction().add(
      await simpleRefreshReserve(
        klendBankrunProgram,
        reserveKey,
        marketKey,
        oracles.tokenAOracle.publicKey
      )
    );
    await processBankrunTransaction(ctx, tx, [groupAdmin.wallet]);

    const reserveAcc: Reserve = Reserve.decode(
      (await bankRunProvider.connection.getAccountInfo(reserveKey)).data
    );

    // Note: prices are stored as scaled fraction (multiply price by 2^60)
    // E.g. the price is 10 so 10 * 2^60 ~= 1.15292e+19
    let expected = Fraction.fromDecimal(new Decimal(oracles.tokenAPrice));
    assertBNApproximately(
      reserveAcc.liquidity.marketPriceSf,
      expected.valueSf,
      100_000
    );
  });

  async function createReserve(
    mint: PublicKey,
    reserveLabel: string,
    decimals: number,
    oracle: PublicKey,
    liquiditySource: PublicKey
  ) {
    const reserve = Keypair.generate();
    const market = kaminoAccounts.get(MARKET);
    const id = klendBankrunProgram.programId;

    const [lendingMarketAuthority] = lendingMarketAuthPda(market, id);
    const [reserveLiquiditySupply] = reserveLiqSupplyPda(market, mint, id);
    const [reserveFeeVault] = reserveFeeVaultPda(market, mint, id);
    const [collatMint] = reserveCollateralMintPda(market, mint, id);
    const [collatSupply] = reserveCollateralSupplyPda(market, mint, id);

    const tx = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: groupAdmin.wallet.publicKey,
        newAccountPubkey: reserve.publicKey,
        space: RESERVE_SIZE + 8,
        lamports:
          await bankRunProvider.connection.getMinimumBalanceForRentExemption(
            RESERVE_SIZE + 8
          ),
        programId: klendBankrunProgram.programId,
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
          reserveCollateralMint: collatMint,
          reserveCollateralSupply: collatSupply,
          initialLiquiditySource: liquiditySource,
          rent: SYSVAR_RENT_PUBKEY,
          liquidityTokenProgram: TOKEN_PROGRAM_ID,
          collateralTokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .instruction()
    );

    await processBankrunTransaction(ctx, tx, [groupAdmin.wallet, reserve]);
    kaminoAccounts.set(reserveLabel, reserve.publicKey);

    if (verbose) {
      console.log("Kamino reserve " + reserveLabel + " " + reserve.publicKey);
    }

    const marketAcc: LendingMarket = LendingMarket.decode(
      (await bankRunProvider.connection.getAccountInfo(market)).data
    );
    const reserveAcc: Reserve = Reserve.decode(
      (await bankRunProvider.connection.getAccountInfo(reserve.publicKey)).data
    );
    assertKeysEqual(reserveAcc.lendingMarket, market);
    // Reserves start in an unconfigured "Hidden" state
    assert.equal(reserveAcc.config.status, 2);

    // Update the reserve to a sane operational state
    const marketWithAddress: MarketWithAddress = {
      address: market,
      state: marketAcc,
    };

    const borrowRateCurve = new BorrowRateCurve({
      points: [
        // At 0% utilization: 50% interest rate
        new CurvePoint({ utilizationRateBps: 0, borrowRateBps: 50000 }),
        // At 50% utilization: 100% interest rate
        new CurvePoint({ utilizationRateBps: 5000, borrowRateBps: 100000 }),
        // At 80% utilization: 500% interest rate
        new CurvePoint({ utilizationRateBps: 8000, borrowRateBps: 500000 }),
        // At 100% utilization: 1000% interest rate
        new CurvePoint({ utilizationRateBps: 10000, borrowRateBps: 1000000 }),
        // Fill remaining points to complete the curve
        ...Array(7).fill(
          new CurvePoint({ utilizationRateBps: 10000, borrowRateBps: 1000000 })
        ),
      ],
    } as BorrowRateCurveFields);
    const assetReserveConfigParams = {
      loanToValuePct: 75, // 75%
      liquidationThresholdPct: 85, // 85%
      borrowRateCurve,
      depositLimit: new Decimal(1_000_000_000),
      borrowLimit: new Decimal(1_000_000_000),
    };

    const priceFeed: PriceFeed = {
      pythPrice: oracle,
      // switchboardPrice: NULL_PUBKEY,
      // switchboardTwapPrice: NULL_PUBKEY,
      // scopePriceConfigAddress: NULL_PUBKEY,
      // scopeChain: [0, 65535, 65535, 65535],
      // scopeTwapChain: [52, 65535, 65535, 65535],
    };

    const assetReserveConfig = new AssetReserveConfig({
      mint: mint,
      mintTokenProgram: TOKEN_PROGRAM_ID,
      tokenName: reserveLabel,
      mintDecimals: decimals,
      priceFeed: priceFeed,
      ...assetReserveConfigParams,
    }).getReserveConfig();

    const updateReserveIx = updateEntireReserveConfigIx(
      marketWithAddress,
      reserve.publicKey,
      assetReserveConfig,
      klendBankrunProgram.programId
    );

    const updateTx = new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({
        units: 1_000_000,
      }),
      updateReserveIx
    );

    await processBankrunTransaction(ctx, updateTx, [groupAdmin.wallet]);
  }
});
