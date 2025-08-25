import { Program, workspace } from "@coral-xyz/anchor";
import { Keypair, Transaction, PublicKey, SYSVAR_INSTRUCTIONS_PUBKEY, SystemProgram } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import { Mocks } from "../target/types/mocks";
import { marginfiGroup, users } from "./rootHooks";
import {
  assertKeysEqual,
  assertKeyDefault,
} from "./utils/genericTests";
import { assert } from "chai";
import { deriveMarginfiAccountPda } from "./utils/pdas";

describe("Initialize user account with PDA via CPI", () => {
  const marginfiProgram = workspace.Marginfi as Program<Marginfi>;
  const mocksProgram = workspace.Mocks as Program<Mocks>;


  it("(user 0) Initialize PDA user account via CPI - happy path", async () => {
    const accountIndex = 50; // Use high index to avoid conflicts with other tests
    const [accountPda, bump] = deriveMarginfiAccountPda(
      marginfiProgram.programId,
      marginfiGroup.publicKey,
      users[0].wallet.publicKey,
      accountIndex
    );

    const callLogKeypair = Keypair.generate();

    let tx: Transaction = new Transaction();
    tx.add(
      await mocksProgram.methods
        .createMarginfiAccountPdaViaCpi(accountIndex, null)
        .accounts({
          marginfiGroup: marginfiGroup.publicKey,
          marginfiAccount: accountPda,
          authority: users[0].wallet.publicKey,
          feePayer: users[0].wallet.publicKey,
          instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
          marginfiProgram: marginfiProgram.programId,
          callLog: callLogKeypair.publicKey,
        })
        .instruction()
    );

    await users[0].mrgnProgram.provider.sendAndConfirm(tx, [callLogKeypair]);

    // Verify the marginfi account was created correctly
    const userAccount = await marginfiProgram.account.marginfiAccount.fetch(accountPda);
    assertKeysEqual(userAccount.group, marginfiGroup.publicKey);
    assertKeysEqual(userAccount.authority, users[0].wallet.publicKey);
    assertKeyDefault(userAccount.migratedFrom);
    assertKeyDefault(userAccount.migratedTo);

    // Verify the call log was created
    const callLog = await mocksProgram.account.cpiCallLog.fetch(callLogKeypair.publicKey);
    assertKeysEqual(callLog.callerProgram, mocksProgram.programId);
    assertKeysEqual(callLog.targetProgram, marginfiProgram.programId);
    assertKeysEqual(callLog.marginfiGroup, marginfiGroup.publicKey);
    assertKeysEqual(callLog.marginfiAccount, accountPda);
    assertKeysEqual(callLog.authority, users[0].wallet.publicKey);
    assert.equal(callLog.accountIndex, accountIndex);
    assert.equal(callLog.thirdPartyId, null);

    users[0].accounts.set("USER_ACCOUNT_PDA_CPI", accountPda);
    66
  });

  it("(user 0) Initialize PDA account via CPI with restricted third-party ID 42 - should succeed", async () => {
    const accountIndex = 51; // Use different index
    const restrictedThirdPartyId = 42;
    const [accountPda, bump] = deriveMarginfiAccountPda(
      marginfiProgram.programId,
      marginfiGroup.publicKey,
      users[0].wallet.publicKey,
      accountIndex,
      restrictedThirdPartyId
    );

    const callLogKeypair = Keypair.generate();

    let tx: Transaction = new Transaction();
    tx.add(
      await mocksProgram.methods
        .createMarginfiAccountPdaViaCpi(accountIndex, restrictedThirdPartyId)
        .accounts({
          marginfiGroup: marginfiGroup.publicKey,
          marginfiAccount: accountPda,
          authority: users[0].wallet.publicKey,
          feePayer: users[0].wallet.publicKey,
          instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
          marginfiProgram: marginfiProgram.programId,
          callLog: callLogKeypair.publicKey,
        })
        .instruction()
    );

    // This should succeed because it's called via CPI from the authorized mocks program
    await users[0].mrgnProgram.provider.sendAndConfirm(tx, [callLogKeypair]);

    // Verify the marginfi account was created
    const userAccount = await marginfiProgram.account.marginfiAccount.fetch(accountPda);
    assertKeysEqual(userAccount.group, marginfiGroup.publicKey);
    assertKeysEqual(userAccount.authority, users[0].wallet.publicKey);

    // Verify the call log includes the third-party ID
    const callLog = await mocksProgram.account.cpiCallLog.fetch(callLogKeypair.publicKey);
    assert.equal(callLog.thirdPartyId, restrictedThirdPartyId);
  });
});