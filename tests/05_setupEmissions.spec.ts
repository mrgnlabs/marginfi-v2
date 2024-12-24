import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { setupEmissions, updateEmissions } from "./utils/group-instructions";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairUsdc,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  verbose,
} from "./rootHooks";
import {
  assertBNEqual,
  assertI80F48Approx,
  assertKeysEqual,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import {
  EMISSIONS_FLAG_BORROW_ACTIVE,
  EMISSIONS_FLAG_LENDING_ACTIVE,
} from "./utils/types";
import { createMintToInstruction } from "@solana/spl-token";
import { deriveEmissionsAuth, deriveEmissionsTokenAccount } from "./utils/pdas";

describe("Lending pool set up emissions", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  const emissionRate = new BN(500_000 * 10 ** ecosystem.tokenBDecimals);
  const totalEmissions = new BN(1_000_000 * 10 ** ecosystem.tokenBDecimals);

  it("Mint token B to the group admin for funding emissions", async () => {
    let tx: Transaction = new Transaction();
    tx.add(
      createMintToInstruction(
        ecosystem.tokenBMint.publicKey,
        groupAdmin.tokenBAccount,
        wallet.publicKey,
        BigInt(100_000_000) * BigInt(10 ** ecosystem.tokenBDecimals)
      )
    );
    await program.provider.sendAndConfirm(tx);
  });

  it("(admin) Set up to token B emissions on (USDC) bank - happy path", async () => {
    const adminBBefore = await getTokenBalance(
      provider,
      groupAdmin.tokenBAccount
    );
    const [emissionsAccKey] = deriveEmissionsTokenAccount(
      program.programId,
      bankKeypairUsdc.publicKey,
      ecosystem.tokenBMint.publicKey
    );
    // Note: an uninitialized account that does nothing...
    const [emissionsAuthKey] = deriveEmissionsAuth(
      program.programId,
      bankKeypairUsdc.publicKey,
      ecosystem.tokenBMint.publicKey
    );

    await groupAdmin.mrgnProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await setupEmissions(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
          bank: bankKeypairUsdc.publicKey,
          emissionsMint: ecosystem.tokenBMint.publicKey,
          fundingAccount: groupAdmin.tokenBAccount,
          emissionsFlags: new BN(
            EMISSIONS_FLAG_BORROW_ACTIVE + EMISSIONS_FLAG_LENDING_ACTIVE
          ),
          emissionsRate: emissionRate,
          totalEmissions: totalEmissions,
        })
      )
    );

    if (verbose) {
      console.log("Started token B borrow/lending emissions on USDC bank");
    }

    const [bank, adminBAfter, emissionsAccAfter] = await Promise.all([
      program.account.bank.fetch(bankKeypairUsdc.publicKey),
      getTokenBalance(provider, groupAdmin.tokenBAccount),
      getTokenBalance(provider, emissionsAccKey),
    ]);

    assertKeysEqual(bank.emissionsMint, ecosystem.tokenBMint.publicKey);
    assertBNEqual(bank.emissionsRate, emissionRate);
    assertI80F48Approx(bank.emissionsRemaining, totalEmissions);
    assertBNEqual(
      bank.flags,
      new BN(EMISSIONS_FLAG_BORROW_ACTIVE + EMISSIONS_FLAG_LENDING_ACTIVE)
    );
    assert.equal(adminBBefore - adminBAfter, totalEmissions.toNumber());
    assert.equal(emissionsAccAfter, totalEmissions.toNumber());
  });

  it("(admin) Add more token B emissions on (USDC) bank - happy path", async () => {
    const [emissionsAccKey] = deriveEmissionsTokenAccount(
      program.programId,
      bankKeypairUsdc.publicKey,
      ecosystem.tokenBMint.publicKey
    );
    const [adminBBefore, emissionsAccBefore] = await Promise.all([
      getTokenBalance(provider, groupAdmin.tokenBAccount),
      getTokenBalance(provider, emissionsAccKey),
    ]);

    await groupAdmin.mrgnProgram!.provider.sendAndConfirm!(
      new Transaction().add(
        await updateEmissions(program, {
          marginfiGroup: marginfiGroup.publicKey,
          admin: groupAdmin.wallet.publicKey,
          bank: bankKeypairUsdc.publicKey,
          emissionsMint: ecosystem.tokenBMint.publicKey,
          fundingAccount: groupAdmin.tokenBAccount,
          emissionsFlags: null,
          emissionsRate: null,
          additionalEmissions: totalEmissions,
        })
      )
    );

    const [bank, adminBAfter, emissionsAccAfter] = await Promise.all([
      program.account.bank.fetch(bankKeypairUsdc.publicKey),
      getTokenBalance(provider, groupAdmin.tokenBAccount),
      getTokenBalance(provider, emissionsAccKey),
    ]);

    assertKeysEqual(bank.emissionsMint, ecosystem.tokenBMint.publicKey);
    assertBNEqual(bank.emissionsRate, emissionRate);
    assertI80F48Approx(bank.emissionsRemaining, totalEmissions.muln(2));
    assertBNEqual(
      bank.flags,
      new BN(EMISSIONS_FLAG_BORROW_ACTIVE + EMISSIONS_FLAG_LENDING_ACTIVE)
    );
    assert.equal(adminBBefore - adminBAfter, totalEmissions.toNumber());
    assert.equal(
      emissionsAccAfter,
      emissionsAccBefore + totalEmissions.toNumber()
    );
  });
});
