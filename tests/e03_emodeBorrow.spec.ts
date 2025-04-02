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
  users,
} from "./rootHooks";
import { assertBankrunTxFailed } from "./utils/genericTests";
import { EMODE_APPLIES_TO_ISOLATED, newEmodeEntry } from "./utils/types";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { deriveBankWithSeed } from "./utils/pdas";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { createMintToInstruction } from "@solana/spl-token";
import { Marginfi } from "../target/types/marginfi";
import { program } from "@coral-xyz/anchor/dist/cjs/native/system";
import { USER_ACCOUNT, USER_ACCOUNT_E } from "./utils/mocks";
import { accountInit, depositIx } from "./utils/user-instructions";
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

    if (emodeAdmin.accounts.get(USER_ACCOUNT_E)) {
      console.log("Skipped creating emode admin");
      return;
    }
    const userAccKeypair = Keypair.generate();
    const userAccount = userAccKeypair.publicKey;
    emodeAdmin.accounts.set(USER_ACCOUNT_E, userAccount);

    let userinitTx: Transaction = new Transaction();
    userinitTx.add(
      await accountInit(emodeAdmin.mrgnBankrunProgram, {
        marginfiGroup: emodeGroup.publicKey,
        marginfiAccount: userAccount,
        authority: emodeAdmin.wallet.publicKey,
        feePayer: emodeAdmin.wallet.publicKey,
      })
    );
    userinitTx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    userinitTx.sign(emodeAdmin.wallet, userAccKeypair);
    await banksClient.processTransaction(userinitTx);
  });

  it("(admin) Seeds liquidity in the banks", async () => {
        const user = users[0];
        const userAccount = user.accounts.get(USER_ACCOUNT_E);
    
        let tx = new Transaction().add(
          await depositIx(user.mrgnBankrunProgram, {
            marginfiAccount: userAccount,
            bank: usdcBank,
            tokenAccount: user.usdcAccount,
            amount: new BN(10 * 10 ** ecosystem.usdcDecimals),
            depositUpToLimit: false,
          }),
          await depositIx(user.mrgnBankrunProgram, {
            marginfiAccount: userAccount,
            bank: solBank,
            tokenAccount: user.wsolAccount,
            amount: new BN(10 * 10 ** ecosystem.wsolDecimals),
            depositUpToLimit: false,
          }),
          await depositIx(user.mrgnBankrunProgram, {
            marginfiAccount: userAccount,
            bank: lstABank,
            tokenAccount: user.lstAlphaAccount,
            amount: new BN(10 * 10 ** ecosystem.lstAlphaDecimals),
            depositUpToLimit: false,
          }),
          await depositIx(user.mrgnBankrunProgram, {
            marginfiAccount: userAccount,
            bank: lstBBank,
            tokenAccount: user.lstAlphaAccount,
            amount: new BN(10 * 10 ** ecosystem.lstAlphaDecimals),
            depositUpToLimit: false,
          })
        );
    
        tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
        tx.sign(user.wallet);
        await banksClient.tryProcessTransaction(tx);
  });
});
