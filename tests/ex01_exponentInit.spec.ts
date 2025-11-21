import { Program } from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { Keypair, PublicKey } from "@solana/web3.js";
import { ExponentCore } from "./fixtures/exponent_core";
import { ecosystem, exponentBankrunProgram, users } from "./rootHooks";
import {
  emptyCpiAccounts,
  ExponentNumber,
  initMarketTwoIx,
  initializeVaultIx,
} from "./utils/exponent-instructions";
import {
  deriveExponentAuthority,
  deriveExponentEscrowYt,
  deriveExponentEscrowLp,
  deriveExponentEscrowPt,
  deriveExponentEscrowSyForMarket,
  deriveExponentMarket,
  deriveExponentMintLp,
  deriveExponentMintPt,
  deriveExponentMintYt,
  deriveExponentVaultYieldPosition,
} from "./utils/pdas";
import { EXPONENT_ADMIN_PROGRAM_ID } from "./utils/types";
import { assert } from "chai";

const TOKEN_METADATA_PROGRAM_ID = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);

// Deterministic vault to re-use across the Exponent suite
const VAULT_SEED = Buffer.from("EXPONENT_VAULT_SEED_0000000000000000");
const vaultKeypair = Keypair.fromSeed(VAULT_SEED);

// Deterministic admin account derived from the Exponent admin program seed
const [adminPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("admin")],
  EXPONENT_ADMIN_PROGRAM_ID
);

const vaultAuthority = deriveExponentAuthority(vaultKeypair.publicKey)[0];
const mintPt = deriveExponentMintPt(vaultKeypair.publicKey)[0];
const mintYt = deriveExponentMintYt(vaultKeypair.publicKey)[0];
const mintLp = deriveExponentMintLp(vaultKeypair.publicKey)[0];
const escrowYt = deriveExponentEscrowYt(vaultKeypair.publicKey)[0];
const escrowPt = deriveExponentEscrowPt(vaultKeypair.publicKey)[0];
const market = deriveExponentMarket(vaultKeypair.publicKey)[0];
const escrowSy = deriveExponentEscrowSyForMarket(vaultKeypair.publicKey)[0];
const escrowLp = deriveExponentEscrowLp(vaultKeypair.publicKey)[0];
const vaultYieldPosition = deriveExponentVaultYieldPosition(
  vaultKeypair.publicKey,
  vaultAuthority
)[0];
const metadata = PublicKey.findProgramAddressSync(
  [
    Buffer.from("metadata"),
    TOKEN_METADATA_PROGRAM_ID.toBuffer(),
    mintPt.toBuffer(),
  ],
  TOKEN_METADATA_PROGRAM_ID
)[0];

const lookupTable = Keypair.generate().publicKey;

const program = exponentBankrunProgram as Program<ExponentCore>;

describe("exponent::init", () => {
  it("builds initialize vault instruction", async () => {
    const escrowSyVault = getAssociatedTokenAddressSync(
      ecosystem.usdcMint.publicKey,
      vaultAuthority,
      true
    );

    const ix = await initializeVaultIx(program, {
      payer: users[0].wallet.publicKey,
      admin: adminPda,
      authority: vaultAuthority,
      vault: vaultKeypair.publicKey,
      mintPt,
      mintYt,
      escrowYt,
      escrowSy: escrowSyVault,
      mintSy: ecosystem.usdcMint.publicKey,
      treasuryTokenAccount: users[0].usdcAccount,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      syProgram: program.programId,
      addressLookupTable: lookupTable,
      yieldPosition: vaultYieldPosition,
      metadata,
      tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
      startTimestamp: 1,
      duration: 60,
      interestBpsFee: 0,
      cpiAccounts: emptyCpiAccounts(),
      minOpSizeStrip: new BN(1_000_000),
      minOpSizeMerge: new BN(1_000_000),
      ptMetadataName: "Exponent PT",
      ptMetadataSymbol: "ePT",
      ptMetadataUri: "https://example.com/metadata.json",
    });

    assert.equal(ix.programId.toBase58(), program.programId.toBase58());
    const authorityMeta = ix.keys.find((k) => k.pubkey.equals(vaultAuthority));
    assert.isDefined(authorityMeta, "authority pda must be wired");
  });

  it("initializes a market for trading PT and YT", async () => {
    const user = users[0];
    const ptSrc = getAssociatedTokenAddressSync(mintPt, user.wallet.publicKey);
    const sySrc = user.usdcAccount;
    const lpDst = getAssociatedTokenAddressSync(mintLp, user.wallet.publicKey);

    const syExchangeRate: ExponentNumber = { mantissa: new BN(1), exp: 0 };

    const ix = await initMarketTwoIx(program, {
      payer: user.wallet.publicKey,
      adminSigner: user.wallet.publicKey,
      market,
      vault: vaultKeypair.publicKey,
      mintSy: ecosystem.usdcMint.publicKey,
      mintPt,
      mintLp,
      escrowPt,
      escrowSy,
      escrowLp,
      ptSrc,
      sySrc,
      lpDst,
      tokenProgram: TOKEN_PROGRAM_ID,
      syProgram: program.programId,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      addressLookupTable: lookupTable,
      admin: adminPda,
      tokenTreasuryFeeSy: user.usdcAccount,
      lnFeeRateRoot: 0.01,
      rateScalarRoot: 0.5,
      initRateAnchor: 0.25,
      syExchangeRate,
      ptInit: new BN(1_000_000),
      syInit: new BN(1_000_000),
      feeTreasurySyBps: 0,
      cpiAccounts: emptyCpiAccounts(),
      seedId: 0,
    });

    assert.equal(ix.programId.toBase58(), program.programId.toBase58());
    const marketMeta = ix.keys.find((k) => k.pubkey.equals(market));
    assert.isDefined(marketMeta, "market pda must be connected");
  });
});

export const exponentTestState = {
  vault: vaultKeypair.publicKey,
  vaultAuthority,
  mintPt,
  mintYt,
  mintLp,
  escrowYt,
  escrowPt,
  escrowSy,
  escrowLp,
  vaultYieldPosition,
  market,
  lookupTable,
};
