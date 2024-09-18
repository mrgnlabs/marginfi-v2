import { BN, Program, workspace } from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { configureBank } from "./utils/instructions";
import { Marginfi } from "../target/types/marginfi";
import { bankKeypairUsdc, groupAdmin, marginfiGroup } from "./rootHooks";
import { assertBNEqual, assertI80F48Approx } from "./utils/genericTests";
import { assert } from "chai";
import {
  BankConfigOptRaw,
  InterestRateConfigRaw,
} from "@mrgnlabs/marginfi-client-v2";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { defaultBankConfigOptRaw } from "./utils/types";

describe("Lending pool configure bank", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  it("(admin) Configure bank (USDC) - happy path", async () => {
    const bankKey = bankKeypairUsdc.publicKey;
    let interestRateConfig: InterestRateConfigRaw = {
      optimalUtilizationRate: bigNumberToWrappedI80F48(0.1),
      plateauInterestRate: bigNumberToWrappedI80F48(0.2),
      maxInterestRate: bigNumberToWrappedI80F48(4),
      insuranceFeeFixedApr: bigNumberToWrappedI80F48(0.3),
      insuranceIrFee: bigNumberToWrappedI80F48(0.4),
      protocolFixedFeeApr: bigNumberToWrappedI80F48(0.5),
      protocolIrFee: bigNumberToWrappedI80F48(0.6),
    };

    let bankConfigOpt: BankConfigOptRaw = {
      assetWeightInit: bigNumberToWrappedI80F48(0.6),
      assetWeightMaint: bigNumberToWrappedI80F48(0.7),
      liabilityWeightInit: bigNumberToWrappedI80F48(1.9),
      liabilityWeightMaint: bigNumberToWrappedI80F48(1.8),
      depositLimit: new BN(5000),
      borrowLimit: new BN(10000),
      riskTier: null,
      totalAssetValueInitLimit: new BN(15000),
      interestRateConfig: interestRateConfig,
      operationalState: {
        paused: undefined,
      },
      oracle: null,
      oracleMaxAge: 50,
      permissionlessBadDebtSettlement: null,
    };

    await groupAdmin.userMarginProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await configureBank(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
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

    assertI80F48Approx(interest.optimalUtilizationRate, 0.1);
    assertI80F48Approx(interest.plateauInterestRate, 0.2);
    assertI80F48Approx(interest.maxInterestRate, 4);
    assertI80F48Approx(interest.insuranceFeeFixedApr, 0.3);
    assertI80F48Approx(interest.insuranceIrFee, 0.4);
    assertI80F48Approx(interest.protocolFixedFeeApr, 0.5);
    assertI80F48Approx(interest.protocolIrFee, 0.6);

    assert.deepEqual(config.operationalState, { paused: {} });
    assert.deepEqual(config.oracleSetup, { pythLegacy: {} }); // no change
    assertBNEqual(config.borrowLimit, 10000);
    assert.deepEqual(config.riskTier, { collateral: {} }); // no change
    assertBNEqual(config.totalAssetValueInitLimit, 15000);
    assert.equal(config.oracleMaxAge, 50);
  });

  it("(admin) Restore default settings to bank (USDC)", async () => {
    await groupAdmin.userMarginProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await configureBank(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
          bank: bankKeypairUsdc.publicKey,
          bankConfigOpt: defaultBankConfigOptRaw(),
        })
      )
    );
  });
});
