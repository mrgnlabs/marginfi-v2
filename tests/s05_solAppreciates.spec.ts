import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import {
  LAMPORTS_PER_SOL,
  STAKE_CONFIG_ID,
  StakeProgram,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_STAKE_HISTORY_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairSol,
  bankrunContext,
  bankrunProgram,
  bankRunProvider,
  banksClient,
  ecosystem,
  marginfiGroup,
  oracles,
  users,
  validators,
  verbose,
} from "./rootHooks";
import { assertBankrunTxFailed, assertKeysEqual } from "./utils/genericTests";
import { assert } from "chai";
import { borrowIx, depositIx } from "./utils/user-instructions";
import { LST_ATA, USER_ACCOUNT } from "./utils/mocks";
import {
  createPoolOnramp,
  getBankrunBlockhash,
  replenishPool,
} from "./utils/spl-staking-utils";
import {
  getEpochAndSlot,
  getStakeAccount,
  getStakeActivation,
} from "./utils/stake-utils";
import { deriveOnRampPool } from "./utils/pdas";
import { SINGLE_POOL_PROGRAM_ID } from "./utils/types";
import { dumpBankrunLogs } from "./utils/tools";
import { getMinimumBalanceForRentExemptAccount } from "@solana/spl-token";

describe("Borrow power grows as v0 Staked SOL gains value from appreciation", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  // User 2 has a validator 0 staked depost [0] position - net value = 1 LST token Users 0/1/2
  // deposited 10 SOL each, so a total of 30 is staked with validator 0 (minus the 1 SOL staked to
  // start the pool, which is non-refundable and doesn't function as collateral)

  /** SOL to add to the validator as pretend-earned mev rewards */
  const appreciation = 30;

  it("(user 2) tries to borrow 1.1 SOL against 1 v0 STAKED - fails, not enough funds", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);

    let tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: bankKeypairSol.publicKey,
        tokenAccount: user.wsolAccount,
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
        amount: new BN(1.1 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(tx);

    // 6010 (Generic risk engine rejection)
    assertBankrunTxFailed(result, "0x177a");

    const userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[1].active, 0);
  });

  // Note: there is also some natural appreciation here because a few epochs have elapsed...

  // In older versions of SVSP, MEV rewards like this would be orphaned here forever and would not
  // count as stake for pricing purposes.
  it(
    "v0 stake sol pool grows by " + appreciation + " SOL (e.g. MEV rewards)",
    async () => {
      let tx = new Transaction();
      tx.add(
        SystemProgram.transfer({
          fromPubkey: wallet.publicKey,
          toPubkey: validators[0].splSolPool,
          lamports: appreciation * LAMPORTS_PER_SOL,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(wallet.payer);
      await banksClient.processTransaction(tx);
    }
  );

  it("(user 2 - attacker) ties to sneak in bad lst mint - should fail", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    let tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: bankKeypairSol.publicKey,
        tokenAccount: user.wsolAccount,
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          validators[1].splMint, // Bad mint
          validators[0].splSolPool,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
        amount: new BN(0.1 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(tx);

    // Throws 6007 (InvalidOracleAccount) first at `try_from_bank_config_with_max_age` which is
    // converted to 6010 (Generic risk engine rejection) downstream
    assertBankrunTxFailed(result, "0x177a");
  });

  it("(user 2 - attacker) ties to sneak in bad sol pool - should fail", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    let tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: bankKeypairSol.publicKey,
        tokenAccount: user.wsolAccount,
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[1].splSolPool, // Bad pool
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
        amount: new BN(0.2 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(tx);

    // Throws 6007 (InvalidOracleAccount) first at `try_from_bank_config_with_max_age` which is
    // converted to 6010 (Generic risk engine rejection) downstream
    assertBankrunTxFailed(result, "0x177a");
  });

  // The stake hasn't changed (even though the SOL balance did) so this should still fail. For this
  // to count, we must realize the MEV rewards first (see the next test)
  it("(user 2) borrows 1.1 SOL against their STAKED position - fails", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    let tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: bankKeypairSol.publicKey,
        tokenAccount: user.wsolAccount,
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
        // Note: We use a different (slightly higher) amount, so Bankrun treats this as a different
        // tx. Using the exact same values as above can cause the test to fail on faster machines
        // because the same tx was already sent for this blockhash (i.e. "this transaction has
        // already been processed")
        amount: new BN(1.112 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    let result = await banksClient.tryProcessTransaction(tx);

    // 6010 (Generic risk engine rejection)
    assertBankrunTxFailed(result, "0x177a");
  });

  it(
    "REMOVE v0 stake sol pool grows by " +
      appreciation +
      " SOL (e.g. MEV rewards)",
    async () => {
      let tx = new Transaction();
      tx.add(
        SystemProgram.transfer({
          fromPubkey: wallet.publicKey,
          toPubkey: validators[0].splPool,
          lamports: appreciation * LAMPORTS_PER_SOL,
        })
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(wallet.payer);
      await banksClient.processTransaction(tx);
    }
  );

  it("Realize income from MEV rewards", async () => {
    // First, create the on-ramp account that will temporarily stake MEV rewards
    const [onRampPoolKey] = deriveOnRampPool(validators[0].splPool);
    const rent =
      await bankRunProvider.connection.getMinimumBalanceForRentExemption(
        StakeProgram.space
      );
    const rentIx = SystemProgram.transfer({
      fromPubkey: wallet.payer.publicKey,
      toPubkey: onRampPoolKey,
      lamports: rent,
    });
    const ix = createPoolOnramp(validators[0].voteAccount);
    let initOnRampTx = new Transaction().add(rentIx, ix);
    initOnRampTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    initOnRampTx.sign(wallet.payer); // pays the tx fee and rent
    await banksClient.processTransaction(initOnRampTx);

    const onRampAccBefore = await bankRunProvider.connection.getAccountInfo(
      onRampPoolKey
    );
    const onRampBefore = getStakeAccount(onRampAccBefore.data);
    const stakeBefore = onRampBefore.stake.delegation.stake.toString();
    if (verbose) {
      console.log("On ramp lamps: " + onRampAccBefore.lamports);
      console.log(" (rent was:    " + rent + ")");
      console.log("On ramp stake: " + stakeBefore);
    }

    let { epoch: epochBeforeWarp, slot: slotBeforeWarp } =
      await getEpochAndSlot(banksClient);
    bankrunContext.warpToEpoch(BigInt(epochBeforeWarp + 1));
    let { epoch: epochAfterWarp, slot: slotAfterWarp } = await getEpochAndSlot(
      banksClient
    );
    for (let i = 0; i < 3; i++) {
      bankrunContext.warpToSlot(BigInt(i + slotAfterWarp + 1));
      const dummyTx = new Transaction();
      dummyTx.add(
        SystemProgram.transfer({
          fromPubkey: users[0].wallet.publicKey,
          toPubkey: bankrunProgram.provider.publicKey,
          lamports: i,
        })
      );
      dummyTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      dummyTx.sign(users[0].wallet);
      await banksClient.processTransaction(dummyTx);
    }

    let { epoch, slot } = await getEpochAndSlot(banksClient);
    if (verbose) {
      console.log("It is now epoch: " + epoch + " slot " + slot);
    }

    // Next, the replenish crank cycles free SOL into the "on ramp" pool
    let replenishTx = new Transaction().add(
      replenishPool(validators[0].voteAccount)
    );
    replenishTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    replenishTx.sign(wallet.payer); // pays the tx fee and rent
    let result = await banksClient.tryProcessTransaction(replenishTx);
    dumpBankrunLogs(result);

    const onRampAccAfter = await bankRunProvider.connection.getAccountInfo(
      onRampPoolKey
    );
    const onRampAfter = getStakeAccount(onRampAccAfter.data);
    const stakeAfter = onRampAfter.stake.delegation.stake.toString();
    if (verbose) {
      console.log("On ramp lamps: " + onRampAccAfter.lamports);
      console.log(" (rent was:    " + rent + ")");
      console.log("On ramp stake: " + stakeAfter);
    }
  });

  // Now the stake is worth enough and the user can borrow
  it("(user 2) borrows 1.1 SOL against their STAKED position - succeeds", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    const userLstAta = user.accounts.get(LST_ATA);
    let tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: bankKeypairSol.publicKey,
        tokenAccount: user.wsolAccount,
        remaining: [
          validators[0].bank,
          oracles.wsolOracle.publicKey,
          validators[0].splMint,
          validators[0].splSolPool,
          bankKeypairSol.publicKey,
          oracles.wsolOracle.publicKey,
        ],
        amount: new BN(1.113 * 10 ** ecosystem.wsolDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    const userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[1].active, 1);
    assertKeysEqual(balances[1].bankPk, bankKeypairSol.publicKey);
  });
});
