import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { LAMPORTS_PER_SOL, SystemProgram, Transaction } from "@solana/web3.js";
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
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { getEpochAndSlot, getStakeActivation } from "./utils/stake-utils";

describe("Borrow power grows as v0 Staked SOL gains value from appreciation", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  // User 2 has a validator 0 staked depost [0] position - net value = 1 LST token Users 0/1/2
  // deposited 10 SOL each, so a total of 30 is staked with validator 0 (minus the 1 SOL staked to
  // start the pool, which is non-refundable and doesn't function as collateral)
  /** SOL to add to the validator as pretend-earned epoch rewards */
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

  // Here we try to a troll exploit by sending SOL directly to the stake pool's sol balance.
  it("v0 stake sol pool grows by " + appreciation + " SOL", async () => {
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
  });

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

  // The stake hasn't changed (even though the SOL balance did) so this should still fail
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

  it("Generate stake income....", async () => {
    // TODO how?
    //
  });

  // Now the stake is worth enough and the user can borrow
  it("(user 2) borrows 1.1 SOL against their STAKED position - succeeds", async () => {
    const user = users[2];
    const userAccount = user.accounts.get(USER_ACCOUNT);
    const userLstAta = user.accounts.get(LST_ATA);
    let tx = new Transaction().add(
      // TODO if we find a way to make stake appreciate on localnet, remove...
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: validators[0].bank,
        tokenAccount: userLstAta,
        amount: new BN(1 * 10 ** ecosystem.wsolDecimals),
      }),
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
