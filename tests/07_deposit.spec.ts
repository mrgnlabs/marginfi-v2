import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { AccountMeta, Transaction } from "@solana/web3.js";
import { Marginfi } from "../target/types/marginfi";
import {
  bankKeypairA,
  bankKeypairUsdc,
  ecosystem,
  groupAdmin,
  marginfiGroup,
  oracles,
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
import { depositIx, withdrawIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { createMintToInstruction } from "@solana/spl-token";
import { deriveBankWithSeed, deriveLiquidityVault } from "./utils/pdas";
import { addBank, addBankWithSeed } from "./utils/group-instructions";
import {
  defaultBankConfig,
  ORACLE_SETUP_PYTH_LEGACY,
  u64MAX_BN,
} from "./utils/types";

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

    await user.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await depositIx(user.mrgnProgram, {
          marginfiAccount: user0Account,
          bank: bankKeypairA.publicKey,
          tokenAccount: user.tokenAAccount,
          amount: depositAmountA_native,
          depositUpToLimit: false,
        })
      )
    );

    const userAcc = await program.account.marginfiAccount.fetch(user0Account);
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[0].active, 1);
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

    await user.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await depositIx(user.mrgnProgram, {
          marginfiAccount: user1Account,
          bank: bankKeypairUsdc.publicKey,
          tokenAccount: user.usdcAccount,
          amount: depositAmountUsdc_native,
          depositUpToLimit: false,
        })
      )
    );

    const userAcc = await program.account.marginfiAccount.fetch(user1Account);
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[0].active, 1);
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

  it("(user 1) deposit up to limit - happy path", async () => {
    const depositAmount0 = 500;
    const depositLimit = 10000;

    // Init a dummy bank for this test...
    let config = defaultBankConfig();
    config.depositLimit = new BN(10_000);
    const seed = new BN(0);
    const [bankKey] = deriveBankWithSeed(
      program.programId,
      marginfiGroup.publicKey,
      ecosystem.tokenAMint.publicKey,
      seed
    );
    await groupAdmin.mrgnProgram.provider.sendAndConfirm!(
      new Transaction().add(
        await addBankWithSeed(groupAdmin.mrgnProgram, {
          marginfiGroup: marginfiGroup.publicKey,
          feePayer: groupAdmin.wallet.publicKey,
          bankMint: ecosystem.tokenAMint.publicKey,
          bank: bankKey,
          // globalFeeWallet: globalFeeWallet,
          config: config,
          seed: seed,
        }),
        await program.methods
          .lendingPoolConfigureBankOracle(
            ORACLE_SETUP_PYTH_LEGACY,
            oracles.tokenAOracle.publicKey
          )
          .accountsPartial({
            group: marginfiGroup.publicKey,
            bank: bankKey,
            admin: groupAdmin.wallet.publicKey,
          })
          .remainingAccounts([
            {
              pubkey: oracles.tokenAOracle.publicKey,
              isSigner: false,
              isWritable: false,
            } as AccountMeta,
          ])
          .instruction()
      )
    );

    // User 0 deposits a small amount of funds...
    const user0Account = users[0].accounts.get(USER_ACCOUNT);
    await users[0].mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await depositIx(users[0].mrgnProgram, {
          marginfiAccount: user0Account,
          bank: bankKey,
          tokenAccount: users[0].tokenAAccount,
          amount: new BN(depositAmount0),
          depositUpToLimit: false,
        })
      )
    );

    // And now user user 1 attempts to deposit up to the deposit cap
    const user = users[1];
    const userTokenABefore = await getTokenBalance(
      provider,
      user.tokenAAccount
    );
    if (verbose) {
      console.log(
        "user 1 Token A before: " + userTokenABefore.toLocaleString()
      );
    }

    const user1Account = user.accounts.get(USER_ACCOUNT);
    await user.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await depositIx(user.mrgnProgram, {
          marginfiAccount: user1Account,
          bank: bankKey,
          tokenAccount: user.tokenAAccount,
          // NOTE: Pass u64::MAX to go up to the deposit limit regardless of amount, or pass some
          // smaller amount to clamp to that amount (the actual amount deposited is always
          // min(amount, deposit_amt_up_to_cap))
          amount: u64MAX_BN,
          depositUpToLimit: true,
        })
      )
    );

    const userTokenAAfter = await getTokenBalance(provider, user.tokenAAccount);
    if (verbose) {
      console.log("user 1 Token A after: " + userTokenAAfter.toLocaleString());
    }
    // Note: We are always 1 token short of the deposit limit, because an internal check performs a
    // < instead of a <= when validating the deposit limit
    const expected = depositLimit - depositAmount0 - 1;
    assert.equal(
      userTokenABefore - userTokenAAfter,
      depositLimit - depositAmount0 - 1
    );
    const userAcc = await program.account.marginfiAccount.fetch(user1Account);
    assertI80F48Approx(
      userAcc.lendingAccount.balances[1].assetShares,
      expected
    );

    // withdraw amounts to restore to previous state...

    await user.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await withdrawIx(user.mrgnProgram, {
          marginfiAccount: user1Account,
          bank: bankKey,
          tokenAccount: user.tokenAAccount,
          remaining: [
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
            bankKey,
            oracles.tokenAOracle.publicKey,
          ],
          amount: new BN(1), // doesn't matter when withdrawing all...
          withdrawAll: true,
        })
      )
    );

    await users[0].mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await withdrawIx(users[0].mrgnProgram, {
          marginfiAccount: user0Account,
          bank: bankKey,
          tokenAccount: users[0].tokenAAccount,
          remaining: [
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
            bankKey,
            oracles.tokenAOracle.publicKey,
          ],
          amount: new BN(1), // doesn't matter when withdrawing all...
          withdrawAll: true,
        })
      )
    );
  });
});
