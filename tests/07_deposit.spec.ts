import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairUsdc,
  ecosystem,
  marginfiGroup,
  users,
  verbose,
} from "./rootHooks";
import {
  assertBNApproximately,
  assertI80F48Approx,
  assertI80F48Equal,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import { depositIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { createMintToInstruction } from "@solana/spl-token";
import { deriveLiquidityVault } from "./utils/pdas";

describe("Deposit funds", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;
  const depositAmountA = 2;
  const depositAmountA_native = new BN(
    depositAmountA * 10 ** ecosystem.tokenADecimals
  );

  const depositAmountUsdc = 100;
  const depositAmountUsdc_native = new BN(
    depositAmountUsdc * 10 ** ecosystem.usdcDecimals
  );

  it("(Fund user 0 and user 1 USDC/Token A token accounts", async () => {
    let tx = new Transaction();
    for (let i = 0; i < users.length; i++) {
      tx.add(
        createMintToInstruction(
          ecosystem.tokenAMint.publicKey,
          users[i].tokenAAccount,
          wallet.publicKey,
          100 * 10 ** ecosystem.tokenADecimals
        )
      );
      tx.add(
        createMintToInstruction(
          ecosystem.usdcMint.publicKey,
          users[i].usdcAccount,
          wallet.publicKey,
          10000 * 10 ** ecosystem.usdcDecimals
        )
      );
    }
    await program.provider.sendAndConfirm(tx);
  });

  it("(user 0) deposit token A to bank - happy path", async () => {
    const user = users[0];
    const [bankLiquidityVault] = deriveLiquidityVault(
      program.programId,
      bankKeypairA.publicKey
    );
    const [userABefore, vaultABefore] = await Promise.all([
      getTokenBalance(provider, user.tokenAAccount),
      getTokenBalance(provider, bankLiquidityVault),
    ]);
    if (verbose) {
      console.log("user 0 A before: " + userABefore.toLocaleString());
      console.log("vault A before:  " + vaultABefore.toLocaleString());
    }

    const user0Account = user.accounts.get(USER_ACCOUNT);

    await users[0].mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await depositIx(program, {
          marginfiGroup: marginfiGroup.publicKey,
          marginfiAccount: user0Account,
          authority: user.wallet.publicKey,
          bank: bankKeypairA.publicKey,
          tokenAccount: user.tokenAAccount,
          amount: depositAmountA_native,
        })
      )
    );

    const userAcc = await program.account.marginfiAccount.fetch(user0Account);
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[0].active, true);
    // Note: The first deposit issues shares 1:1 and the shares use the same decimals
    assertI80F48Approx(balances[0].assetShares, depositAmountA_native);
    assertI80F48Equal(balances[0].liabilityShares, 0);
    assertI80F48Equal(balances[0].emissionsOutstanding, 0);

    let now = Math.floor(Date.now() / 1000);
    assertBNApproximately(balances[0].lastUpdate, now, 2);

    const [userAAfter, vaultAAfter] = await Promise.all([
      getTokenBalance(provider, user.tokenAAccount),
      getTokenBalance(provider, bankLiquidityVault),
    ]);
    if (verbose) {
      console.log("user 0 A after: " + userAAfter.toLocaleString());
      console.log("vault A after:  " + vaultAAfter.toLocaleString());
    }
    assert.equal(userABefore - depositAmountA_native.toNumber(), userAAfter);
    assert.equal(vaultABefore + depositAmountA_native.toNumber(), vaultAAfter);
  });

  it("(user 1) deposit USDC to bank - happy path", async () => {
    const user = users[1];
    const userUsdcBefore = await getTokenBalance(provider, user.usdcAccount);
    if (verbose) {
      console.log("user 1 USDC before: " + userUsdcBefore.toLocaleString());
    }

    const user1Account = user.accounts.get(USER_ACCOUNT);

    await users[1].mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await depositIx(program, {
          marginfiGroup: marginfiGroup.publicKey,
          marginfiAccount: user1Account,
          authority: user.wallet.publicKey,
          bank: bankKeypairUsdc.publicKey,
          tokenAccount: user.usdcAccount,
          amount: depositAmountUsdc_native,
        })
      )
    );

    const userAcc = await program.account.marginfiAccount.fetch(user1Account);
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[0].active, true);
    // Note: The first deposit issues shares 1:1 and the shares use the same decimals
    assertI80F48Approx(balances[0].assetShares, depositAmountUsdc_native);
    assertI80F48Equal(balances[0].liabilityShares, 0);
    assertI80F48Equal(balances[0].emissionsOutstanding, 0);

    let now = Math.floor(Date.now() / 1000);
    assertBNApproximately(balances[0].lastUpdate, now, 2);

    const userUsdcAfter = await getTokenBalance(provider, user.usdcAccount);
    if (verbose) {
      console.log("user 1 USDC after: " + userUsdcAfter.toLocaleString());
    }
    assert.equal(
      userUsdcBefore - depositAmountUsdc_native.toNumber(),
      userUsdcAfter
    );
  });
});
