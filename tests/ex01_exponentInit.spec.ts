import { BN, Program } from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  AddressLookupTableProgram,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  bankrunContext,
  bankRunProvider,
  users,
} from "./rootHooks";
import {
  ExponentCpiAccounts,
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
import { EXPONENT_ADMIN_PROGRAM_ID, MOCKS_PROGRAM_ID } from "./utils/types";
import { processBankrunTransaction } from "./utils/tools";
import { Mocks } from "../target/types/mocks";
import mocksIdl from "../target/idl/mocks.json";

// Deterministic vault to re-use across the Exponent suite
const VAULT_SEED = Buffer.from("VAULT_E_BANK_SEED_00000000000000");
const vaultKeypair = Keypair.fromSeed(VAULT_SEED);

// Deterministic admin account derived from the Exponent admin program seed
const [adminPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("admin")],
  EXPONENT_ADMIN_PROGRAM_ID
);

const marketSeedId = 0;
const vaultAuthority = deriveExponentAuthority(vaultKeypair.publicKey)[0];
const mintPt = deriveExponentMintPt(vaultKeypair.publicKey)[0];
const mintYt = deriveExponentMintYt(vaultKeypair.publicKey)[0];
const market = deriveExponentMarket(vaultKeypair.publicKey, marketSeedId)[0];
const mintLp = deriveExponentMintLp(market)[0];
const escrowYt = deriveExponentEscrowYt(vaultKeypair.publicKey)[0];
const escrowPt = deriveExponentEscrowPt(market)[0];
const marketEscrowSy = deriveExponentEscrowSyForMarket(market)[0];
const escrowLp = deriveExponentEscrowLp(market)[0];
const vaultYieldPosition = deriveExponentVaultYieldPosition(
  vaultKeypair.publicKey,
  vaultAuthority
)[0];

const syPersonal = Keypair.generate();
let lookupTable: PublicKey;
let syState: PublicKey;
let syMint: PublicKey;
let syPool: PublicKey;
let userSyTokenAccount: PublicKey;
let vaultEscrowSy: PublicKey;
let vaultCpiAccounts: ExponentCpiAccounts;
let marketCpiAccounts: ExponentCpiAccounts;
let setupComplete = false;
const EXCHANGE_RATE_ONE: ExponentNumber = { mantissa: new BN(1), exp: 0 };
const SY_MINT_AMOUNT = new BN(5_000_000);
const STRIP_AMOUNT = new BN(2_000_000);
const MARKET_PT_INIT = new BN(1_000_000);
const MARKET_SY_INIT = new BN(1_000_000);

export const exponentTestState: {
  vault: PublicKey;
  vaultAuthority: PublicKey;
  mintPt: PublicKey;
  mintYt: PublicKey;
  mintLp: PublicKey;
  escrowYt: PublicKey;
  escrowPt: PublicKey;
  escrowSy: PublicKey;
  escrowLp: PublicKey;
  vaultYieldPosition: PublicKey;
  market: PublicKey;
  lookupTable?: PublicKey;
  syMint?: PublicKey;
  syState?: PublicKey;
  syPool?: PublicKey;
  vaultEscrowSy?: PublicKey;
  userSyTokenAccount?: PublicKey;
} = {
  vault: vaultKeypair.publicKey,
  vaultAuthority,
  mintPt,
  mintYt,
  mintLp,
  escrowYt,
  escrowPt,
  escrowSy: marketEscrowSy,
  escrowLp,
  vaultYieldPosition,
  market,
};

const buildCpiAccounts = (
  fromSyIndex: number,
  authorityIndex: number
): ExponentCpiAccounts => ({
  getSyState: [{ altIndex: 0, isSigner: false, isWritable: false }],
  depositSy: [
    { altIndex: 0, isSigner: false, isWritable: true }, // state
    { altIndex: 1, isSigner: false, isWritable: true }, // personal
    { altIndex: fromSyIndex, isSigner: false, isWritable: true }, // from_sy
    { altIndex: 3, isSigner: false, isWritable: true }, // pool_sy
    { altIndex: authorityIndex, isSigner: true, isWritable: false }, // authority
    { altIndex: 5, isSigner: false, isWritable: false }, // token_program
    { altIndex: 6, isSigner: true, isWritable: false }, // owner
  ],
  withdrawSy: [],
  claimEmission: [],
  getPositionState: [],
});

const setupSyProgram = async (user: (typeof users)[number]) => {
  if (setupComplete) return;

  const mocksProgram = new Program<Mocks>(
    mocksIdl as Mocks,
    bankRunProvider
  );

  const [state] = PublicKey.findProgramAddressSync(
    [Buffer.from("state"), user.wallet.publicKey.toBuffer()],
    MOCKS_PROGRAM_ID
  );
  const [mintSyPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("mint_sy"), state.toBuffer()],
    MOCKS_PROGRAM_ID
  );
  const [poolSyPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("pool_sy"), state.toBuffer()],
    MOCKS_PROGRAM_ID
  );

  syState = state;
  syMint = mintSyPda;
  syPool = poolSyPda;
  vaultEscrowSy = getAssociatedTokenAddressSync(syMint, vaultAuthority, true);

  const initStateIx = await mocksProgram.methods
    .initState(EXCHANGE_RATE_ONE)
    .accounts({
      state: syState,
      mintSy: syMint,
      poolSy: syPool,
      admin: user.wallet.publicKey,
      payer: user.wallet.publicKey,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();

  await processBankrunTransaction(
    bankrunContext,
    new Transaction().add(initStateIx),
    [user.wallet]
  );

  const altAddresses = [
    syState,
    syPersonal.publicKey,
    vaultEscrowSy,
    syPool,
    vaultAuthority,
    TOKEN_PROGRAM_ID,
    user.wallet.publicKey,
    marketEscrowSy,
    market,
  ];

  const recentSlot = await bankRunProvider.connection.getSlot();
  const [createLookupIx, lookupAddr] =
    AddressLookupTableProgram.createLookupTable({
      authority: user.wallet.publicKey,
      payer: user.wallet.publicKey,
      recentSlot,
    });
  const extendLookupIx = AddressLookupTableProgram.extendLookupTable({
    authority: user.wallet.publicKey,
    payer: user.wallet.publicKey,
    lookupTable: lookupAddr,
    addresses: altAddresses,
  });

  await processBankrunTransaction(
    bankrunContext,
    new Transaction().add(createLookupIx, extendLookupIx),
    [user.wallet]
  );

  lookupTable = lookupAddr;
  vaultCpiAccounts = buildCpiAccounts(2, 4);
  marketCpiAccounts = buildCpiAccounts(7, 8);

  userSyTokenAccount = getAssociatedTokenAddressSync(
    syMint,
    user.wallet.publicKey
  );
  const createSyAtaIx = createAssociatedTokenAccountInstruction(
    user.wallet.publicKey,
    userSyTokenAccount,
    user.wallet.publicKey,
    syMint
  );
  const mintSyIx = await mocksProgram.methods
    .mintSy(SY_MINT_AMOUNT)
    .accounts({
      state: syState,
      mintSy: syMint,
      dstSy: userSyTokenAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();

  await processBankrunTransaction(
    bankrunContext,
    new Transaction().add(createSyAtaIx, mintSyIx),
    [user.wallet]
  );

  Object.assign(exponentTestState, {
    lookupTable,
    syMint,
    syState,
    syPool,
    vaultEscrowSy,
    userSyTokenAccount,
    escrowSy: marketEscrowSy,
  });

  setupComplete = true;
};

// TODO init sy program state
describe("exponent::init", () => {
  it("initialize exponent YT vault", async () => {
    const user = users[0];
    const program = user.exponentBankrunProgram;

    await setupSyProgram(user);

    const escrowSyVault = vaultEscrowSy;
    const remainingAccounts = [
      { pubkey: syPersonal.publicKey, isWritable: true, isSigner: true },
      { pubkey: user.wallet.publicKey, isWritable: true, isSigner: true },
      { pubkey: SystemProgram.programId, isWritable: false, isSigner: false },
    ];

    const tx = new Transaction().add(
      await initializeVaultIx(program, {
        payer: user.wallet.publicKey,
        admin: adminPda,
        authority: vaultAuthority,
        vault: vaultKeypair.publicKey,
        mintPt,
        mintYt,
        escrowYt,
        escrowSy: escrowSyVault,
        mintSy: syMint,
        treasuryTokenAccount: userSyTokenAccount,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        syProgram: MOCKS_PROGRAM_ID,
        addressLookupTable: lookupTable,
        yieldPosition: vaultYieldPosition,
        // It's less annoying to skip all metadata initiation on localnet.
        metadata: PublicKey.default,
        tokenMetadataProgram: PublicKey.default,
        startTimestamp: 1,
        duration: 60,
        interestBpsFee: 0,
        cpiAccounts: vaultCpiAccounts,
        minOpSizeStrip: new BN(1_000_000),
        minOpSizeMerge: new BN(1_000_000),
        ptMetadataName: "Exponent PT",
        ptMetadataSymbol: "ePT",
        ptMetadataUri: "https://example.com/metadata.json",
        remainingAccounts: remainingAccounts,
      })
    );

    await processBankrunTransaction(bankrunContext, tx, [
      user.wallet,
      vaultKeypair,
      syPersonal,
    ]);
  });

  // TODO complete this stub and actually send this tx
  it("initializes a market for trading PT and YT", async () => {
    const user = users[0];
    const program = user.exponentBankrunProgram;
    await setupSyProgram(user);

    const ptSrc = getAssociatedTokenAddressSync(mintPt, user.wallet.publicKey);
    const ytDst = getAssociatedTokenAddressSync(mintYt, user.wallet.publicKey);
    const sySrc = userSyTokenAccount;
    const lpDst = getAssociatedTokenAddressSync(mintLp, user.wallet.publicKey);

    const createUserAtasTx = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        user.wallet.publicKey,
        ptSrc,
        user.wallet.publicKey,
        mintPt
      ),
      createAssociatedTokenAccountInstruction(
        user.wallet.publicKey,
        ytDst,
        user.wallet.publicKey,
        mintYt
      )
    );

    await processBankrunTransaction(bankrunContext, createUserAtasTx, [
      user.wallet,
    ]);

    const stripIx = await program.methods
      .strip(STRIP_AMOUNT)
      .accounts({
        depositor: user.wallet.publicKey,
        authority: vaultAuthority,
        vault: vaultKeypair.publicKey,
        sySrc,
        escrowSy: vaultEscrowSy,
        ytDst,
        ptDst: ptSrc,
        mintYt,
        mintPt,
        tokenProgram: TOKEN_PROGRAM_ID,
        addressLookupTable: lookupTable,
        syProgram: MOCKS_PROGRAM_ID,
        yieldPosition: vaultYieldPosition,
      })
      .remainingAccounts([
        { pubkey: syState, isWritable: true, isSigner: false },
        { pubkey: syPersonal.publicKey, isWritable: true, isSigner: false },
        { pubkey: syPool, isWritable: true, isSigner: false },
      ])
      .instruction();

    await processBankrunTransaction(
      bankrunContext,
      new Transaction().add(stripIx),
      [user.wallet]
    );

    const ix = await initMarketTwoIx(program, {
      payer: user.wallet.publicKey,
      adminSigner: user.wallet.publicKey,
      market,
      vault: vaultKeypair.publicKey,
      mintSy: syMint,
      mintPt,
      mintLp,
      escrowPt,
      escrowSy: marketEscrowSy,
      escrowLp,
      ptSrc,
      sySrc,
      lpDst,
      tokenProgram: TOKEN_PROGRAM_ID,
      syProgram: MOCKS_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      addressLookupTable: lookupTable,
      admin: adminPda,
      tokenTreasuryFeeSy: sySrc,
      lnFeeRateRoot: 0.01,
      rateScalarRoot: 0.5,
      initRateAnchor: 0.25,
      syExchangeRate: EXCHANGE_RATE_ONE,
      ptInit: MARKET_PT_INIT,
      syInit: MARKET_SY_INIT,
      feeTreasurySyBps: 0,
      cpiAccounts: marketCpiAccounts,
      seedId: marketSeedId,
      remainingAccounts: [
        { pubkey: syState, isWritable: true, isSigner: false },
        { pubkey: syPersonal.publicKey, isWritable: true, isSigner: false },
        { pubkey: syPool, isWritable: true, isSigner: false },
      ],
    });

    await processBankrunTransaction(
      bankrunContext,
      new Transaction().add(ix),
      [user.wallet],
      true,
      true
    );
  });
});
