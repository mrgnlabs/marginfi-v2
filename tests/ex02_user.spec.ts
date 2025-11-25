// import { Program } from "@coral-xyz/anchor";
// import { getAssociatedTokenAddressSync } from "@solana/spl-token";
// import { ExponentCore } from "./fixtures/exponent_core";
// import { exponentBankrunProgram, users } from "./rootHooks";
// import { exponentTestState } from "./ex01_exponentInit.spec";
// import {
//   initializeYieldPositionIx,
//   tradePtIx,
// } from "./utils/exponent-instructions";
// import { deriveExponentUserYieldPosition } from "./utils/pdas";
// import { assert } from "chai";

// const program = exponentBankrunProgram as Program<ExponentCore>;

// describe("exponent::user", () => {
//   it("creates a user yield position instruction", async () => {
//     const user = users[0];
//     const userYieldPosition = deriveExponentUserYieldPosition(
//       exponentTestState.vault,
//       user.wallet.publicKey
//     )[0];

//     const ix = await initializeYieldPositionIx(program, {
//       owner: user.wallet.publicKey,
//       vault: exponentTestState.vault,
//       yieldPosition: userYieldPosition,
//     });

//     assert.equal(ix.programId.toBase58(), program.programId.toBase58());
//     const ownerMeta = ix.keys.find((k) =>
//       k.pubkey.equals(user.wallet.publicKey)
//     );
//     assert.isTrue(
//       ownerMeta?.isSigner ?? false,
//       "owner should sign yield init instruction"
//     );
//   });

//   it("allows a second user to buy PT for fixed yield", async () => {
//     const fixedBuyer = users[1];
//     const tokenSyTrader = fixedBuyer.usdcAccount;
//     const tokenPtTrader = getAssociatedTokenAddressSync(
//       exponentTestState.mintPt,
//       fixedBuyer.wallet.publicKey
//     );

//     const ix = await tradePtIx(program, {
//       trader: fixedBuyer.wallet.publicKey,
//       market: exponentTestState.market,
//       tokenSyTrader,
//       tokenPtTrader,
//       tokenSyEscrow: exponentTestState.escrowSy,
//       tokenPtEscrow: exponentTestState.escrowPt,
//       addressLookupTable: exponentTestState.lookupTable,
//       syProgram: program.programId,
//       tokenFeeTreasurySy: users[0].usdcAccount,
//       netTraderPt: BigInt(1_000_000),
//       syConstraint: BigInt(2_000_000),
//     });

//     assert.equal(ix.programId.toBase58(), program.programId.toBase58());
//     const traderMeta = ix.keys.find((k) =>
//       k.pubkey.equals(fixedBuyer.wallet.publicKey)
//     );
//     assert.isTrue(
//       traderMeta?.isSigner ?? false,
//       "fixed buyer should sign PT trade instruction"
//     );
//   });

//   const exponentUserState = {
//     userYieldPosition: deriveExponentUserYieldPosition(
//       exponentTestState.vault,
//       users[0].wallet.publicKey
//     )[0],
//     userYtAta: getAssociatedTokenAddressSync(
//       exponentTestState.mintYt,
//       users[0].wallet.publicKey
//     ),
//     fixedBuyerPtAta: getAssociatedTokenAddressSync(
//       exponentTestState.mintPt,
//       users[1].wallet.publicKey
//     ),
//   };
// });
