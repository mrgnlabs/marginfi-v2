import {
  AnchorProvider,
  BN,
  getProvider,
  Program,
  Wallet,
  workspace,
} from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import { configBankEmode } from "./utils/group-instructions";
import {
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  EMODE_SEED,
  emodeAdmin,
  emodeGroup,
  groupAdmin,
  users,
} from "./rootHooks";
import { assertBankrunTxFailed } from "./utils/genericTests";
import { EMODE_APPLIES_TO_ISOLATED, newEmodeEntry } from "./utils/types";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { deriveBankWithSeed } from "./utils/pdas";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { createMintToInstruction } from "@solana/spl-token";
import { Marginfi } from "../target/types/marginfi";

// By convention, all tags must be in 13375p34k (kidding, but only sorta)
const EMODE_STABLE_TAG = 5748; // STAB because 574813 is out of range
const EMODE_SOL_TAG = 501;
const EMODE_LST_TAG = 157;

const seed = new BN(EMODE_SEED);
let usdcBank: PublicKey;
let solBank: PublicKey;
let lstABank: PublicKey;
let lstBBank: PublicKey;

describe("Init e-mode settings for a set of banks", () => {
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

  it("(user 1) Bad admin tries to edit emode - should fail", async () => {
    let tx = new Transaction();

    tx.add(
      await configBankEmode(users[1].mrgnBankrunProgram, {
        bank: usdcBank,
        tag: EMODE_STABLE_TAG,
        entries: [],
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(users[1].wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    // 6042 (Unauthorized)
    assertBankrunTxFailed(result, "0x179a");
  });

  it("(emode admin) Bad emode settings - should fail", async () => {
    // init > maint weight
    let tx = new Transaction();
    tx.add(
      await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
        bank: solBank,
        tag: EMODE_SOL_TAG,
        entries: [
          newEmodeEntry(
            EMODE_LST_TAG,
            EMODE_APPLIES_TO_ISOLATED,
            bigNumberToWrappedI80F48(0.95),
            bigNumberToWrappedI80F48(0.9)
          ),
        ],
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(emodeAdmin.wallet);
    let result = await banksClient.tryProcessTransaction(tx);
    // 6075 (BadEmodeConfig)
    assertBankrunTxFailed(result, "0x17bb");

    // weight > 1
    tx = new Transaction();
    tx.add(
      await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
        bank: solBank,
        tag: EMODE_SOL_TAG,
        entries: [
          newEmodeEntry(
            EMODE_LST_TAG,
            EMODE_APPLIES_TO_ISOLATED,
            bigNumberToWrappedI80F48(1.9),
            bigNumberToWrappedI80F48(1.95)
          ),
        ],
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(emodeAdmin.wallet);
    result = await banksClient.tryProcessTransaction(tx);
    // 6075 (BadEmodeConfig)
    assertBankrunTxFailed(result, "0x17bb");
  });

  // * Note: you can pack two emode configure ixes into one tx, but that's it, since the data
  //   payload is just over 400 bytes. In production, when editing multiple banks, the emode admin
  //   should use a jito bundle to ensure they all update at the same time and don't trigger
  //   liquidations accidentally.
  // * Note: The default init/maint weight for all banks in this test suite is 0.5/0.6
  it("(emode admin) Configures bank emodes - happy path", async () => {
    let tx = new Transaction();

    tx.add(
      await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
        bank: usdcBank,
        tag: EMODE_STABLE_TAG,
        entries: [
          // USDC doesn't have any favored entries
        ],
      })
    );

    tx.add(
      await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
        bank: solBank,
        tag: EMODE_SOL_TAG,
        entries: [
          newEmodeEntry(
            EMODE_LST_TAG,
            EMODE_APPLIES_TO_ISOLATED,
            bigNumberToWrappedI80F48(0.9),
            bigNumberToWrappedI80F48(0.95)
          ),
        ],
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(emodeAdmin.wallet);
    await banksClient.processTransaction(tx);

    tx = new Transaction().add(
      await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
        bank: lstABank,
        tag: EMODE_LST_TAG,
        entries: [
          newEmodeEntry(
            EMODE_SOL_TAG,
            EMODE_APPLIES_TO_ISOLATED,
            bigNumberToWrappedI80F48(0.85),
            bigNumberToWrappedI80F48(0.9)
          ),
          // Note: borrowing LST against another LST is a fairly common use-case and generally
          // considered little to no risk. In this scenario, the entry is also the bank's own emode
          // tag, and this is not an issue, as you cannot borrow against an asset you are already
          // lending anyways. Since lstBBank shares the same emode risk tag, borrows of lstBBank
          // against lstABank positions will be treated more favorably, as expected.
          newEmodeEntry(
            EMODE_LST_TAG,
            EMODE_APPLIES_TO_ISOLATED,
            bigNumberToWrappedI80F48(0.9),
            bigNumberToWrappedI80F48(0.95)
          ),
        ],
      })
    );

    tx.add(
      await configBankEmode(emodeAdmin.mrgnBankrunProgram, {
        bank: lstBBank,
        tag: EMODE_LST_TAG,
        entries: [
          newEmodeEntry(
            EMODE_SOL_TAG,
            EMODE_APPLIES_TO_ISOLATED,
            bigNumberToWrappedI80F48(0.85),
            bigNumberToWrappedI80F48(0.9)
          ),
          newEmodeEntry(
            EMODE_LST_TAG,
            EMODE_APPLIES_TO_ISOLATED,
            bigNumberToWrappedI80F48(0.9),
            bigNumberToWrappedI80F48(0.95)
          ),
        ],
      })
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(emodeAdmin.wallet);
    await banksClient.processTransaction(tx);
  });

  it("(Fund users/admin USDC/WSOL/LST token accounts", async () => {
    const provider = getProvider() as AnchorProvider;
    const wallet = provider.wallet as Wallet;
    for (let i = 0; i < users.length; i++) {
      let tx = new Transaction();
      // Note: WSOL is really just an spl token in this implementation, we don't simulate the
      // exchange of SOL for WSOL, but that doesn't really matter.
      tx.add(
        createMintToInstruction(
          ecosystem.wsolMint.publicKey,
          users[i].wsolAccount,
          wallet.publicKey,
          100 * 10 ** ecosystem.wsolDecimals
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
      tx.add(
        createMintToInstruction(
          ecosystem.lstAlphaMint.publicKey,
          users[i].lstAlphaAccount,
          wallet.publicKey,
          10000 * 10 ** ecosystem.lstAlphaDecimals
        )
      );
      tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
      tx.sign(wallet.payer);
      await banksClient.processTransaction(tx);
    }

    // Seed the admin with funds as well
    let tx = new Transaction();
    tx.add(
      createMintToInstruction(
        ecosystem.wsolMint.publicKey,
        groupAdmin.wsolAccount,
        wallet.publicKey,
        100 * 10 ** ecosystem.wsolDecimals
      )
    );
    tx.add(
      createMintToInstruction(
        ecosystem.usdcMint.publicKey,
        groupAdmin.usdcAccount,
        wallet.publicKey,
        10000 * 10 ** ecosystem.usdcDecimals
      )
    );
    tx.add(
      createMintToInstruction(
        ecosystem.lstAlphaMint.publicKey,
        groupAdmin.lstAlphaAccount,
        wallet.publicKey,
        10000 * 10 ** ecosystem.lstAlphaDecimals
      )
    );

    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(wallet.payer);
    await banksClient.processTransaction(tx);
  });
});
