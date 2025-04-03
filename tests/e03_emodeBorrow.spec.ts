import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { configBankEmode } from "./utils/group-instructions";
import {
  bankKeypairUsdc,
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  EMODE_SEED,
  emodeAdmin,
  emodeGroup,
  groupAdmin,
  marginfiGroup,
  oracles,
  users,
  verbose,
} from "./rootHooks";
import { assertBankrunTxFailed } from "./utils/genericTests";
import { EMODE_APPLIES_TO_ISOLATED, newEmodeEntry } from "./utils/types";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { deriveBankWithSeed } from "./utils/pdas";
import {
  bigNumberToWrappedI80F48,
  wrappedI80F48toBigNumber,
} from "@mrgnlabs/mrgn-common";
import { createMintToInstruction } from "@solana/spl-token";
import { Marginfi } from "../target/types/marginfi";
import { program } from "@coral-xyz/anchor/dist/cjs/native/system";
import { USER_ACCOUNT, USER_ACCOUNT_E } from "./utils/mocks";
import { accountInit, borrowIx, depositIx } from "./utils/user-instructions";
import { dumpBankrunLogs } from "./utils/tools";

// By convention, all tags must be in 13375p34k (kidding, but only sorta)
const EMODE_STABLE_TAG = 5748; // STAB because 574813 is out of range
const EMODE_SOL_TAG = 501;
const EMODE_LST_TAG = 157;

const seed = new BN(EMODE_SEED);
let usdcBank: PublicKey;
let solBank: PublicKey;
let lstABank: PublicKey;
let lstBBank: PublicKey;

describe("Emode borrowing", () => {
  before(async () => {
    [usdcBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      seed
    );
    [solBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.wsolMint.publicKey,
      seed
    );
    [lstABank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      seed
    );
    [lstBBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      seed.addn(1)
    );
  });

  it("Initialize user accounts (if needed)", async () => {
    for (let i = 0; i < users.length; i++) {
      if (users[i].accounts.get(USER_ACCOUNT_E)) {
        console.log("Skipped creating user " + i);
        continue;
      }
      const userAccKeypair = Keypair.generate();
      const userAccount = userAccKeypair.publicKey;
      users[i].accounts.set(USER_ACCOUNT_E, userAccount);

      let userinitTx: Transaction = new Transaction();
      userinitTx.add(
        await accountInit(users[i].mrgnBankrunProgram, {
          marginfiGroup: emodeGroup.publicKey,
          marginfiAccount: userAccount,
          authority: users[i].wallet.publicKey,
          feePayer: users[i].wallet.publicKey,
        })
      );
      userinitTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      userinitTx.sign(users[i].wallet, userAccKeypair);
      await banksClient.processTransaction(userinitTx);
    }

    if (groupAdmin.accounts.get(USER_ACCOUNT_E)) {
      console.log("Skipped creating admin account");
      return;
    }
    const userAccKeypair = Keypair.generate();
    const userAccount = userAccKeypair.publicKey;
    groupAdmin.accounts.set(USER_ACCOUNT_E, userAccount);

    let userinitTx: Transaction = new Transaction();
    userinitTx.add(
      await accountInit(groupAdmin.mrgnBankrunProgram, {
        marginfiGroup: emodeGroup.publicKey,
        marginfiAccount: userAccount,
        authority: groupAdmin.wallet.publicKey,
        feePayer: groupAdmin.wallet.publicKey,
      })
    );
    userinitTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    userinitTx.sign(groupAdmin.wallet, userAccKeypair);
    await banksClient.processTransaction(userinitTx);
  });

  it("(admin) Seeds liquidity in the banks", async () => {
    const user = groupAdmin;
    const userAccount = user.accounts.get(USER_ACCOUNT_E);

    let tx = new Transaction().add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: usdcBank,
        tokenAccount: user.usdcAccount,
        amount: new BN(100 * 10 ** ecosystem.usdcDecimals),
        depositUpToLimit: false,
      }),
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: solBank,
        tokenAccount: user.wsolAccount,
        amount: new BN(100 * 10 ** ecosystem.wsolDecimals),
        depositUpToLimit: false,
      }),
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: lstABank,
        tokenAccount: user.lstAlphaAccount,
        amount: new BN(100 * 10 ** ecosystem.lstAlphaDecimals),
        depositUpToLimit: false,
      }),
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: lstBBank,
        tokenAccount: user.lstAlphaAccount,
        amount: new BN(100 * 10 ** ecosystem.lstAlphaDecimals),
        depositUpToLimit: false,
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);
  });

  /*
   * SOL is worth $150, and LST is worth $175. Against a 10 SOL position, worth $1500, with a
   * `EMODE_INIT_RATE_SOL_TO_LST` of 90% we expect to borrow .9 * 1500 / 175 ~= 7.71428571429 LST.
   *
   * Due to confidence adjustments the actual SOL price used for risk purposes is ~99% of $150 and
   * the LST liability is ~102% of $175 so .9 * (1500 * .99) / (175 * 1.0424) ~= 7.3264992874 LST
   *
   * With the bank's default rate sans-emode (50%), we could only borrow ~4.28571428571 LST
   *
   * Note: the liability weight was assumed to be 1 in the calculations above, emode never modifies
   * liability weights.
   *
   * Note: To derive the conf bands above, the confidence of Pyth Legacy is 1%, and Pyth Pull is 2%,
   * and we apply a 2.12 constant multiplier, see health pulse printout for actual internal pricing.
   */
  it("(user 0) borrows LST A against SOL at a favorable rate - happy path", async () => {
    const user = users[0];
    const userAccount = user.accounts.get(USER_ACCOUNT_E);

    let tx = new Transaction().add(
      await depositIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: solBank,
        tokenAccount: user.wsolAccount,
        amount: new BN(10 * 10 ** ecosystem.wsolDecimals),
        depositUpToLimit: false,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      await borrowIx(user.mrgnBankrunProgram, {
        marginfiAccount: userAccount,
        bank: lstABank,
        tokenAccount: user.lstAlphaAccount,
        remaining: [
          solBank,
          oracles.wsolOracle.publicKey,
          lstABank,
          oracles.pythPullLst.publicKey,
        ],
        amount: new BN(7.3 * 10 ** ecosystem.lstAlphaDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(user.wallet);
    await banksClient.processTransaction(tx);

    let userAcc = await bankrunProgram.account.marginfiAccount.fetch(
      userAccount
    );
    const cacheAfter = userAcc.healthCache;
    const assetValue = wrappedI80F48toBigNumber(cacheAfter.assetValue);
    const liabValue = wrappedI80F48toBigNumber(cacheAfter.liabilityValue);
    if (verbose) {
      console.log("---user health state---");
      console.log("asset value: " + assetValue.toString());
      console.log("liab value: " + liabValue.toString());
      console.log("prices: ");
      for (let i = 0; i < cacheAfter.prices.length; i++) {
        const price = wrappedI80F48toBigNumber(cacheAfter.prices[i]).toNumber();
        if (price != 0) {
          console.log(" [" + i + "] " + price);
        }
      }
    }

    // TODO assert balances
  });

  // TODO moar tests....
});
