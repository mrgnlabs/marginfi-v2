import { workspace, Program } from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import BN from "bn.js";
import { Marginfi } from "../target/types/marginfi";
import {
  marginfiGroup,
  validators,
  groupAdmin,
  oracles,
  bankrunContext,
  banksClient,
  bankrunProgram,
} from "./rootHooks";
import {
  editStakedSettings,
  propagateStakedSettings,
} from "./utils/group-instructions";
import { deriveBankWithSeed, deriveStakedSettings } from "./utils/pdas";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { assert } from "chai";
import {
  assertKeysEqual,
  assertI80F48Approx,
  assertBNEqual,
  assertBankrunTxFailed,
} from "./utils/genericTests";
import {
  defaultStakedInterestSettings,
  StakedSettingsEdit,
} from "./utils/types";

describe("Edit and propagate staked settings", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  let settingsKey: PublicKey;
  let bankKey: PublicKey;

  before(async () => {
    [settingsKey] = deriveStakedSettings(
      program.programId,
      marginfiGroup.publicKey
    );
    [bankKey] = deriveBankWithSeed(
      program.programId,
      marginfiGroup.publicKey,
      validators[0].splMint,
      new BN(0)
    );
  });

  it("(admin) edits some settings - happy path", async () => {
    const settings: StakedSettingsEdit = {
      oracle: oracles.usdcOracle.publicKey,
      assetWeightInit: bigNumberToWrappedI80F48(0.2),
      assetWeightMaint: bigNumberToWrappedI80F48(0.3),
      depositLimit: new BN(42),
      totalAssetValueInitLimit: new BN(43),
      oracleMaxAge: 44,
      riskTier: {
        collateral: undefined,
      },
    };
    let tx = new Transaction().add(
      await editStakedSettings(groupAdmin.userMarginProgram, {
        settingsKey: settingsKey,
        settings: settings,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    let settingsAcc = await bankrunProgram.account.stakedSettings.fetch(
      settingsKey
    );
    assertKeysEqual(settingsAcc.key, settingsKey);
    assertKeysEqual(settingsAcc.oracle, oracles.usdcOracle.publicKey);
    assertI80F48Approx(settingsAcc.assetWeightInit, 0.2);
    assertI80F48Approx(settingsAcc.assetWeightMaint, 0.3);
    assertBNEqual(settingsAcc.depositLimit, 42);
    assertBNEqual(settingsAcc.totalAssetValueInitLimit, 43);
    assert.equal(settingsAcc.oracleMaxAge, 44);
    assert.deepEqual(settingsAcc.riskTier, { collateral: {} });
  });

  it("(permissionless) Propagate staked settings to a bank - happy path", async () => {
    let tx = new Transaction();
    tx.add(
      await propagateStakedSettings(program, {
        settings: settingsKey,
        bank: bankKey,
        oracle: oracles.usdcOracle.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet); // just to the pay the fee
    let result = await banksClient.tryProcessTransaction(tx);

    const bank = await bankrunProgram.account.bank.fetch(bankKey);
    const config = bank.config;
    assertKeysEqual(config.oracleKeys[0], oracles.usdcOracle.publicKey);
    assertI80F48Approx(config.assetWeightInit, 0.2);
    assertI80F48Approx(config.assetWeightMaint, 0.3);
    assertBNEqual(config.depositLimit, 42);
    assertBNEqual(config.totalAssetValueInitLimit, 43);
    assert.equal(config.oracleMaxAge, 44);
    assert.deepEqual(config.riskTier, { collateral: {} });
  });

  it("(admin) sets a bad oracle - fails at propagation", async () => {
    const settings: StakedSettingsEdit = {
      oracle: PublicKey.default,
      assetWeightInit: null,
      assetWeightMaint: null,
      depositLimit: null,
      totalAssetValueInitLimit: null,
      oracleMaxAge: null,
      riskTier: null,
    };
    let tx = new Transaction().add(
      await editStakedSettings(groupAdmin.userMarginProgram, {
        settingsKey: settingsKey,
        settings: settings,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    let settingsAcc = await bankrunProgram.account.stakedSettings.fetch(
      settingsKey
    );
    assertKeysEqual(settingsAcc.oracle, PublicKey.default);

    tx = new Transaction();
    tx.add(
      await propagateStakedSettings(program, {
        settings: settingsKey,
        bank: bankKey,
        oracle: PublicKey.default,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet); // just to the pay the fee
    let result = await banksClient.tryProcessTransaction(tx);

    // 6007 (InvalidOracleAccount)
    assertBankrunTxFailed(result, "0x1777");
  });

  it("(admin) restores default settings - happy path", async () => {
    const defaultSettings = defaultStakedInterestSettings(
      oracles.wsolOracle.publicKey
    );
    const settings: StakedSettingsEdit = {
      oracle: defaultSettings.oracle,
      assetWeightInit: defaultSettings.assetWeightInit,
      assetWeightMaint: defaultSettings.assetWeightMaint,
      depositLimit: defaultSettings.depositLimit,
      totalAssetValueInitLimit: defaultSettings.totalAssetValueInitLimit,
      oracleMaxAge: defaultSettings.oracleMaxAge,
      riskTier: defaultSettings.riskTier,
    };
    // Note you can pack propagates into the edit tx, so with a LUT you can easily propagate
    // hundreds of banks in the same ts as edit
    let tx = new Transaction().add(
      await editStakedSettings(groupAdmin.userMarginProgram, {
        settingsKey: settingsKey,
        settings: settings,
      }),
      await propagateStakedSettings(program, {
        settings: settingsKey,
        bank: bankKey,
        oracle: defaultSettings.oracle,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
  });
});
