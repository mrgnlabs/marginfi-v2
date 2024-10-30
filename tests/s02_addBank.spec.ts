import { BN, Program, workspace } from "@coral-xyz/anchor";
import { Keypair, Transaction } from "@solana/web3.js";
import {
  addBank,
  addBankPermissionless,
  groupInitialize,
  initStakedSettings,
} from "./utils/group-instructions";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairSol,
  bankKeypairUsdc,
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
  validators,
  verbose,
} from "./rootHooks";
import {
  assertBankrunTxFailed,
  assertBNEqual,
  assertI80F48Approx,
  assertI80F48Equal,
  assertKeysEqual,
} from "./utils/genericTests";
import {
  ASSET_TAG_DEFAULT,
  ASSET_TAG_SOL,
  ASSET_TAG_STAKED,
  defaultBankConfig,
  defaultStakedInterestSettings,
  I80F48_ONE,
  SINGLE_POOL_PROGRAM_ID,
} from "./utils/types";
import { assert } from "chai";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { deriveBankWithSeed, deriveStakedSettings } from "./utils/pdas";

describe("Init group and add banks with asset category flags", () => {
  const program = workspace.Marginfi as Program<Marginfi>;

  it("(admin) Init group - happy path", async () => {
    let tx = new Transaction();

    tx.add(
      await groupInitialize(program, {
        marginfiGroup: marginfiGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, marginfiGroup);
    await banksClient.processTransaction(tx);

    let group = await bankrunProgram.account.marginfiGroup.fetch(
      marginfiGroup.publicKey
    );
    assertKeysEqual(group.admin, groupAdmin.wallet.publicKey);
    if (verbose) {
      console.log("*init group: " + marginfiGroup.publicKey);
      console.log(" group admin: " + group.admin);
    }
  });

  // TODO add bank permissionless fails prior to opting in

  it("(admin) Init staked settings for group - opts in to use staked collateral", async () => {
    const settings = defaultStakedInterestSettings(
      oracles.wsolOracle.publicKey
    );
    let tx = new Transaction();

    tx.add(
      await initStakedSettings(groupAdmin.userMarginProgram, {
        group: marginfiGroup.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        settings: settings,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    const [settingsKey] = deriveStakedSettings(
      program.programId,
      marginfiGroup.publicKey
    );
    if (verbose) {
      console.log("*init staked settings: " + settingsKey);
    }

    let settingsAcc = await bankrunProgram.account.stakedSettings.fetch(
      settingsKey
    );
    assertKeysEqual(settingsAcc.key, settingsKey);
    assertKeysEqual(settingsAcc.oracle, oracles.wsolOracle.publicKey);
    assertI80F48Approx(settingsAcc.assetWeightInit, 0.8);
    assertI80F48Approx(settingsAcc.assetWeightMaint, 0.9);
    assertBNEqual(settingsAcc.depositLimit, 1_000_000_000_000);
    assertBNEqual(settingsAcc.totalAssetValueInitLimit, 150_000_000);
    assert.equal(settingsAcc.oracleMaxAge, 10);
    assert.deepEqual(settingsAcc.riskTier, { collateral: {} });
  });

  it("(admin) Add bank (USDC) - is neither SOL nor staked LST", async () => {
    let setConfig = defaultBankConfig(oracles.usdcOracle.publicKey);
    let bankKey = bankKeypairUsdc.publicKey;

    let tx = new Transaction();
    tx.add(
      await addBank(program, {
        marginfiGroup: marginfiGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.usdcMint.publicKey,
        bank: bankKey,
        config: setConfig,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, bankKeypairUsdc);
    await banksClient.processTransaction(tx);

    if (verbose) {
      console.log("*init USDC bank " + bankKey);
    }

    const bank = await bankrunProgram.account.bank.fetch(bankKey);
    assert.equal(bank.config.assetTag, ASSET_TAG_DEFAULT);
  });

  it("(admin) Add bank (SOL) - is tagged as SOL", async () => {
    let setConfig = defaultBankConfig(oracles.wsolOracle.publicKey);
    setConfig.assetTag = ASSET_TAG_SOL;
    let bankKey = bankKeypairSol.publicKey;

    let tx = new Transaction();
    tx.add(
      await addBank(program, {
        marginfiGroup: marginfiGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: ecosystem.wsolMint.publicKey,
        bank: bankKey,
        config: setConfig,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, bankKeypairSol);
    await banksClient.processTransaction(tx);

    if (verbose) {
      console.log("*init SOL bank " + bankKey);
    }

    const bank = await bankrunProgram.account.bank.fetch(bankKey);
    assert.equal(bank.config.assetTag, ASSET_TAG_SOL);
  });

  it("(admin) Tries to add staked bank WITH permission - should fail", async () => {
    let setConfig = defaultBankConfig(oracles.wsolOracle.publicKey);
    setConfig.assetTag = ASSET_TAG_STAKED;
    setConfig.borrowLimit = new BN(0);
    let bankKeypair = Keypair.generate();

    let tx = new Transaction();
    tx.add(
      await addBank(groupAdmin.userMarginProgram, {
        marginfiGroup: marginfiGroup.publicKey,
        admin: groupAdmin.wallet.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        bankMint: validators[0].splMint,
        bank: bankKeypair.publicKey,
        config: setConfig,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet, bankKeypair);
    let result = await banksClient.tryProcessTransaction(tx);
    assertBankrunTxFailed(result, "0x17a2");
  });

  it("(admin) Add bank (validator 0) permissionless - is tagged as Staked", async () => {
    const [bankKey] = deriveBankWithSeed(
      program.programId,
      marginfiGroup.publicKey,
      validators[0].splMint,
      new BN(0)
    );
    validators[0].bank = bankKey;

    let tx = new Transaction();
    tx.add(
      await addBankPermissionless(program, {
        marginfiGroup: marginfiGroup.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
        pythOracle: oracles.wsolOracle.publicKey,
        stakePool: validators[0].splPool,
        seed: new BN(0),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);

    if (verbose) {
      console.log("*init LST bank " + validators[0].bank + " (validator 0)");
    }
    const bank = await bankrunProgram.account.bank.fetch(validators[0].bank);
    assert.equal(bank.config.assetTag, ASSET_TAG_STAKED);
    // Note: This field is set for all banks, but only relevant for ASSET_TAG_STAKED banks.
    assertI80F48Equal(bank.solAppreciationRate, I80F48_ONE);
    // TODO assert other relevant default values...
  });
});
