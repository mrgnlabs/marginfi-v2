import { BN } from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  Keypair,
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import {
  ecosystem,
  groupAdmin,
  driftAccounts,
  DRIFT_USDC_BANK,
  driftGroup,
  DRIFT_USDC_SPOT_MARKET,
  oracles,
  DRIFT_TOKENA_SPOT_MARKET,
  DRIFT_TOKENA_BANK,
  DRIFT_TOKENA_PULL_ORACLE,
  verbose,
  users,
  bankrunContext,
  bankRunProvider,
  bankrunProgram,
  driftBankrunProgram,
  globalProgramAdmin,
} from "./rootHooks";
import {
  assertBNEqual,
  assertI80F48Equal,
  assertKeysEqual,
  getTokenBalance,
  assertBankrunTxFailed,
} from "./utils/genericTests";
import {
  ASSET_TAG_DRIFT,
  defaultDriftBankConfig,
  DRIFT_SCALED_BALANCE_DECIMALS,
} from "./utils/drift-utils";
import { CLOSE_ENABLED_FLAG } from "./utils/types";
import { assert } from "chai";
import { processBankrunTransaction, safeGetAccountInfo } from "./utils/tools";
import { ProgramTestContext } from "solana-bankrun";
import {
  makeAddDriftBankIx,
  makeInitDriftUserIx,
} from "./utils/drift-instructions";
import {
  deriveBankWithSeed,
  deriveFeeVault,
  deriveInsuranceVault,
  deriveLiquidityVault,
  deriveLiquidityVaultAuthority,
  deriveUserPDA,
  deriveUserStatsPDA,
} from "./utils/pdas";
import { DRIFT_PROGRAM_ID } from "./utils/types";
import { deriveSpotMarketPDA } from "./utils/pdas";

let ctx: ProgramTestContext;
const seed = new BN(555);
let mrgnID: PublicKey;

let usdcSpotMarket: PublicKey;
let tokenASpotMarket: PublicKey;

describe("d06: Init Drift banks", () => {
  before(async () => {
    ctx = bankrunContext;
    mrgnID = bankrunProgram.programId;

    usdcSpotMarket = driftAccounts.get(DRIFT_USDC_SPOT_MARKET);
    tokenASpotMarket = driftAccounts.get(DRIFT_TOKENA_SPOT_MARKET);
  });

  it("(admin) Add Drift bank (drift USDC) and init user - happy path", async () => {
    let defaultConfig = defaultDriftBankConfig(oracles.usdcOracle.publicKey);
    const now = Date.now() / 1000;

    const [bankKey] = deriveBankWithSeed(
      mrgnID,
      driftGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      seed
    );

    const tx = new Transaction().add(
      await makeAddDriftBankIx(
        groupAdmin.mrgnBankrunProgram,
        {
          group: driftGroup.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: ecosystem.usdcMint.publicKey,
          driftSpotMarket: usdcSpotMarket,
          oracle: oracles.usdcOracle.publicKey,
        },
        {
          config: defaultConfig,
          seed: seed,
        }
      )
    );
    await processBankrunTransaction(ctx, tx, [groupAdmin.wallet], false, true);

    driftAccounts.set(DRIFT_USDC_BANK, bankKey);
    const initUserAmount = new BN(100); // 100 smallest units (0.0001 USDC)
    const initUserTx = new Transaction().add(
      await makeInitDriftUserIx(
        users[0].mrgnBankrunProgram,
        {
          feePayer: users[0].wallet.publicKey,
          bank: bankKey,
          signerTokenAccount: users[0].usdcAccount,
        },
        {
          amount: initUserAmount,
        },
        0 // USDC market index
      )
    );
    await processBankrunTransaction(
      ctx,
      initUserTx,
      [users[0].wallet],
      false,
      true
    );

    const group = await bankrunProgram.account.marginfiGroup.fetch(
      driftGroup.publicKey
    );
    assert.equal(group.banks, 1);

    const bank = await bankrunProgram.account.bank.fetch(bankKey);
    const config = bank.config;

    assertKeysEqual(bank.mint, ecosystem.usdcMint.publicKey);
    assert.equal(bank.mintDecimals, DRIFT_SCALED_BALANCE_DECIMALS);
    assertKeysEqual(bank.group, driftGroup.publicKey);

    assertKeysEqual(config.oracleKeys[0], oracles.usdcOracle.publicKey);

    assertBNEqual(bank.flags, CLOSE_ENABLED_FLAG);

    assertI80F48Equal(config.assetWeightInit, 1);
    assertI80F48Equal(config.assetWeightMaint, 1);
    assertBNEqual(
      config.depositLimit,
      new BN(100_000_000 * 10 ** ecosystem.usdcDecimals)
    ); // 100mil USDC because big interest needed

    assert.deepEqual(config.operationalState, { operational: {} });
    assert.deepEqual(config.oracleSetup, { driftPythPull: {} });
    assertBNEqual(config.borrowLimit, 0);
    assert.deepEqual(config.riskTier, { collateral: {} });
    assertBNEqual(
      config.totalAssetValueInitLimit,
      new BN(10_000_000 * 10 ** ecosystem.usdcDecimals)
    );
    assert.equal(config.oracleMaxAge, 100);

    const [liquidityVault] = deriveLiquidityVault(mrgnID, bankKey);
    assertKeysEqual(bank.liquidityVault, liquidityVault);
    const [insuranceVault] = deriveInsuranceVault(mrgnID, bankKey);
    assertKeysEqual(bank.insuranceVault, insuranceVault);
    const [feeVault] = deriveFeeVault(mrgnID, bankKey);
    assertKeysEqual(bank.feeVault, feeVault);

    assertKeysEqual(bank.driftSpotMarket, usdcSpotMarket);
    const [liquidityVaultAuthority] = deriveLiquidityVaultAuthority(
      mrgnID,
      bankKey
    );
    const [driftUser] = deriveUserPDA(
      DRIFT_PROGRAM_ID,
      liquidityVaultAuthority,
      0
    );
    const [driftUserStats] = deriveUserStatsPDA(
      DRIFT_PROGRAM_ID,
      liquidityVaultAuthority
    );
    assertKeysEqual(bank.driftUser, driftUser);
    assertKeysEqual(bank.driftUserStats, driftUserStats);
    assert.equal(config.assetTag, ASSET_TAG_DRIFT);
    assertKeysEqual(bank.config.oracleKeys[1], usdcSpotMarket);

    const usdcSpotMarketData =
      await driftBankrunProgram.account.spotMarket.fetch(usdcSpotMarket);
    assertKeysEqual(usdcSpotMarketData.mint, bank.mint);
    assert.equal(usdcSpotMarketData.decimals, ecosystem.usdcDecimals);
  });

  it("(admin) Init Token A bank and init user", async () => {
    const user = groupAdmin;
    let defaultConfig = defaultDriftBankConfig(oracles.tokenAOracle.publicKey);

    const [tokenABankKey] = deriveBankWithSeed(
      mrgnID,
      driftGroup.publicKey,
      ecosystem.tokenAMint.publicKey,
      seed
    );

    const tx = new Transaction().add(
      await makeAddDriftBankIx(
        user.mrgnBankrunProgram,
        {
          group: driftGroup.publicKey,
          feePayer: user.wallet.publicKey,
          bankMint: ecosystem.tokenAMint.publicKey,
          driftSpotMarket: tokenASpotMarket,
          oracle: oracles.tokenAOracle.publicKey,
        },
        {
          config: defaultConfig,
          seed: seed,
        }
      )
    );
    await processBankrunTransaction(ctx, tx, [user.wallet], false, true);
    driftAccounts.set(DRIFT_TOKENA_BANK, tokenABankKey);

    const initUserAmount = new BN(100);
    const initUserTx = new Transaction().add(
      await makeInitDriftUserIx(
        users[1].mrgnBankrunProgram,
        {
          feePayer: users[1].wallet.publicKey,
          bank: tokenABankKey,
          signerTokenAccount: users[1].tokenAAccount,
          driftOracle: driftAccounts.get(DRIFT_TOKENA_PULL_ORACLE)!,
        },
        {
          amount: initUserAmount,
        },
        1
      )
    );
    await processBankrunTransaction(
      ctx,
      initUserTx,
      [users[1].wallet],
      false,
      true
    );

    const bank = await bankrunProgram.account.bank.fetch(tokenABankKey);
    assert.equal(bank.mintDecimals, DRIFT_SCALED_BALANCE_DECIMALS);
  });

  it("(user 0) Tries to init bank - admin only, should fail", async () => {
    const throwawaySeed = new BN(999);
    const user = users[0];
    let defaultConfig = defaultDriftBankConfig(oracles.tokenAOracle.publicKey);

    const tx = new Transaction().add(
      await makeAddDriftBankIx(
        user.mrgnBankrunProgram,
        {
          group: driftGroup.publicKey,
          feePayer: user.wallet.publicKey,
          bankMint: ecosystem.tokenAMint.publicKey,
          driftSpotMarket: tokenASpotMarket,
          oracle: oracles.tokenAOracle.publicKey,
        },
        {
          config: defaultConfig,
          seed: throwawaySeed,
        }
      )
    );
    let result = await processBankrunTransaction(ctx, tx, [user.wallet], true);
    assertBankrunTxFailed(result, 2001);
  });

  it("(admin) Tries pass the wrong spot market/mint for this asset - should fail", async () => {
    const throwawaySeed = new BN(999);
    const usr = groupAdmin;
    let defaultConfig = defaultDriftBankConfig(oracles.tokenAOracle.publicKey);

    const tx1 = new Transaction().add(
      await makeAddDriftBankIx(
        usr.mrgnBankrunProgram,
        {
          group: driftGroup.publicKey,
          feePayer: usr.wallet.publicKey,
          bankMint: ecosystem.tokenAMint.publicKey,
          driftSpotMarket: usdcSpotMarket,
          oracle: oracles.tokenAOracle.publicKey,
        },
        {
          config: defaultConfig,
          seed: throwawaySeed,
        }
      )
    );
    let result1 = await processBankrunTransaction(
      ctx,
      tx1,
      [usr.wallet],
      true,
      false
    );
    // DriftSpotMarketMintMismatch
    assertBankrunTxFailed(result1, 6301);

    const tx2 = new Transaction().add(
      await makeAddDriftBankIx(
        usr.mrgnBankrunProgram,
        {
          group: driftGroup.publicKey,
          feePayer: usr.wallet.publicKey,
          bankMint: ecosystem.usdcMint.publicKey,
          driftSpotMarket: tokenASpotMarket, // wrong
          oracle: oracles.usdcOracle.publicKey,
        },
        {
          config: defaultConfig,
          seed: throwawaySeed,
        }
      )
    );
    let result2 = await processBankrunTransaction(
      ctx,
      tx2,
      [usr.wallet],
      true,
      false
    );
    // DriftSpotMarketMintMismatch
    assertBankrunTxFailed(result2, 6301);
  });

  it("(admin) Tries to use the wrong oracle type - should fail", async () => {
    const throwawaySeed = new BN(999);
    const usr = groupAdmin;
    let config1 = defaultDriftBankConfig(oracles.tokenAOracle.publicKey);
    config1.oracleSetup = {
      pythPushOracle: {},
    } as any; // Cast to any to test invalid oracle setup
    const tx1 = new Transaction().add(
      await makeAddDriftBankIx(
        usr.mrgnBankrunProgram,
        {
          group: driftGroup.publicKey,
          feePayer: usr.wallet.publicKey,
          bankMint: ecosystem.tokenAMint.publicKey,
          driftSpotMarket: tokenASpotMarket,
          oracle: oracles.tokenAOracle.publicKey,
        },
        {
          config: config1,
          seed: throwawaySeed,
        }
      )
    );
    let result1 = await processBankrunTransaction(
      ctx,
      tx1,
      [usr.wallet],
      true,
      false
    );
    assertBankrunTxFailed(result1, 6300);
  });

  it("(admin) Tries to re-init USDC bank when drift user already exists - should fail", async () => {
    const user = groupAdmin;
    let defaultConfig = defaultDriftBankConfig(oracles.usdcOracle.publicKey);

    const tx = new Transaction().add(
      await makeAddDriftBankIx(
        user.mrgnBankrunProgram,
        {
          group: driftGroup.publicKey,
          feePayer: user.wallet.publicKey,
          bankMint: ecosystem.usdcMint.publicKey,
          driftSpotMarket: usdcSpotMarket,
          oracle: oracles.usdcOracle.publicKey,
        },
        {
          config: defaultConfig,
          seed: seed,
        }
      )
    );

    const result = await processBankrunTransaction(
      ctx,
      tx,
      [user.wallet],
      true,
      false
    );

    // This should fail because the drift user and user stats accounts already exist
    // (0xbc3 = AccountNotSystemOwned.)
    assertBankrunTxFailed(result, 0xbc3);
  });
});
