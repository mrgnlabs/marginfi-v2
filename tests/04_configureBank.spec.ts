import { BN, Program, workspace } from "@coral-xyz/anchor";
import { configureBank, configureBankOracle } from "./utils/group-instructions";
import { Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import { bankKeypairUsdc, groupAdmin, oracles, users } from "./rootHooks";
import {
  assertBNEqual,
  assertI80F48Approx,
  assertI80F48Equal,
  assertKeysEqual,
  expectFailedTxWithError,
  expectFailedTxWithMessage,
} from "./utils/genericTests";
import { assert } from "chai";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import {
  aprToU32,
  ASSET_TAG_SOL,
  BankConfigOptRaw,
  CLOSE_ENABLED_FLAG,
  defaultBankConfigOptRaw,
  FREEZE_SETTINGS,
  InterestRateConfig1_6,
  InterestRateConfigOpt1_6,
  makeRatePoints,
} from "./utils/types";

describe("Lending pool configure bank", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  it("(admin) Configure bank (USDC) - happy path", async () => {
    const bankKey = bankKeypairUsdc.publicKey;
    const expPoints = makeRatePoints([0.3, 0.4, 0.5], [1, 2, 3]);
    let interestRateConfig: InterestRateConfigOpt1_6 = {
      insuranceFeeFixedApr: bigNumberToWrappedI80F48(0.3),
      insuranceIrFee: bigNumberToWrappedI80F48(0.4),
      protocolFixedFeeApr: bigNumberToWrappedI80F48(0.5),
      protocolIrFee: bigNumberToWrappedI80F48(0.6),
      protocolOriginationFee: bigNumberToWrappedI80F48(0.7),
      zeroUtilRate: aprToU32(0.1),
      hundredUtilRate: aprToU32(4),
      points: expPoints,
    };

    let bankConfigOpt: BankConfigOptRaw = {
      assetWeightInit: bigNumberToWrappedI80F48(0.6),
      assetWeightMaint: bigNumberToWrappedI80F48(0.7),
      liabilityWeightInit: bigNumberToWrappedI80F48(1.9),
      liabilityWeightMaint: bigNumberToWrappedI80F48(1.8),
      depositLimit: new BN(5000),
      borrowLimit: new BN(10000),
      riskTier: null,
      assetTag: ASSET_TAG_SOL,
      totalAssetValueInitLimit: new BN(15000),
      interestRateConfig: interestRateConfig,
      operationalState: {
        paused: undefined,
      },
      oracleMaxAge: 150,
      permissionlessBadDebtSettlement: null,
      freezeSettings: null,
      oracleMaxConfidence: 420000,
    };

    await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await configureBank(groupAdmin.mrgnProgram, {
          bank: bankKey,
          bankConfigOpt: bankConfigOpt,
        })
      )
    );

    const bank = await program.account.bank.fetch(bankKey);
    const config = bank.config;
    const interest = config.interestRateConfig;

    assertI80F48Approx(config.assetWeightInit, 0.6);
    assertI80F48Approx(config.assetWeightMaint, 0.7);
    assertI80F48Approx(config.liabilityWeightInit, 1.9);
    assertI80F48Approx(config.liabilityWeightMaint, 1.8);
    assertBNEqual(config.depositLimit, 5000);

    // Note: Zero since 1.6 replaced the legacy curve system
    assertI80F48Equal(interest.optimalUtilizationRate, 0);
    assertI80F48Equal(interest.plateauInterestRate, 0);
    assertI80F48Equal(interest.maxInterestRate, 0);

    assertI80F48Approx(interest.insuranceFeeFixedApr, 0.3);
    assertI80F48Approx(interest.insuranceIrFee, 0.4);
    assertI80F48Approx(interest.protocolFixedFeeApr, 0.5);
    assertI80F48Approx(interest.protocolIrFee, 0.6);
    assertI80F48Approx(interest.protocolOriginationFee, 0.7);

    assert.approximately(interest.zeroUtilRate, aprToU32(0.1), 2);
    assert.approximately(interest.hundredUtilRate, aprToU32(4), 2);
    for (let i = 0; i < 3; i++) {
      assert.approximately(interest.points[i].util, expPoints[i].util, 2);
      assert.approximately(interest.points[i].rate, expPoints[i].rate, 2);
    }
    // Rest is padding
    for (let i = 3; i < 5; i++) {
      const p = interest.points[i];
      if (interest.points[i].util === 0 && interest.points[i].rate === 0) {
        assert.equal(p.util, 0);
        assert.equal(p.rate, 0);
      } else {
        assert.ok(false, "expected padding");
      }
    }

    assert.deepEqual(config.operationalState, { paused: {} });
    assert.deepEqual(config.oracleSetup, { pythPushOracle: {} }); // no change
    assertBNEqual(config.borrowLimit, 10000);
    assert.deepEqual(config.riskTier, { collateral: {} }); // no change
    assert.equal(config.assetTag, ASSET_TAG_SOL);
    assertBNEqual(config.totalAssetValueInitLimit, 15000);
    assert.equal(config.oracleMaxAge, 150);
    assert.equal(config.oracleMaxConfidence, 420000);
  });

  it("(admin) Restore default settings to bank (USDC)", async () => {
    await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await configureBank(groupAdmin.mrgnProgram, {
          bank: bankKeypairUsdc.publicKey,
          bankConfigOpt: defaultBankConfigOptRaw(),
        })
      )
    );
  });

  it("(admin) update oracle (USDC)", async () => {
    const bankKey = bankKeypairUsdc.publicKey;
    await groupAdmin.mrgnProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await configureBankOracle(groupAdmin.mrgnProgram, {
          bank: bankKey,
          type: 3, // pyth pull
          oracle: oracles.tokenAOracle.publicKey,
        })
      )
    );
    const bank = await program.account.bank.fetch(bankKey);
    const config = bank.config;
    assert.deepEqual(config.oracleSetup, { pythPushOracle: {} }); // no change
    assertKeysEqual(config.oracleKeys[0], oracles.tokenAOracle.publicKey);
  });

  it("(admin) restore to valid oracle (USDC)", async () => {
    const bankKey = bankKeypairUsdc.publicKey;
    await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await configureBankOracle(groupAdmin.mrgnProgram, {
          bank: bankKey,
          type: 3, // pyth pull
          oracle: oracles.usdcOracle.publicKey,
        })
      )
    );
    const bank = await program.account.bank.fetch(bankKey);
    const config = bank.config;
    assert.deepEqual(config.oracleSetup, { pythPushOracle: {} }); // no change
    assertKeysEqual(config.oracleKeys[0], oracles.usdcOracle.publicKey);
  });

  it("(admin) update oracle to invalid state - should fail", async () => {
    const bankKey = bankKeypairUsdc.publicKey;
    await expectFailedTxWithError(
      async () => {
        await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
          new Transaction().add(
            await configureBankOracle(groupAdmin.mrgnProgram, {
              bank: bankKey,
              type: 3,
              oracle: oracles.tokenAOracleFeed.publicKey, // sneaky sneaky
            })
          )
        );
      },
      "PythPushInvalidAccount",
      6060
    );

    await expectFailedTxWithMessage(async () => {
      await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
        new Transaction().add(
          await configureBankOracle(groupAdmin.mrgnProgram, {
            bank: bankKey,
            type: 0,
            oracle: oracles.tokenAOracle.publicKey,
          })
        )
      );
    }, "OracleNotSetup");

    await expectFailedTxWithMessage(async () => {
      await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
        new Transaction().add(
          await configureBankOracle(groupAdmin.mrgnProgram, {
            bank: bankKey,
            type: 2,
            oracle: oracles.tokenAOracle.publicKey,
          })
        )
      );
    }, "swb v2 is deprecated");

    await expectFailedTxWithMessage(async () => {
      await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
        new Transaction().add(
          await configureBankOracle(groupAdmin.mrgnProgram, {
            bank: bankKey,
            type: 42,
            oracle: oracles.tokenAOracle.publicKey,
          })
        )
      );
    }, "unsupported oracle type");
  });

  it("(attacker) tries to change oracle  - should fail with generic signature failure", async () => {
    const bankKey = bankKeypairUsdc.publicKey;

    await expectFailedTxWithError(
      async () => {
        await users[0].mrgnProgram.provider.sendAndConfirm!(
          new Transaction().add(
            await configureBankOracle(users[0].mrgnProgram, {
              bank: bankKey,
              type: 3,
              oracle: oracles.wsolOracle.publicKey,
            })
          )
        );
      },
      "ConstraintHasOne",
      2001
    );

    await expectFailedTxWithMessage(async () => {
      await users[0].mrgnProgram!.provider.sendAndConfirm!(
        new Transaction().add(
          await configureBankOracle(groupAdmin.mrgnProgram, {
            bank: bankKey,
            type: 3,
            oracle: oracles.wsolOracle.publicKey,
          })
        )
      );
    }, "Missing signature for");
  });

  it("(admin) Freeze USDC settings so they cannot be changed again (USDC)", async () => {
    let config = defaultBankConfigOptRaw();
    config.freezeSettings = true;
    await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await configureBank(groupAdmin.mrgnProgram, {
          bank: bankKeypairUsdc.publicKey,
          bankConfigOpt: config,
        })
      )
    );
    const bank = await program.account.bank.fetch(bankKeypairUsdc.publicKey);
    // Note: The CLOSE_ENABLED_FLAG is never unset
    assertBNEqual(bank.flags, FREEZE_SETTINGS + CLOSE_ENABLED_FLAG);
  });

  it("(admin) attempt to update oracle after freeze - fails with generic panic", async () => {
    const bankKey = bankKeypairUsdc.publicKey;

    await expectFailedTxWithMessage(async () => {
      await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
        new Transaction().add(
          await configureBankOracle(groupAdmin.mrgnProgram, {
            bank: bankKey,
            type: 3,
            oracle: oracles.wsolOracle.publicKey,
          })
        )
      );
    }, "change oracle settings on frozen banks");
  });

  it("(admin) Update settings after a freeze - only deposit/borrow caps update", async () => {
    let configNew = defaultBankConfigOptRaw();
    const newDepositLimit = new BN(2_000_000_000);
    const newBorrowLimit = new BN(3_000_000_000);
    configNew.depositLimit = newDepositLimit;
    configNew.borrowLimit = newBorrowLimit;

    // These will be ignored...
    configNew.oracleMaxAge = 42;
    configNew.freezeSettings = false;

    await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await configureBank(groupAdmin.mrgnProgram, {
          bank: bankKeypairUsdc.publicKey,
          bankConfigOpt: configNew,
        })
      )
    );
    const bank = await program.account.bank.fetch(bankKeypairUsdc.publicKey);
    const config = bank.config;
    assertBNEqual(config.depositLimit, newDepositLimit);
    assertBNEqual(config.borrowLimit, newBorrowLimit);

    // Ignored fields didn't change..
    assert.equal(config.oracleMaxAge, 240);
    assertBNEqual(bank.flags, FREEZE_SETTINGS + CLOSE_ENABLED_FLAG); // still frozen
  });
});
