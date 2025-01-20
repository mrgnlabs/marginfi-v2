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
  oracles,
  users,
  verbose,
} from "./rootHooks";
import {
  assertBNApproximately,
  assertBNEqual,
  assertI80F48Approx,
  assertI80F48Equal,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import { borrowIx, depositIx, withdrawIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { createMintToInstruction } from "@solana/spl-token";
import { updatePriceAccount } from "./utils/pyth_mocks";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";

describe("Withdraw funds", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  const withdrawAmountTokenA = 0.1;
  const withdrawAmountTokenA_native = new BN(
    withdrawAmountTokenA * 10 ** ecosystem.tokenADecimals
  );

  it("Oracle data refreshes", async () => {
    const usdcPrice = BigInt(oracles.usdcPrice * 10 ** oracles.usdcDecimals);
    await updatePriceAccount(
      oracles.usdcOracle,
      {
        exponent: -oracles.usdcDecimals,
        aggregatePriceInfo: {
          price: usdcPrice,
          conf: usdcPrice / BigInt(100), // 1% of the price
        },
        twap: {
          // aka ema
          valueComponent: usdcPrice,
        },
      },
      wallet
    );

    const tokenAPrice = BigInt(
      oracles.tokenAPrice * 10 ** oracles.tokenADecimals
    );
    await updatePriceAccount(
      oracles.tokenAOracle,
      {
        exponent: -oracles.tokenADecimals,
        aggregatePriceInfo: {
          price: tokenAPrice,
          conf: tokenAPrice / BigInt(100), // 1% of the price
        },
        twap: {
          // aka ema
          valueComponent: tokenAPrice,
        },
      },
      wallet
    );
  });

  it("(user 0) withdraws some token A - happy path", async () => {
    const user = users[0];
    const userAccKey = user.accounts.get(USER_ACCOUNT);
    const userAccBefore = await program.account.marginfiAccount.fetch(
      userAccKey
    );
    const balancesBefore = userAccBefore.lendingAccount.balances;

    const bank = bankKeypairA.publicKey;
    const userTokenABefore = await getTokenBalance(
      provider,
      user.tokenAAccount
    );
    const bankBefore = await program.account.bank.fetch(bank);
    const vaultUsdcBefore = await getTokenBalance(
      provider,
      bankBefore.liquidityVault
    );

    await user.mrgnProgram.provider.sendAndConfirm(
      new Transaction().add(
        await withdrawIx(user.mrgnProgram, {
          marginfiAccount: userAccKey,
          bank: bank,
          tokenAccount: user.tokenAAccount,
          remaining: [
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
          ],
          amount: withdrawAmountTokenA_native,
        })
      )
    );

    const userAccAfter = await program.account.marginfiAccount.fetch(
      userAccKey
    );
    const bankAfter = await program.account.bank.fetch(bank);
    const vaultUsdcAfter = await getTokenBalance(
      provider,
      bankAfter.liquidityVault
    );
    const balancesAfter = userAccAfter.lendingAccount.balances;
    const userTokenAAfter = await getTokenBalance(provider, user.tokenAAccount);

    const withdrawExpected = withdrawAmountTokenA_native.toNumber();
    if (verbose) {
      console.log(
        "User 0 withdrew " +
          withdrawAmountTokenA +
          " token A (" +
          withdrawExpected.toString() +
          ") native"
      );
    }

    // user gains the token A, the liquidity vault loses it....
    assert.equal(userTokenAAfter, userTokenABefore + withdrawExpected);
    assert.equal(vaultUsdcAfter, vaultUsdcBefore - withdrawExpected);

    // User loses the shares of Token A...
    // Since there hasn't been any interest (no Token A borrowed), shares and Token A are 1:1
    const sharesBefore = wrappedI80F48toBigNumber(
      balancesBefore[0].assetShares
    ).toNumber();
    const sharesAfter = wrappedI80F48toBigNumber(
      balancesAfter[0].assetShares
    ).toNumber();
    assert.equal(sharesAfter, sharesBefore - withdrawExpected);

    // The bank has also lost the same amount of shares...
    const bankSharesBefore = wrappedI80F48toBigNumber(
      bankBefore.totalAssetShares
    ).toNumber();
    const bankSharesAfter = wrappedI80F48toBigNumber(
      bankAfter.totalAssetShares
    ).toNumber();
    assert.equal(bankSharesAfter, bankSharesBefore - withdrawExpected);
  });

  // TODO repay, withdraw all, then restore the original balances via deposit/borrow
});
