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
  assertI80F48Approx,
  assertI80F48Equal,
  getTokenBalance,
} from "./utils/genericTests";
import { assert } from "chai";
import { borrowIx, depositIx } from "./utils/user-instructions";
import { USER_ACCOUNT } from "./utils/mocks";
import { createMintToInstruction } from "@solana/spl-token";
import { updatePriceAccount } from "./utils/pyth_mocks";

describe("Borrow funds", () => {
  const program = workspace.Marginfi as Program<Marginfi>;
  const provider = getProvider() as AnchorProvider;
  const wallet = provider.wallet as Wallet;

  // Bank has 100 USDC available to borrow
  // User has 2 Token A (worth $20) deposited
  const borrowAmountUsdc = 5;
  const borrowAmountUsdc_native = new BN(
    borrowAmountUsdc * 10 ** ecosystem.usdcDecimals
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

  it("(user 0) borrows USDC against their token A position - happy path", async () => {
    const user = users[0];
    const userUsdcBefore = await getTokenBalance(provider, user.usdcAccount);
    if (verbose) {
      console.log("user 0 USDC before: " + userUsdcBefore.toLocaleString());
    }

    const user0Account = user.accounts.get(USER_ACCOUNT);

    await users[0].userMarginProgram.provider.sendAndConfirm(
      new Transaction().add(
        await borrowIx(program, {
          marginfiGroup: marginfiGroup.publicKey,
          marginfiAccount: user0Account,
          authority: user.wallet.publicKey,
          bank: bankKeypairUsdc.publicKey,
          tokenAccount: user.usdcAccount,
          remaining: [
            bankKeypairA.publicKey,
            oracles.tokenAOracle.publicKey,
            bankKeypairUsdc.publicKey,
            oracles.usdcOracle.publicKey,
          ],
          amount: borrowAmountUsdc_native,
        })
      )
    );

    const userAcc = await program.account.marginfiAccount.fetch(user0Account);
    const balances = userAcc.lendingAccount.balances;
    assert.equal(balances[1].active, true);
    assertI80F48Equal(balances[1].assetShares, 0);
    // Note: The first borrow issues shares 1:1 and the shares use the same decimals
    assertI80F48Approx(balances[1].liabilityShares, borrowAmountUsdc_native);
    assertI80F48Equal(balances[1].emissionsOutstanding, 0);

    let now = Math.floor(Date.now() / 1000);
    assertBNApproximately(balances[1].lastUpdate, now, 2);

    const userUsdcAfter = await getTokenBalance(provider, user.usdcAccount);
    if (verbose) {
      console.log("user 0 USDC after: " + userUsdcAfter.toLocaleString());
    }
    assert.equal(
      userUsdcAfter - borrowAmountUsdc_native.toNumber(),
      userUsdcBefore
    );
  });
});
