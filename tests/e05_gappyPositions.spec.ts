import { BN } from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { bankrunProgram, ecosystem, EMODE_SEED, emodeGroup } from "./rootHooks";
import { deriveBankWithSeed } from "./utils/pdas";
import { assertKeysEqual } from "./utils/genericTests";
import { dumpAccBalances } from "./utils/tools";

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

/** USDC funding for the liquidator (user 2) */
const liquidator_usdc: number = 10;
/** SOL funding for the liquidator (user 2) */
const liquidator_sol: number = 0.1;

const REDUCED_INIT_SOL_LST_RATE = 0.85;
const REDUCED_MAINT_SOL_LST_RATE = 0.9;

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
  });
});
