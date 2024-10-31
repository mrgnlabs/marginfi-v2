import { BN, Program, workspace } from "@coral-xyz/anchor";
import { AccountMeta, Keypair, PublicKey, Transaction } from "@solana/web3.js";
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
  users,
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
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

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

  it("(attacker) Add bank (validator 0) with bad accounts + metadata - should fail", async () => {
    const [settingsKey] = deriveStakedSettings(
      program.programId,
      marginfiGroup.publicKey
    );
    const goodStakePool = validators[0].splPool;
    const goodLstMint = validators[0].splMint;
    const goodSolPool = validators[0].splSolPool;

    // Attacker tries to sneak in the wrong validator's information
    const badStakePool = validators[1].splPool;
    const badLstMint = validators[1].splMint;
    const badSolPool = validators[1].splSolPool;

    const stakePools = [goodStakePool, badStakePool];
    const lstMints = [goodLstMint, badLstMint];
    const solPools = [goodSolPool, badSolPool];

    for (const stakePool of stakePools) {
      for (const lstMint of lstMints) {
        for (const solPool of solPools) {
          // Skip the "all good" combination
          if (
            stakePool.equals(goodStakePool) &&
            lstMint.equals(goodLstMint) &&
            solPool.equals(goodSolPool)
          ) {
            continue;
          }

          // Skip the "all bad" combination (equivalent to a valid init of validator 1)
          if (
            stakePool.equals(badStakePool) &&
            lstMint.equals(badLstMint) &&
            solPool.equals(badSolPool)
          ) {
            continue;
          }

          const oracleMeta: AccountMeta = {
            pubkey: oracles.wsolOracle.publicKey,
            isSigner: false,
            isWritable: false,
          };
          const lstMeta: AccountMeta = {
            pubkey: lstMint,
            isSigner: false,
            isWritable: false,
          };
          const solPoolMeta: AccountMeta = {
            pubkey: solPool,
            isSigner: false,
            isWritable: false,
          };

          const ix = await program.methods
            .lendingPoolAddBankPermissionless(new BN(0))
            .accounts({
              stakedSettings: settingsKey,
              feePayer: users[0].wallet.publicKey,
              bankMint: lstMint,
              solPool: solPool,
              stakePool: stakePool,
              tokenProgram: TOKEN_PROGRAM_ID,
            })
            .remainingAccounts([oracleMeta, lstMeta, solPoolMeta])
            .instruction();

          let tx = new Transaction();
          tx.add(ix);
          tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
          tx.sign(users[0].wallet);

          let result = await banksClient.tryProcessTransaction(tx);
          assertBankrunTxFailed(result, "0x17a0");
        }
      }
    }
  });

  it("(attacker) Add bank (validator 0) with good accounts but bad metadata - should fail", async () => {
    const [settingsKey] = deriveStakedSettings(
      program.programId,
      marginfiGroup.publicKey
    );

    const goodStakePool = validators[0].splPool;
    const goodLstMint = validators[0].splMint;
    const goodSolPool = validators[0].splSolPool;

    // Note: StakePool is N/A because we do not pass StakePool in meta.
    // const badStakePool = validators[1].splPool;
    const badLstMint = validators[1].splMint;
    const badSolPool = validators[1].splSolPool;

    // Test all combinations of bad metadata keys
    const lstMints = [goodLstMint, badLstMint];
    const solPools = [goodSolPool, badSolPool];

    for (const lstMint of lstMints) {
      for (const solPool of solPools) {
        // Skip the all-good metadata case
        if (lstMint.equals(goodLstMint) && solPool.equals(goodSolPool)) {
          continue;
        }

        const oracleMeta: AccountMeta = {
          pubkey: oracles.wsolOracle.publicKey,
          isSigner: false,
          isWritable: false,
        };
        const lstMeta: AccountMeta = {
          pubkey: lstMint,
          isSigner: false,
          isWritable: false,
        };
        const solPoolMeta: AccountMeta = {
          pubkey: solPool,
          isSigner: false,
          isWritable: false,
        };

        const ix = await program.methods
          .lendingPoolAddBankPermissionless(new BN(0))
          .accounts({
            stakedSettings: settingsKey,
            feePayer: users[0].wallet.publicKey,
            bankMint: goodLstMint, // Good key
            solPool: goodSolPool, // Good key
            stakePool: goodStakePool, // Good key
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .remainingAccounts([oracleMeta, lstMeta, solPoolMeta]) // Bad metadata keys
          .instruction();

        let tx = new Transaction();
        tx.add(ix);
        tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
        tx.sign(users[0].wallet);

        let result = await banksClient.tryProcessTransaction(tx);
        assertBankrunTxFailed(result, "0x17a0");
      }
    }

    // Bad oracle meta
    const oracleMeta: AccountMeta = {
      pubkey: oracles.usdcOracle.publicKey, // Bad meta
      isSigner: false,
      isWritable: false,
    };
    const lstMeta: AccountMeta = {
      pubkey: goodLstMint,
      isSigner: false,
      isWritable: false,
    };
    const solPoolMeta: AccountMeta = {
      pubkey: goodSolPool,
      isSigner: false,
      isWritable: false,
    };

    const ix = await program.methods
      .lendingPoolAddBankPermissionless(new BN(0))
      .accounts({
        stakedSettings: settingsKey,
        feePayer: users[0].wallet.publicKey,
        bankMint: goodLstMint, // Good key
        solPool: goodSolPool, // Good key
        stakePool: goodStakePool, // Good key
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .remainingAccounts([oracleMeta, lstMeta, solPoolMeta]) // Bad oracle meta
      .instruction();

    let tx = new Transaction();
    tx.add(ix);
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(users[0].wallet);

    let result = await banksClient.tryProcessTransaction(tx);
    // Note: different error
    assertBankrunTxFailed(result, "0x1777");
  });

  it("(permissionless) Add bank (validator 0) - is tagged as Staked", async () => {
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
