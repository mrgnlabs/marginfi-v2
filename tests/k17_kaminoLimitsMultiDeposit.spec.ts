import { BN } from "@coral-xyz/anchor";
import {
  AddressLookupTableAccount,
  AddressLookupTableProgram,
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import {
  bankrunContext,
  banksClient,
  bankRunProvider,
  ecosystem,
  groupAdmin,
  globalProgramAdmin,
  kaminoAccounts,
  kaminoGroup,
  klendBankrunProgram,
  MARKET,
  oracles,
  TOKEN_A_RESERVE,
  users,
  verbose,
} from "./rootHooks";
import { refreshPullOraclesBankrun } from "./utils/bankrun-oracles";
import {
  makeAddKaminoBankIx,
  makeInitObligationIx,
  makeKaminoDepositIx,
} from "./utils/kamino-instructions";
import {
  defaultKaminoBankConfig,
  simpleRefreshReserve,
  simpleRefreshObligation,
} from "./utils/kamino-utils";
import {
  deriveBankWithSeed,
  deriveLiquidityVaultAuthority,
  deriveBaseObligation,
} from "./utils/pdas";
import {
  dumpAccBalances,
  dumpBankrunLogs,
  processBankrunTransaction,
} from "./utils/tools";
import {
  lendingMarketAuthPda,
  reserveLiqSupplyPda,
  reserveFeeVaultPda,
  reserveCollateralMintPda,
  reserveCollateralSupplyPda,
  LendingMarket,
  Reserve,
  MarketWithAddress,
  BorrowRateCurve,
  CurvePoint,
  BorrowRateCurveFields,
  PriceFeed,
  AssetReserveConfig,
  updateEntireReserveConfigIx,
} from "@kamino-finance/klend-sdk";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { ComputeBudgetProgram } from "@solana/web3.js";
import Decimal from "decimal.js";

const MAX_KAMINO_DEPOSITS = 15;
const USER_ACCOUNT = "user_account_k17";
const STARTING_SEED = 17000;
const LENDING_MARKET_SIZE = 7352;
const RESERVE_SIZE = 8832 - 8;

describe("k17: Limits test - 15 Kamino deposits + 1 borrow, liquidation with LUT", () => {
  let kaminoMarkets: PublicKey[] = [];
  let kaminoReserves: PublicKey[] = [];
  let kaminoBanks: PublicKey[] = [];
  let regularBank: PublicKey;
  let lutAddress: PublicKey;
  let lut: AddressLookupTableAccount;

  it("Refresh oracles", async () => {
    await refreshPullOraclesBankrun(oracles, bankrunContext, banksClient);
  });

  it("Create 15 markets + 15 reserves + 15 Kamino banks", async () => {
    if (verbose) {
      console.log(
        `Creating ${MAX_KAMINO_DEPOSITS} Kamino markets/reserves/banks...`
      );
    }

    // Create all markets/reserves/banks sequentially
    for (let i = 0; i < MAX_KAMINO_DEPOSITS; i++) {
      if (verbose) {
        console.log(`\nCreating market/reserve/bank #${i}...`);
      }

      // Step 1: Create Kamino market
      const marketKeypair = Keypair.generate();
      const quoteCurrency = Array(32).fill(0); // USD quote currency
      const id = klendBankrunProgram.programId;
      const [lendingMarketAuthority] = lendingMarketAuthPda(
        marketKeypair.publicKey,
        id
      );

      const createMarketTx = new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: groupAdmin.wallet.publicKey,
          newAccountPubkey: marketKeypair.publicKey,
          space: LENDING_MARKET_SIZE + 8,
          lamports:
            await bankRunProvider.connection.getMinimumBalanceForRentExemption(
              LENDING_MARKET_SIZE + 8
            ),
          programId: id,
        }),
        await klendBankrunProgram.methods
          .initLendingMarket(quoteCurrency)
          .accounts({
            lendingMarketOwner: groupAdmin.wallet.publicKey,
            lendingMarket: marketKeypair.publicKey,
            lendingMarketAuthority,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
          })
          .instruction()
      );

      await processBankrunTransaction(bankrunContext, createMarketTx, [
        groupAdmin.wallet,
        marketKeypair,
      ]);

      if (verbose) {
        console.log("Created market:", marketKeypair.publicKey.toBase58());
      }

      // Step 2: Create Kamino reserve
      const reserveKeypair = Keypair.generate();
      const mint = ecosystem.tokenAMint.publicKey;

      const [reserveLiquiditySupply] = reserveLiqSupplyPda(
        marketKeypair.publicKey,
        mint,
        id
      );
      const [reserveFeeVault] = reserveFeeVaultPda(
        marketKeypair.publicKey,
        mint,
        id
      );
      const [collatMint] = reserveCollateralMintPda(
        marketKeypair.publicKey,
        mint,
        id
      );
      const [collatSupply] = reserveCollateralSupplyPda(
        marketKeypair.publicKey,
        mint,
        id
      );

      const createReserveTx = new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: groupAdmin.wallet.publicKey,
          newAccountPubkey: reserveKeypair.publicKey,
          space: RESERVE_SIZE + 8,
          lamports:
            await bankRunProvider.connection.getMinimumBalanceForRentExemption(
              RESERVE_SIZE + 8
            ),
          programId: id,
        }),
        await klendBankrunProgram.methods
          .initReserve()
          .accounts({
            lendingMarketOwner: groupAdmin.wallet.publicKey,
            lendingMarket: marketKeypair.publicKey,
            lendingMarketAuthority,
            reserve: reserveKeypair.publicKey,
            reserveLiquidityMint: mint,
            reserveLiquiditySupply,
            feeReceiver: reserveFeeVault,
            reserveCollateralMint: collatMint,
            reserveCollateralSupply: collatSupply,
            initialLiquiditySource: groupAdmin.tokenAAccount,
            rent: SYSVAR_RENT_PUBKEY,
            liquidityTokenProgram: TOKEN_PROGRAM_ID,
            collateralTokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .instruction()
      );

      await processBankrunTransaction(bankrunContext, createReserveTx, [
        groupAdmin.wallet,
        reserveKeypair,
      ]);

      if (verbose) {
        console.log("Created reserve:", reserveKeypair.publicKey.toBase58());
      }

      // Step 2.5: Update reserve config to make it operational
      const marketAcc: LendingMarket = LendingMarket.decode(
        (
          await bankRunProvider.connection.getAccountInfo(
            marketKeypair.publicKey
          )
        ).data
      );
      const marketWithAddress: MarketWithAddress = {
        address: marketKeypair.publicKey,
        state: marketAcc,
      };

      const borrowRateCurve = new BorrowRateCurve({
        points: [
          new CurvePoint({ utilizationRateBps: 0, borrowRateBps: 50000 }),
          new CurvePoint({ utilizationRateBps: 5000, borrowRateBps: 100000 }),
          new CurvePoint({ utilizationRateBps: 8000, borrowRateBps: 500000 }),
          new CurvePoint({ utilizationRateBps: 10000, borrowRateBps: 1000000 }),
          ...Array(7).fill(
            new CurvePoint({
              utilizationRateBps: 10000,
              borrowRateBps: 1000000,
            })
          ),
        ],
      } as BorrowRateCurveFields);

      const assetReserveConfigParams = {
        loanToValuePct: 75,
        liquidationThresholdPct: 85,
        borrowRateCurve,
        depositLimit: new Decimal(1_000_000_000),
        borrowLimit: new Decimal(1_000_000_000),
      };

      const priceFeed: PriceFeed = {
        pythPrice: oracles.tokenAOracle.publicKey,
      };

      const assetReserveConfig = new AssetReserveConfig({
        mint: mint,
        mintTokenProgram: TOKEN_PROGRAM_ID,
        tokenName: "TOKEN_A",
        mintDecimals: ecosystem.tokenADecimals,
        priceFeed: priceFeed,
        ...assetReserveConfigParams,
      }).getReserveConfig();

      const updateReserveIx = updateEntireReserveConfigIx(
        marketWithAddress,
        reserveKeypair.publicKey,
        assetReserveConfig,
        klendBankrunProgram.programId
      );

      const updateReserveTx = new Transaction().add(
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
        updateReserveIx
      );

      await processBankrunTransaction(bankrunContext, updateReserveTx, [
        groupAdmin.wallet,
      ]);

      if (verbose) {
        console.log("Updated reserve config to operational");
      }

      // Step 3: Create marginfi Kamino bank
      const seed = new BN(STARTING_SEED + i);
      const config = defaultKaminoBankConfig(oracles.tokenAOracle.publicKey);

      const [bankKey] = deriveBankWithSeed(
        groupAdmin.mrgnBankrunProgram.programId,
        kaminoGroup.publicKey,
        mint,
        seed
      );

      const createBankTx = new Transaction().add(
        await makeAddKaminoBankIx(
          groupAdmin.mrgnBankrunProgram,
          {
            group: kaminoGroup.publicKey,
            feePayer: groupAdmin.wallet.publicKey,
            bankMint: mint,
            kaminoReserve: reserveKeypair.publicKey,
            kaminoMarket: marketKeypair.publicKey,
            oracle: oracles.tokenAOracle.publicKey,
          },
          {
            config,
            seed,
          }
        )
      );

      await processBankrunTransaction(bankrunContext, createBankTx, [
        groupAdmin.wallet,
      ]);

      if (verbose) {
        console.log("Created Kamino bank:", bankKey.toBase58());
      }

      // Step 4: Initialize obligation for the Kamino bank
      const initObligationTx = new Transaction().add(
        ComputeBudgetProgram.setComputeUnitLimit({ units: 2_000_000 }),
        await makeInitObligationIx(
          groupAdmin.mrgnBankrunProgram,
          {
            feePayer: groupAdmin.wallet.publicKey,
            bank: bankKey,
            signerTokenAccount: groupAdmin.tokenAAccount,
            lendingMarket: marketKeypair.publicKey,
            reserveLiquidityMint: mint,
            pythOracle: oracles.tokenAOracle.publicKey,
          },
          new BN(100)
        )
      );

      await processBankrunTransaction(bankrunContext, initObligationTx, [
        groupAdmin.wallet,
      ]);

      kaminoBanks.push(bankKey);
      kaminoMarkets.push(marketKeypair.publicKey);
      kaminoReserves.push(reserveKeypair.publicKey);

      if (verbose) {
        console.log(
          `Created and initialized bank #${i}: ${bankKey.toBase58()}`
        );
      }
    }

    if (verbose) {
      console.log(
        `\n✅ Successfully created ${MAX_KAMINO_DEPOSITS} markets, reserves, and banks`
      );
    }
  });

  it("(user 0) Create marginfi account", async () => {
    const { accountInit } = await import("./utils/user-instructions");

    const accountKeypair = Keypair.generate();
    const tx = new Transaction().add(
      await accountInit(users[0].mrgnProgram, {
        marginfiGroup: kaminoGroup.publicKey,
        marginfiAccount: accountKeypair.publicKey,
        authority: users[0].wallet.publicKey,
        feePayer: users[0].wallet.publicKey,
      })
    );

    await processBankrunTransaction(bankrunContext, tx, [
      users[0].wallet,
      accountKeypair,
    ]);

    users[0].accounts.set(USER_ACCOUNT, accountKeypair.publicKey);

    if (verbose) {
      console.log(
        `Created marginfi account for user 0: ${accountKeypair.publicKey.toBase58()}`
      );
    }
  });

  it("(user 0) Deposits into all 15 Kamino banks", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    const depositAmount = new BN(10 * 10 ** ecosystem.tokenADecimals);

    if (verbose) {
      console.log(`\nDepositing into ${MAX_KAMINO_DEPOSITS} Kamino banks...`);
    }

    // Deposit into each bank sequentially
    for (let i = 0; i < MAX_KAMINO_DEPOSITS; i++) {
      const bank = kaminoBanks[i];
      const market = kaminoMarkets[i];
      const reserve = kaminoReserves[i];

      // Derive the obligation for this bank
      const [liquidityVaultAuthority] = deriveLiquidityVaultAuthority(
        groupAdmin.mrgnBankrunProgram.programId,
        bank
      );
      const [obligation] = deriveBaseObligation(
        liquidityVaultAuthority,
        market
      );

      const tx = new Transaction().add(
        await simpleRefreshReserve(
          klendBankrunProgram,
          reserve,
          market,
          oracles.tokenAOracle.publicKey
        ),
        await simpleRefreshObligation(klendBankrunProgram, market, obligation, [
          reserve,
        ]),
        await makeKaminoDepositIx(
          user.mrgnBankrunProgram,
          {
            marginfiAccount: userAccount,
            bank: bank,
            signerTokenAccount: user.tokenAAccount,
            lendingMarket: market,
            reserveLiquidityMint: ecosystem.tokenAMint.publicKey,
          },
          depositAmount
        )
      );

      await processBankrunTransaction(bankrunContext, tx, [user.wallet]);

      if (verbose) {
        console.log(`Deposited into bank #${i}: ${bank.toBase58()}`);
      }
    }

    if (verbose) {
      console.log(
        `\n✅ Successfully deposited into all ${MAX_KAMINO_DEPOSITS} Kamino banks`
      );
    }
  });

  it("(admin) Create admin account on kaminoGroup", async () => {
    const { accountInit } = await import("./utils/user-instructions");

    const accountKeypair = Keypair.generate();
    const tx = new Transaction().add(
      await accountInit(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: kaminoGroup.publicKey,
        marginfiAccount: accountKeypair.publicKey,
        authority: groupAdmin.wallet.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
      })
    );

    await processBankrunTransaction(bankrunContext, tx, [
      groupAdmin.wallet,
      accountKeypair,
    ]);

    groupAdmin.accounts.set(USER_ACCOUNT, accountKeypair.publicKey);

    if (verbose) {
      console.log(
        `Created admin account: ${accountKeypair.publicKey.toBase58()}`
      );
    }
  });

  it("(admin) Create regular USDC bank for borrowing", async () => {
    const { addBankWithSeed } = await import("./utils/group-instructions");
    const { defaultBankConfig, ORACLE_SETUP_PYTH_PUSH } = await import(
      "./utils/types"
    );
    const { bigNumberToWrappedI80F48 } = await import("@mrgnlabs/mrgn-common");

    const seed = new BN(STARTING_SEED + 100);
    const [bankKey] = deriveBankWithSeed(
      groupAdmin.mrgnBankrunProgram.programId,
      kaminoGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      seed
    );

    const config = defaultBankConfig();
    config.assetWeightInit = bigNumberToWrappedI80F48(0.5);
    config.assetWeightMaint = bigNumberToWrappedI80F48(0.6);
    config.depositLimit = new BN(100_000_000_000_000);
    config.borrowLimit = new BN(100_000_000_000_000);

    const configOracleIx = await groupAdmin.mrgnBankrunProgram.methods
      .lendingPoolConfigureBankOracle(
        ORACLE_SETUP_PYTH_PUSH,
        oracles.usdcOracle.publicKey
      )
      .accountsPartial({
        group: kaminoGroup.publicKey,
        bank: bankKey,
        admin: groupAdmin.wallet.publicKey,
      })
      .remainingAccounts([
        {
          pubkey: oracles.usdcOracle.publicKey,
          isSigner: false,
          isWritable: false,
        },
      ])
      .instruction();

    const tx = new Transaction().add(
      await addBankWithSeed(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: kaminoGroup.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.usdcMint.publicKey,
        config,
        seed,
      }),
      configOracleIx
    );

    await processBankrunTransaction(bankrunContext, tx, [groupAdmin.wallet]);

    regularBank = bankKey;

    if (verbose) {
      console.log(`Created regular USDC bank: ${bankKey.toBase58()}`);
    }
  });

  it("(admin) Seed liquidity in USDC bank", async () => {
    const { depositIx } = await import("./utils/user-instructions");
    const { createMintToInstruction } = await import("@solana/spl-token");

    const depositAmount = new BN(100_000 * 10 ** ecosystem.usdcDecimals);

    const tx = new Transaction().add(
      createMintToInstruction(
        ecosystem.usdcMint.publicKey,
        groupAdmin.usdcAccount,
        globalProgramAdmin.wallet.publicKey,
        10_000_000 * 10 ** ecosystem.usdcDecimals
      ),
      await depositIx(groupAdmin.mrgnBankrunProgram, {
        marginfiAccount: groupAdmin.accounts.get(USER_ACCOUNT),
        bank: regularBank,
        tokenAccount: groupAdmin.usdcAccount,
        amount: depositAmount,
      })
    );

    await processBankrunTransaction(bankrunContext, tx, [
      globalProgramAdmin.wallet,
      groupAdmin.wallet,
    ]);

    if (verbose) {
      console.log(`Seeded USDC bank with ${depositAmount.toString()} tokens`);
    }
  });

  it("(user 1) Create Address Lookup Table for liquidation", async () => {
    const { getBankrunBlockhash } = await import("./utils/spl-staking-utils");
    const { getEpochAndSlot } = await import("./utils/stake-utils");

    const user = users[1];

    // Step 1: Create the LUT
    const recentSlot = Number(await banksClient.getSlot());
    const [createLutIx, lutAddr] = AddressLookupTableProgram.createLookupTable({
      authority: user.wallet.publicKey,
      payer: user.wallet.publicKey,
      recentSlot: recentSlot - 1,
    });

    lutAddress = lutAddr;

    let createLutTx = new Transaction().add(createLutIx);
    createLutTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    createLutTx.sign(user.wallet);
    await banksClient.processTransaction(createLutTx);

    if (verbose) {
      console.log(`Created LUT: ${lutAddress.toBase58()}`);
    }

    // Step 2: Extend the LUT with all accounts (split into chunks)
    const allAddresses: PublicKey[] = [];

    // Add all 15 Kamino banks, oracles, and reserves
    for (let i = 0; i < MAX_KAMINO_DEPOSITS; i++) {
      allAddresses.push(kaminoBanks[i]);
      allAddresses.push(oracles.tokenAOracle.publicKey);
      allAddresses.push(kaminoReserves[i]);
    }

    // Add the regular USDC bank and oracle
    allAddresses.push(regularBank);
    allAddresses.push(oracles.usdcOracle.publicKey);

    if (verbose) {
      console.log(`Extending LUT with ${allAddresses.length} addresses...`);
    }

    // Extend in chunks of 20 addresses to avoid transaction size limits
    const chunkSize = 20;
    for (let i = 0; i < allAddresses.length; i += chunkSize) {
      const chunk = allAddresses.slice(i, i + chunkSize);

      let extendLutTx = new Transaction().add(
        AddressLookupTableProgram.extendLookupTable({
          authority: user.wallet.publicKey,
          payer: user.wallet.publicKey,
          lookupTable: lutAddress,
          addresses: chunk,
        })
      );
      extendLutTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      extendLutTx.sign(user.wallet);
      await banksClient.processTransaction(extendLutTx);

      if (verbose) {
        console.log(
          `Extended LUT with chunk ${i / chunkSize + 1} (${
            chunk.length
          } addresses)`
        );
      }
    }

    // Step 3: Activate the LUT by warping slot forward
    const ONE_MINUTE = 60;
    const slotsToAdvance = ONE_MINUTE * 0.4; // ~24 slots
    let { epoch: _, slot } = await getEpochAndSlot(banksClient);
    bankrunContext.warpToSlot(BigInt(slot + slotsToAdvance));

    if (verbose) {
      console.log(`Warped ${slotsToAdvance} slots forward to activate LUT`);
      console.log(`✅ LUT ready with ${allAddresses.length} addresses`);
    }
  });

  it("(user 0) Borrow USDC from regular bank using LUT", async () => {
    const { borrowIx, composeRemainingAccounts } = await import(
      "./utils/user-instructions"
    );
    const { getBankrunBlockhash } = await import("./utils/spl-staking-utils");

    // Refresh oracles after slot warp
    await refreshPullOraclesBankrun(oracles, bankrunContext, banksClient);

    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    const borrowAmount = new BN(1000 * 10 ** ecosystem.usdcDecimals);

    // Batch refresh all 15 Kamino reserves instruction
    const reserveAccounts: {
      pubkey: PublicKey;
      isSigner: boolean;
      isWritable: boolean;
    }[] = [];
    for (let i = 0; i < MAX_KAMINO_DEPOSITS; i++) {
      reserveAccounts.push({
        pubkey: kaminoReserves[i],
        isSigner: false,
        isWritable: true,
      });
      reserveAccounts.push({
        pubkey: kaminoMarkets[i],
        isSigner: false,
        isWritable: false,
      });
    }

    const batchRefreshIx = await klendBankrunProgram.methods
      .refreshReservesBatch(true) // skip_price_updates = true (oracles already refreshed)
      .remainingAccounts(reserveAccounts)
      .instruction();

    // Build remaining accounts: all 15 Kamino banks + oracles + reserves, plus the USDC bank
    const remainingAccounts: PublicKey[][] = [];
    for (let i = 0; i < MAX_KAMINO_DEPOSITS; i++) {
      remainingAccounts.push([
        kaminoBanks[i],
        oracles.tokenAOracle.publicKey,
        kaminoReserves[i],
      ]);
    }
    // TODO WTF? It's definitely 8624 in prod
    // (https://solscan.io/account/d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q), where are we
    // getting extra bytes from? 
    const res = await bankRunProvider.connection.getAccountInfo(
      kaminoReserves[0]
    );
    console.log("res size: " + res.data.length);
    remainingAccounts.push([regularBank, oracles.usdcOracle.publicKey]);

    // Create the borrow instruction
    const borrowInstruction = await borrowIx(user.mrgnBankrunProgram, {
      marginfiAccount: userAccount,
      bank: regularBank,
      tokenAccount: user.usdcAccount,
      remaining: composeRemainingAccounts(remainingAccounts),
      amount: borrowAmount,
    });

    // Fetch the activated LUT
    const lutRaw = await banksClient.getAccount(lutAddress);
    const lutState = AddressLookupTableAccount.deserialize(lutRaw.data);
    lut = new AddressLookupTableAccount({
      key: lutAddress,
      state: lutState,
    });

    // Add compute budget instruction
    const computeBudgetIx = ComputeBudgetProgram.setComputeUnitLimit({
      units: 1_400_000,
    });

    // Create versioned transaction with LUT (compute budget + batch refresh + borrow)
    const messageV0 = new TransactionMessage({
      payerKey: user.wallet.publicKey,
      recentBlockhash: await getBankrunBlockhash(bankrunContext),
      instructions: [computeBudgetIx, batchRefreshIx, borrowInstruction],
    }).compileToV0Message([lut]);

    const versionedTx = new VersionedTransaction(messageV0);
    versionedTx.sign([user.wallet]);
    let result = await banksClient.tryProcessTransaction(versionedTx);
    dumpBankrunLogs(result);

    if (verbose) {
      console.log(`User 0 borrowed ${borrowAmount.toString()} USDC using LUT`);
    }
  });

  it("(admin) Make user 0 unhealthy by increasing USDC bank liability ratio", async () => {
    const { configureBank } = await import("./utils/group-instructions");
    const { blankBankConfigOptRaw } = await import("./utils/types");
    const { bigNumberToWrappedI80F48 } = await import("@mrgnlabs/mrgn-common");
    const { healthPulse, composeRemainingAccounts } = await import(
      "./utils/user-instructions"
    );
    const { getBankrunBlockhash } = await import("./utils/spl-staking-utils");

    let config = blankBankConfigOptRaw();
    config.liabilityWeightInit = bigNumberToWrappedI80F48(1.7); // 170%
    config.liabilityWeightMaint = bigNumberToWrappedI80F48(1.6); // 160%

    let tx = new Transaction().add(
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: regularBank,
        bankConfigOpt: config,
      })
    );
    await processBankrunTransaction(bankrunContext, tx, [groupAdmin.wallet]);

    // Health pulse to update cache and verify user is unhealthy (using LUT)
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    const positionAccounts: PublicKey[][] = [];
    for (let i = 0; i < MAX_KAMINO_DEPOSITS; i++) {
      positionAccounts.push([
        kaminoBanks[i],
        oracles.tokenAOracle.publicKey,
        kaminoReserves[i],
      ]);
    }
    positionAccounts.push([regularBank, oracles.usdcOracle.publicKey]);

    const healthPulseIx = await healthPulse(user.mrgnBankrunProgram, {
      marginfiAccount: userAccount,
      remaining: composeRemainingAccounts(positionAccounts),
    });

    const computeBudgetIx = ComputeBudgetProgram.setComputeUnitLimit({
      units: 1_400_000,
    });

    const messageV0 = new TransactionMessage({
      payerKey: user.wallet.publicKey,
      recentBlockhash: await getBankrunBlockhash(bankrunContext),
      instructions: [computeBudgetIx, healthPulseIx],
    }).compileToV0Message([lut]);

    const versionedTx = new VersionedTransaction(messageV0);
    versionedTx.sign([user.wallet]);
    await banksClient.processTransaction(versionedTx);

    if (verbose) {
      console.log(
        "Increased USDC bank liability ratio - checking if user 0 is unhealthy"
      );
    }
  });

  it("(user 1) Create marginfi account and deposit collateral", async () => {
    const { accountInit } = await import("./utils/user-instructions");
    const { depositIx } = await import("./utils/user-instructions");

    const user = users[1];

    // Create account
    const accountKeypair = Keypair.generate();
    let tx = new Transaction().add(
      await accountInit(user.mrgnProgram, {
        marginfiGroup: kaminoGroup.publicKey,
        marginfiAccount: accountKeypair.publicKey,
        authority: user.wallet.publicKey,
        feePayer: user.wallet.publicKey,
      })
    );
    await processBankrunTransaction(bankrunContext, tx, [
      user.wallet,
      accountKeypair,
    ]);
    user.accounts.set(USER_ACCOUNT, accountKeypair.publicKey);

    if (verbose) {
      console.log(
        `Created liquidator account: ${accountKeypair.publicKey.toBase58()}`
      );
    }

    // Deposit some USDC as collateral for liquidation
    const depositAmount = new BN(10_000 * 10 ** ecosystem.usdcDecimals);
    tx = new Transaction().add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: accountKeypair.publicKey,
        bank: regularBank,
        tokenAccount: user.usdcAccount,
        amount: depositAmount,
      })
    );
    await processBankrunTransaction(bankrunContext, tx, [user.wallet]);

    if (verbose) {
      console.log(`Deposited ${depositAmount.toString()} USDC for liquidation`);
    }
  });

  it("(user 1) Find max Kamino deposits for liquidation", async () => {
    const { liquidateIx, composeRemainingAccounts } = await import(
      "./utils/user-instructions"
    );
    const { getBankrunBlockhash } = await import("./utils/spl-staking-utils");
    const { dumpBankrunLogs } = await import("./utils/tools");

    console.log("reg bank: " + regularBank);

    const liquidator = users[1];
    const liquidatorAccount = liquidator.accounts.get(USER_ACCOUNT);
    const accliqor =
      await liquidator.mrgnBankrunProgram.account.marginfiAccount.fetch(
        liquidatorAccount
      );
    dumpAccBalances(accliqor);
    const liquidatee = users[0];
    const liquidateeAccount = liquidatee.accounts.get(USER_ACCOUNT);
    const accliqee =
      await liquidator.mrgnBankrunProgram.account.marginfiAccount.fetch(
        liquidateeAccount
      );
    dumpAccBalances(accliqee);

    // Pick one Kamino bank to liquidate (bank 0)
    const assetBankKey = kaminoBanks[0];
    const assetReserve = kaminoReserves[0];
    const liquidateAmount = new BN(5 * 10 ** ecosystem.tokenADecimals);

    // Fetch the LUT once
    const lutRaw = await banksClient.getAccount(lutAddress);
    const lutState = AddressLookupTableAccount.deserialize(lutRaw.data);
    const lut = new AddressLookupTableAccount({
      key: lutAddress,
      state: lutState,
    });

    console.log(
      "\n========== TESTING LIQUIDATION WITH DIFFERENT ACCOUNT COUNTS =========="
    );

    // TODO this decrease-by-1-thing doesn't work, you would have to first withdraw a position as
    // the user to reduce the position count. Might as well remove the for-loop, etc. Either it works with 16 or it doesn't.

    // Try different numbers of Kamino deposits, from 15 down to 1
    let maxWorkingDeposits = 0;
    for (let numDeposits = 15; numDeposits >= 15; numDeposits--) {
      // Build position accounts with only numDeposits Kamino banks
      const positionAccounts: PublicKey[][] = [];
      for (let i = 0; i < numDeposits; i++) {
        positionAccounts.push([
          kaminoBanks[i],
          oracles.tokenAOracle.publicKey,
          kaminoReserves[i],
        ]);
      }
      positionAccounts.push([regularBank, oracles.usdcOracle.publicKey]);

      // Build remaining accounts: asset oracle + reserve + liability oracle + liquidator accounts + liquidatee accounts
      const remainingForLiq: PublicKey[] = [
        oracles.tokenAOracle.publicKey, // asset oracle
        assetReserve, // asset reserve
        oracles.usdcOracle.publicKey, // liability oracle
      ];

      // Add liquidator accounts
      remainingForLiq.push(
        ...composeRemainingAccounts([
          // Existing collateral position
          [regularBank, oracles.usdcOracle.publicKey],
          // Gains this liability position....
          [assetBankKey, oracles.tokenAOracle.publicKey, assetReserve],
        ])
      );

      // Add liquidatee accounts
      remainingForLiq.push(...composeRemainingAccounts(positionAccounts));

      const liquidateInstruction = await liquidateIx(
        liquidator.mrgnBankrunProgram,
        {
          assetBankKey: assetBankKey,
          liabilityBankKey: regularBank,
          liquidatorMarginfiAccount: liquidatorAccount,
          liquidateeMarginfiAccount: liquidateeAccount,
          remaining: remainingForLiq,
          amount: liquidateAmount,
        }
      );

      // Add compute budget
      const computeBudgetIx = ComputeBudgetProgram.setComputeUnitLimit({
        units: 2_000_000,
      });

      // Create versioned transaction with LUT
      const messageV0 = new TransactionMessage({
        payerKey: liquidator.wallet.publicKey,
        recentBlockhash: await getBankrunBlockhash(bankrunContext),
        instructions: [computeBudgetIx, liquidateInstruction],
      }).compileToV0Message([lut]);

      const versionedTx = new VersionedTransaction(messageV0);
      versionedTx.sign([liquidator.wallet]);

      // Try the transaction
      const result = await banksClient.tryProcessTransaction(versionedTx);

      const hasOOMError = result.meta?.logMessages?.some((log) =>
        log.includes("out of memory")
      );

      if (result.result === null && !hasOOMError) {
        console.log(`✅ ${numDeposits} Kamino deposits: SUCCESS`);
        maxWorkingDeposits = numDeposits;
        break;
      } else {
        if (hasOOMError) {
          console.log(`❌ ${numDeposits} Kamino deposits: OUT OF MEMORY`);
        } else {
          console.log(
            `❌ ${numDeposits} Kamino deposits: FAILED - ${
              result.meta?.logMessages?.[result.meta.logMessages.length - 1] ||
              "unknown error"
            }`
          );
        }

        // Dump full logs for specific failing configurations
        if (
          numDeposits === 15 ||
          numDeposits === 10 ||
          numDeposits === 5 ||
          numDeposits === 1
        ) {
          console.log(`\n--- Full logs for ${numDeposits} Kamino deposits ---`);
          dumpBankrunLogs(result);
          console.log(`--- End logs for ${numDeposits} Kamino deposits ---\n`);
        }
      }
    }

    console.log(
      `\n📊 Maximum working Kamino deposits for regular liquidate: ${maxWorkingDeposits}`
    );
    console.log(
      "======================================================================\n"
    );

    if (maxWorkingDeposits === 0) {
      throw new Error("Could not find a working configuration for liquidation");
    }
  });
});
