import { Program, workspace } from "@coral-xyz/anchor";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { addBank, groupConfigure } from "./utils/instructions";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairUsdc,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
  verbose,
} from "./rootHooks";
import {
  assertBNEqual,
  assertI80F48Approx,
  assertI80F48Equal,
  assertKeyDefault,
  assertKeysEqual,
} from "./utils/genericTests";
import { defaultBankConfig, OracleSetup } from "./utils/types";
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

    await groupAdmin.userMarginProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await addBank(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: ecosystem.usdcMint.publicKey,
          bank: bankKey,
          config: setConfig,
        })
      ),
      [bankKeypairUsdc]
    );

    if (verbose) {
      console.log("*init USDC bank " + bankKey);
    }

    let bankData = (
      await program.provider.connection.getAccountInfo(bankKey)
    ).data.subarray(8);
    printBufferGroups(bankData, 8, 896);

    const bank = await program.account.bank.fetch(bankKey);
    const config = bank.config;
    const interest = config.interestRateConfig;
    const id = program.programId;

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
    assertBNEqual(config.depositLimit, 1_000_000_000);

    const tolerance = 0.000001;
    assertI80F48Approx(interest.optimalUtilizationRate, 0.5, tolerance);
    assertI80F48Approx(interest.plateauInterestRate, 0.6, tolerance);
    assertI80F48Approx(interest.maxInterestRate, 3, tolerance);
    assertI80F48Equal(interest.insuranceFeeFixedApr, 0);
    assertI80F48Equal(interest.insuranceIrFee, 0);
    assertI80F48Equal(interest.protocolFixedFeeApr, 0);
    assertI80F48Equal(interest.protocolIrFee, 0);

    assert.deepEqual(config.operationalState, { operational: {} });
    assert.deepEqual(config.oracleSetup, { pythLegacy: {} });
    assertBNEqual(config.borrowLimit, 1_000_000_000);
    // TODO modify RiskTier to pass alignment requirements...
    assert.deepEqual(config.riskTier, { collateral: {} });
    assertBNEqual(config.totalAssetValueInitLimit, 100_000_000_000);
    assert.equal(config.oracleMaxAge, 100);
  });

  it("(admin) Add bank (token A) - happy path", async () => {
    let config = defaultBankConfig(oracles.tokenAOracle.publicKey);
    let bankKey = bankKeypairA.publicKey;

    await groupAdmin.userMarginProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await addBank(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: ecosystem.tokenAMint.publicKey,
          bank: bankKey,
          config: config,
        })
      ),
      [bankKeypairA]
    );

    if (verbose) {
      console.log("*init token A bank " + bankKey);
    }
  });
});
