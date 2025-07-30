import { BN } from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  bankrunContext,
  bankrunProgram,
  banksClient,
  ecosystem,
  EMODE_SEED,
  emodeGroup,
  groupAdmin,
  oracles,
} from "./rootHooks";
import { deriveBankWithSeed } from "./utils/pdas";
import { assertKeysEqual } from "./utils/genericTests";
import { dumpAccBalances } from "./utils/tools";
import { defaultBankConfigOptRaw } from "./utils/types";
import { bigNumberToWrappedI80F48 } from "@mrgnlabs/mrgn-common";
import { configureBank } from "./utils/group-instructions";
import { getBankrunBlockhash } from "./utils/spl-staking-utils";
import {
  composeRemainingAccounts,
  liquidateIx,
} from "./utils/user-instructions";
import { getUserMarginfiProgram } from "./utils/mocks";

// Banks are listed here in the sorted-by-public-keys order - the same used in the lending account balances
const seed = new BN(EMODE_SEED);
let usdcBank: PublicKey;
let lstBBank: PublicKey;
let stableBank: PublicKey;
let lstABank: PublicKey;
let solBank: PublicKey;

const WALLET_SEED_3 = Buffer.from("SECRET_WALLET_" + 3 + "00000000000000000");
const WALLET_SEED_4 = Buffer.from("SECRET_WALLET_" + 4 + "00000000000000000");
const gappy3Account = new PublicKey(
  "7qoe1Xmd3WUfPFHQaMYMGwSJT2mU55t3d4C4ZXZ1GJmn"
);
const gappy4Account = new PublicKey(
  "6pbRghQuRw9AsPJqhrGLFRVYDcvfXeGh4zNdYMt8mods"
);
let gappyUser3: Keypair;
let gappyUser4: Keypair;

describe("Liquidation with gaps in accounts", () => {
  before(async () => {
    [usdcBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      seed
    );
    [lstBBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      seed.addn(1)
    );
    [lstABank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.lstAlphaMint.publicKey,
      seed
    );
    [solBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.wsolMint.publicKey,
      seed
    );
    [stableBank] = deriveBankWithSeed(
      bankrunProgram.programId,
      emodeGroup.publicKey,
      ecosystem.usdcMint.publicKey,
      seed.addn(1)
    );

    /*
│ (index) │ Bank PK                                        │ Tag │ Liab Shares    │ Asset Shares      │ Emissions │
├─────────┼────────────────────────────────────────────────┼─────┼────────────────┼───────────────────┼───────────┤
│ 0       │ 'AS6TnrAic2NV4eppLbLwyNj6LYzDV9SdBRjZiJmavYSN' │ 0   │ '-'            │ '10000000.0000'   │ '-'       │
│ 1       │ 'empty'                                        │ '-' │ '-'            │ '-'               │ '-'       │
│ 2       │ '7rw9Bbg3R3vQ3LDApqfrYS1qTV3vFpigWJFUmh7eZfwE' │ 0   │ '-'            │ '1000000000.0000' │ '-'       │
│ 3       │ 'empty'                                        │ '-' │ '-'            │ '-'               │ '-'       │
│ 4       │ 'HPAZgpFHTA9Wqx22Gk1C75Wyonxz1L55LJhXom4uZfaL' │ 0   │ '1000000.0000' │ '-'               │ '-'       │
(lend stable/lst A, borrow usdc)
   */
    gappyUser3 = Keypair.fromSeed(WALLET_SEED_3);

    /*
│ (index) │ Bank PK                                        │ Tag │ Liab Shares      │ Asset Shares      │ Emissions │
├─────────┼────────────────────────────────────────────────┼─────┼──────────────────┼───────────────────┼───────────┤
│ 0       │ '2HDFUn7Ug3Ro2JJ4Kn1tYKuLybRdiHXDCEZ2xe7c4ify' │ 1   │ '-'              │ '1000000000.0000' │ '-'       │
│ 1       │ '7rw9Bbg3R3vQ3LDApqfrYS1qTV3vFpigWJFUmh7eZfwE' │ 0   │ '-'              │ '1000000000.0000' │ '-'       │
│ 2       │ 'empty'                                        │ '-' │ '-'              │ '-'               │ '-'       │
│ 3       │ 'empty'                                        │ '-' │ '-'              │ '-'               │ '-'       │
│ 4       │ 'Ga2Vuevw9mTjEf8wchQgYfXfWB7KqGbKq6sKombnCGPx' │ 0   │ '250000000.0000' │ '-'               │ '-'       │
(lend sol/lst A, borrow lst b)
    */
    gappyUser4 = Keypair.fromSeed(WALLET_SEED_4);
  });

  it("Print gappy user info to validate they have loaded correctly", async () => {
    let gappy3Acc = await bankrunProgram.account.marginfiAccount.fetch(
      gappy3Account
    );
    let gappy4Acc = await bankrunProgram.account.marginfiAccount.fetch(
      gappy4Account
    );

    console.log("bal: " + gappy3Acc.lendingAccount.balances[0].bankPk);
    dumpAccBalances(gappy3Acc);
    dumpAccBalances(gappy3Acc);
    assertKeysEqual(gappy3Acc.authority, gappyUser3.publicKey);
    assertKeysEqual(gappy4Acc.authority, gappyUser4.publicKey);

    // Fund gappy wallets with some SOL for tx fees..
    const tx: Transaction = new Transaction();
    tx.add(
      SystemProgram.transfer({
        fromPubkey: groupAdmin.wallet.publicKey,
        toPubkey: gappyUser3.publicKey,
        lamports: 5 * LAMPORTS_PER_SOL,
      }),
      SystemProgram.transfer({
        fromPubkey: groupAdmin.wallet.publicKey,
        toPubkey: gappyUser4.publicKey,
        lamports: 5 * LAMPORTS_PER_SOL,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
  });

  it("(admin) vastly increase usdc liability ratio to make gappy 3 unhealthy", async () => {
    let config = defaultBankConfigOptRaw();
    config.liabilityWeightInit = bigNumberToWrappedI80F48(210); // 21000%
    config.liabilityWeightMaint = bigNumberToWrappedI80F48(200); // 20000%

    let tx = new Transaction().add(
      await configureBank(groupAdmin.mrgnBankrunProgram, {
        bank: usdcBank,
        bankConfigOpt: config,
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(groupAdmin.wallet);
    await banksClient.processTransaction(tx);
  });

  it("(gappy 4) liquidates gappy 3", async () => {
    const gappy4Program = getUserMarginfiProgram(bankrunProgram, gappyUser4);
    let tx = new Transaction().add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 400_000 }),
      await liquidateIx(gappy4Program, {
        assetBankKey: lstABank,
        liabilityBankKey: usdcBank,
        liquidatorMarginfiAccount: gappy4Account,
        liquidateeMarginfiAccount: gappy3Account,
        remaining: [
          oracles.pythPullLst.publicKey, // asset oracle
          oracles.usdcOracle.publicKey, // liab oracle

          // gappy 4 (lend sol/lst A, borrow lst b)
          ...composeRemainingAccounts([
            // liquidator accounts
            [lstABank, oracles.pythPullLst.publicKey],
            [lstBBank, oracles.pythPullLst.publicKey],
            [usdcBank, oracles.usdcOracle.publicKey], // (new)
            [solBank, oracles.wsolOracle.publicKey],
          ]),

          // gappy 3: (lend stable/lst A, borrow usdc)
          ...composeRemainingAccounts([
            // liquidatee accounts
            [stableBank, oracles.usdcOracle.publicKey],
            [usdcBank, oracles.usdcOracle.publicKey],
            [lstABank, oracles.pythPullLst.publicKey],
          ]),
        ],
        amount: new BN(0.1 * 10 ** ecosystem.usdcDecimals),
      })
    );
    tx.recentBlockhash = await getBankrunBlockhash(bankrunContext);
    tx.sign(gappyUser4);
    await banksClient.processTransaction(tx);
  });
});
