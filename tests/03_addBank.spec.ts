import { BN, Program, workspace } from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import { addBank } from "./utils/group-instructions";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairUsdc,
  ecosystem,
  globalFeeWallet,
  groupAdmin,
  INIT_POOL_ORIGINATION_FEE,
  marginfiGroup,
  oracles,
  printBuffers,
  verbose,
} from "./rootHooks";
import {
  assertBNEqual,
  assertI80F48Approx,
  assertI80F48Equal,
  assertKeyDefault,
  assertKeysEqual,
} from "./utils/genericTests";
import { ASSET_TAG_DEFAULT, defaultBankConfig } from "./utils/types";
import {
  deriveLiquidityVaultAuthority,
  deriveLiquidityVault,
  deriveInsuranceVaultAuthority,
  deriveInsuranceVault,
  deriveFeeVaultAuthority,
  deriveFeeVault,
} from "./utils/pdas";
import { assert } from "chai";
import { printBufferGroups } from "./utils/tools";

describe("Lending pool add bank (add bank to group)", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  it("(admin) Add bank (USDC) - happy path", async () => {
    let setConfig = defaultBankConfig(oracles.usdcOracle.publicKey);
    let bankKey = bankKeypairUsdc.publicKey;
    const now = Date.now() / 1000;

    const feeAccSolBefore = await program.provider.connection.getBalance(
      globalFeeWallet
    );

    await groupAdmin.mrgnProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await addBank(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: ecosystem.usdcMint.publicKey,
          bank: bankKey,
          // globalFeeWallet: globalFeeWallet,
          config: setConfig,
        })
      ),
      [bankKeypairUsdc]
    );

    const feeAccSolAfter = await program.provider.connection.getBalance(
      globalFeeWallet
    );

    if (verbose) {
      console.log("*init USDC bank " + bankKey);
      console.log(
        " Origination fee collected: " + (feeAccSolAfter - feeAccSolBefore)
      );
    }

    assert.equal(feeAccSolAfter - feeAccSolBefore, INIT_POOL_ORIGINATION_FEE);

    let bankData = (
      await program.provider.connection.getAccountInfo(bankKey)
    ).data.subarray(8);
    if (printBuffers) {
      printBufferGroups(bankData, 16, 896);
    }

    const bank = await program.account.bank.fetch(bankKey);
    const config = bank.config;
    const interest = config.interestRateConfig;
    const id = program.programId;

    assertKeysEqual(bank.mint, ecosystem.usdcMint.publicKey);
    assert.equal(bank.mintDecimals, ecosystem.usdcDecimals);
    assertKeysEqual(bank.group, marginfiGroup.publicKey);

    // Keys and bumps...
    assertKeysEqual(config.oracleKeys[0], oracles.usdcOracle.publicKey);

    const [_liqAuth, liqAuthBump] = deriveLiquidityVaultAuthority(id, bankKey);
    const [liquidityVault, liqVaultBump] = deriveLiquidityVault(id, bankKey);
    assertKeysEqual(bank.liquidityVault, liquidityVault);
    assert.equal(bank.liquidityVaultBump, liqVaultBump);
    assert.equal(bank.liquidityVaultAuthorityBump, liqAuthBump);

    const [_insAuth, insAuthBump] = deriveInsuranceVaultAuthority(id, bankKey);
    const [insuranceVault, insurVaultBump] = deriveInsuranceVault(id, bankKey);
    assertKeysEqual(bank.insuranceVault, insuranceVault);
    assert.equal(bank.insuranceVaultBump, insurVaultBump);
    assert.equal(bank.insuranceVaultAuthorityBump, insAuthBump);

    const [_feeVaultAuth, feeAuthBump] = deriveFeeVaultAuthority(id, bankKey);
    const [feeVault, feeVaultBump] = deriveFeeVault(id, bankKey);
    assertKeysEqual(bank.feeVault, feeVault);
    assert.equal(bank.feeVaultBump, feeVaultBump);
    assert.equal(bank.feeVaultAuthorityBump, feeAuthBump);

    assertKeyDefault(bank.emissionsMint);

    // Constants/Defaults...
    assertI80F48Equal(bank.assetShareValue, 1);
    assertI80F48Equal(bank.liabilityShareValue, 1);
    assertI80F48Equal(bank.collectedInsuranceFeesOutstanding, 0);
    assertI80F48Equal(bank.collectedGroupFeesOutstanding, 0);
    assertI80F48Equal(bank.totalLiabilityShares, 0);
    assertI80F48Equal(bank.totalAssetShares, 0);
    assertBNEqual(bank.flags, 0);
    assertBNEqual(bank.emissionsRate, 0);
    assertI80F48Equal(bank.emissionsRemaining, 0);

    // Settings and non-default values...
    let lastUpdate = bank.lastUpdate.toNumber();
    assert.approximately(now, lastUpdate, 2);
    assertI80F48Equal(config.assetWeightInit, 1);
    assertI80F48Equal(config.assetWeightMaint, 1);
    assertI80F48Equal(config.liabilityWeightInit, 1);
    assertI80F48Equal(config.liabilityWeightMaint, 1);
    assertBNEqual(config.depositLimit, 100_000_000_000);

    const tolerance = 0.000001;
    assertI80F48Approx(interest.optimalUtilizationRate, 0.5, tolerance);
    assertI80F48Approx(interest.plateauInterestRate, 0.6, tolerance);
    assertI80F48Approx(interest.maxInterestRate, 3, tolerance);

    assertI80F48Approx(interest.insuranceFeeFixedApr, 0.01, tolerance);
    assertI80F48Approx(interest.insuranceIrFee, 0.02, tolerance);
    assertI80F48Approx(interest.protocolFixedFeeApr, 0.03, tolerance);
    assertI80F48Approx(interest.protocolIrFee, 0.04, tolerance);
    assertI80F48Approx(interest.protocolOriginationFee, 0.01, tolerance);

    assertI80F48Approx(interest.protocolOriginationFee, 0.01, tolerance);

    assert.deepEqual(config.operationalState, { operational: {} });
    assert.deepEqual(config.oracleSetup, { pythLegacy: {} });
    assertBNEqual(config.borrowLimit, 100_000_000_000);
    assert.deepEqual(config.riskTier, { collateral: {} });
    assert.equal(config.assetTag, ASSET_TAG_DEFAULT);
    assertBNEqual(config.totalAssetValueInitLimit, 1_000_000_000_000);
    assert.equal(config.oracleMaxAge, 240);

    assertI80F48Equal(bank.collectedProgramFeesOutstanding, 0);
  });

  it("(admin) Add bank (token A) - happy path", async () => {
    let config = defaultBankConfig(oracles.tokenAOracle.publicKey);
    let bankKey = bankKeypairA.publicKey;

    await groupAdmin.mrgnProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await addBank(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: ecosystem.tokenAMint.publicKey,
          bank: bankKey,
          // globalFeeWallet: globalFeeWallet,
          config: config,
        })
      ),
      [bankKeypairA]
    );

    if (verbose) {
      console.log("*init token A bank " + bankKey);
    }
  });

  it("Decodes a mainnet bank configured before manual padding", async () => {
    // mainnet program ID
    const id = new PublicKey("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");
    const group = new PublicKey("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8");

    let bonkBankKey = new PublicKey(
      "DeyH7QxWvnbbaVB4zFrf4hoq7Q8z1ZT14co42BGwGtfM"
    );
    let bonkBankData = (
      await program.provider.connection.getAccountInfo(bonkBankKey)
    ).data.subarray(8);
    if (printBuffers) {
      printBufferGroups(bonkBankData, 16, 896);
    }

    let cloudBankKey = new PublicKey(
      "4kNXetv8hSv9PzvzPZzEs1CTH6ARRRi2b8h6jk1ad1nP"
    );
    let cloudBankData = (
      await program.provider.connection.getAccountInfo(cloudBankKey)
    ).data.subarray(8);
    if (printBuffers) {
      printBufferGroups(cloudBankData, 16, 896);
    }

    const bbk = bonkBankKey;
    const bb = await program.account.bank.fetch(bonkBankKey);
    const bonkConfig = bb.config;
    const bonkInterest = bonkConfig.interestRateConfig;

    assertKeysEqual(
      bb.mint,
      new PublicKey("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263")
    );
    assert.equal(bb.mintDecimals, 5);
    assertKeysEqual(bb.group, group);

    const [_liqAu_bb, liqAuBmp_bb] = deriveLiquidityVaultAuthority(id, bbk);
    const [liquidityVault_bb, liqVaultBump_bb] = deriveLiquidityVault(id, bbk);
    assertKeysEqual(bb.liquidityVault, liquidityVault_bb);
    assert.equal(bb.liquidityVaultBump, liqVaultBump_bb);
    assert.equal(bb.liquidityVaultAuthorityBump, liqAuBmp_bb);

    const [_insAu_bb, insAuBmp_bb] = deriveInsuranceVaultAuthority(id, bbk);
    const [insVault_bb, insVaultBump_bb] = deriveInsuranceVault(id, bbk);
    assertKeysEqual(bb.insuranceVault, insVault_bb);
    assert.equal(bb.insuranceVaultBump, insVaultBump_bb);
    assert.equal(bb.insuranceVaultAuthorityBump, insAuBmp_bb);

    const [_feeVaultAuth_bb, feeAuthBump_bb] = deriveFeeVaultAuthority(id, bbk);
    const [feeVault_bb, feeVaultBump_bb] = deriveFeeVault(id, bbk);
    assertKeysEqual(bb.feeVault, feeVault_bb);
    assert.equal(bb.feeVaultBump, feeVaultBump_bb);
    assert.equal(bb.feeVaultAuthorityBump, feeAuthBump_bb);

    assertKeyDefault(bb.emissionsMint);

    // Constants/Defaults...
    // assertI80F48Equal(bank.assetShareValue, 1);
    // assertI80F48Equal(bank.liabilityShareValue, 1);
    // assertI80F48Equal(bank.collectedInsuranceFeesOutstanding, 0);
    // assertI80F48Equal(bank.collectedGroupFeesOutstanding, 0);
    // assertI80F48Equal(bank.totalLiabilityShares, 0);
    // assertI80F48Equal(bank.totalAssetShares, 0);
    assertBNEqual(bb.flags, 0);
    assertBNEqual(bb.emissionsRate, 0);
    assertI80F48Equal(bb.emissionsRemaining, 0);

    // Settings and non-default values...
    // let lastUpdate = bank.lastUpdate.toNumber();
    // assert.approximately(now, lastUpdate, 2);
    // assertI80F48Equal(config.assetWeightInit, 1);
    // assertI80F48Equal(config.assetWeightMaint, 1);
    // assertI80F48Equal(config.liabilityWeightInit, 1);

    // 1 trillion BONK with 5 decimals (100_000_000_000_000_000)
    assertBNEqual(bonkConfig.depositLimit, new BN("100000000000000000"));

    // assertI80F48Approx(interest.optimalUtilizationRate, 0.5, tolerance);
    // assertI80F48Approx(interest.plateauInterestRate, 0.6, tolerance);
    // assertI80F48Approx(interest.maxInterestRate, 3, tolerance);
    // assertI80F48Equal(interest.insuranceFeeFixedApr, 0);
    // assertI80F48Equal(interest.insuranceIrFee, 0);
    // assertI80F48Equal(interest.protocolFixedFeeApr, 0);
    // assertI80F48Equal(interest.protocolIrFee, 0);

    // Bank added before this feature existed, should be zero
    assertI80F48Equal(bonkInterest.protocolOriginationFee, 0);

    assert.deepEqual(bonkConfig.operationalState, { operational: {} });
    assert.deepEqual(bonkConfig.oracleSetup, { pythPushOracle: {} });
    // roughly 26.41 billion BONK with 5 decimals.
    assertBNEqual(bonkConfig.borrowLimit, 2_640_570_785_700_000);
    assert.deepEqual(bonkConfig.riskTier, { collateral: {} });
    assertBNEqual(bonkConfig.totalAssetValueInitLimit, 38_866_899);
    assert.equal(bonkConfig.oracleMaxAge, 120);

    const cbk = cloudBankKey;
    const cb = await program.account.bank.fetch(cloudBankKey);
    const cloudConfig = cb.config;
    const cloudInterest = cloudConfig.interestRateConfig;

    assertKeysEqual(
      cb.mint,
      new PublicKey("CLoUDKc4Ane7HeQcPpE3YHnznRxhMimJ4MyaUqyHFzAu")
    );
    assert.equal(cb.mintDecimals, 9);
    assertKeysEqual(cb.group, group);

    const [_liqAu_cb, liqAuBmp_cb] = deriveLiquidityVaultAuthority(id, cbk);
    const [liquidityVault_cb, liqVaultBump_cb] = deriveLiquidityVault(id, cbk);
    assertKeysEqual(cb.liquidityVault, liquidityVault_cb);
    assert.equal(cb.liquidityVaultBump, liqVaultBump_cb);
    assert.equal(cb.liquidityVaultAuthorityBump, liqAuBmp_cb);

    const [_insAu_cb, insAuBmp_cb] = deriveInsuranceVaultAuthority(id, cbk);
    const [insVault_cb, insVaultBump_cb] = deriveInsuranceVault(id, cbk);
    assertKeysEqual(cb.insuranceVault, insVault_cb);
    assert.equal(cb.insuranceVaultBump, insVaultBump_cb);
    assert.equal(cb.insuranceVaultAuthorityBump, insAuBmp_cb);

    const [_feeVaultAuth_cb, feeAuthBump_cb] = deriveFeeVaultAuthority(id, cbk);
    const [feeVault_cb, feeVaultBump_cb] = deriveFeeVault(id, cbk);
    assertKeysEqual(cb.feeVault, feeVault_cb);
    assert.equal(cb.feeVaultBump, feeVaultBump_cb);
    assert.equal(cb.feeVaultAuthorityBump, feeAuthBump_cb);

    assertKeyDefault(cb.emissionsMint);

    assertBNEqual(cb.flags, 0);
    assertBNEqual(cb.emissionsRate, 0);
    assertI80F48Equal(cb.emissionsRemaining, 0);

    // 1 million CLOUD with 9 decimals (1_000_000_000_000_000)
    assertBNEqual(cloudConfig.depositLimit, 1_000_000_000_000_000);

    // Bank added before this feature existed, should be zero
    assertI80F48Equal(cloudInterest.protocolOriginationFee, 0);

    assert.deepEqual(cloudConfig.operationalState, { operational: {} });
    assert.deepEqual(cloudConfig.oracleSetup, { switchboardV2: {} });
    // 50,000 CLOUD with 9 decimals (50_000_000_000_000)
    assertBNEqual(cloudConfig.borrowLimit, 50_000_000_000_000);
    assert.deepEqual(cloudConfig.riskTier, { isolated: {} });
    assertBNEqual(cloudConfig.totalAssetValueInitLimit, 0);
    assert.equal(cloudConfig.oracleMaxAge, 60);

    // Assert emissions mint (one of the last fields) is also aligned correctly.
    let pyUsdcBankKey = new PublicKey(
      "Fe5QkKPVAh629UPP5aJ8sDZu8HTfe6M26jDQkKyXVhoA"
    );
    let pyUsdcBankData = (
      await program.provider.connection.getAccountInfo(pyUsdcBankKey)
    ).data.subarray(8);
    if (printBuffers) {
      printBufferGroups(pyUsdcBankData, 16, 896);
    }

    const pb = await program.account.bank.fetch(pyUsdcBankKey);
    assertKeysEqual(
      pb.emissionsMint,
      new PublicKey("2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo")
    );
  });
});

// TODO add bank with seed
