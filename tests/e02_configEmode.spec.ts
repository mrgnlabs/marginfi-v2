import { BN } from "@coral-xyz/anchor";
import { AccountMeta, PublicKey, Transaction } from "@solana/web3.js";
import {
  addBankWithSeed,
  groupConfigure,
  groupInitialize,
} from "./utils/group-instructions";
import {
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  EMODE_SEED,
  emodeAdmin,
  emodeGroup,
  groupAdmin,
  oracles,
  verbose,
} from "./rootHooks";
import { assertKeyDefault, assertKeysEqual } from "./utils/genericTests";
import {
  ASSET_TAG_DEFAULT,
  ASSET_TAG_SOL,
  defaultBankConfig,
  ORACLE_SETUP_PYTH_LEGACY,
  ORACLE_SETUP_PYTH_PUSH,
} from "./utils/types";
import { assert } from "chai";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import { deriveBankWithSeed } from "./utils/pdas";

const seed = new BN(EMODE_SEED);
let usdcBank: PublicKey;
let solBank: PublicKey;
let lstABank: PublicKey;
let lstBBank: PublicKey;

describe("Init e-mode enabled group", () => {
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

  it("(emode admin) Configures the 4 banks - happy path", async () => {
    let tx = new Transaction();

    // TODO

    // let actual = await bankrunProgram.account.bank.fetch(PublicKey.default);
    // let entry = actual.emode.emodeConfig.entries[0];

    // tx.add(
    //   await groupConfigure(groupAdmin.mrgnBankrunProgram, {
    //     marginfiGroup: emodeGroup.publicKey,
    //     newEmodeAdmin: emodeAdmin.wallet.publicKey
    //   })
    // );
    // tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    // tx.sign(groupAdmin.wallet, emodeGroup);
    // await banksClient.processTransaction(tx);

    // let group = await bankrunProgram.account.marginfiGroup.fetch(
    //   emodeGroup.publicKey
    // );
    // assertKeysEqual(group.emodeAdmin, emodeAdmin.wallet.publicKey);
  });
});
