import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { bankrunProgram, ecosystem, EMODE_SEED, emodeGroup } from "./rootHooks";
import { deriveBankWithSeed } from "./utils/pdas";

// By convention, all tags must be in 13375p34k (kidding, but only sorta)
const EMODE_STABLE_TAG = 5748; // STAB because 574813 is out of range
const EMODE_SOL_TAG = 501;
const EMODE_LST_TAG = 157;

const seed = new BN(EMODE_SEED);
let usdcBank: PublicKey;
let solBank: PublicKey;
let lstABank: PublicKey;
let lstBBank: PublicKey;

describe("Emode liquidation", () => {
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

  it("Emode in effect - Can't liquidate", async () => {
    // TODO can't liquidate
  });

  it("Emode reduced - Can liquidate", async () => {
    // TODO can liquidate after emode reduction/removal
  });
});
