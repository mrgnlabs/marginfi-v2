import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import BigNumber from "bignumber.js";
import {
  NodeWallet,
  getConfig,
  MarginfiClient,
  AccountType,
  loadKeypair,
  MarginfiGroup,
  uiToNative,
  USDC_DECIMALS,
  nativeToUi,
} from "@mrgnlabs/marginfi-client-v2";
import MarginfiAccount, {
  MarginRequirementType,
} from "@mrgnlabs/marginfi-client-v2/src/account";
import Bank, { PriceBias } from "@mrgnlabs/marginfi-client-v2/src/bank";
import bs58 from "bs58";
import fetch from "node-fetch";
import JSBI from "jsbi";
import { Jupiter, RouteInfo, TOKEN_LIST_URL } from "@jup-ag/core";
import { Token } from "@jup-ag/core/dist/lib/amms/marcoPolo/type";
import { associatedAddress } from "@project-serum/anchor/dist/esm/utils/token";

const LIQUIDATOR_PK = new PublicKey(process.env.LIQUIDATOR_PK);
const connection = new Connection(process.env.RPC_ENDPOINT, "confirmed");
const wallet = new NodeWallet(loadKeypair(process.env.KEYPAIR_PATH));
const MARGINFI_GROUP_PK = new PublicKey(process.env.MARGINFI_GROUP_PK);

const USCD_MINT = new PublicKey("");

async function processAccount(
  group: MarginfiGroup,
  client: MarginfiClient,
  liquidatorAccount: MarginfiAccount,
  marginfiAccountAddress: PublicKey,
  jupiter: Jupiter
) {
  const marginfiAccount = await MarginfiAccount.fetch(
    marginfiAccountAddress,
    client
  );
  const usdcBank = group.getBankByLabel("USDC")!;
  if (marginfiAccount.canBeLiquidated()) {
    const [assets, liabs] = marginfiAccount.getHealthComponents(
      MarginRequirementType.Maint
    );

    const maxLiabilityPaydown = liabs.minus(assets);
    console.log(
      "Account can be liquidated, max liability paydown: %d",
      maxLiabilityPaydown
    );
  } else {
    console.log("Account cannot be liquidated");
  }

  if (!marginfiAccount.canBeLiquidated()) {
    console.log("Account is healthy");
  }

  let maxLiabilityPaydownUsdValue = new BigNumber(0);
  let bestLiabAccountIndex = 0;

  // Find biggest liability account that can be covered by liquidator
  // within the liquidators liquidation capacity
  for (let i = 0; i < marginfiAccount.lendingAccount.length; i++) {
    const balance = marginfiAccount.lendingAccount[i];
    const bank = group.getBankByPk(balance.bankPk)!;
    const maxLiabCoverage = liquidatorAccount.getMaxWithdrawForBank(bank);
    const liquidatorLiabPayoffCapacityUsd = bank.getUsdValue(
      maxLiabCoverage,
      PriceBias.None
    );
    const [_, liquidateeLiabUsdValue] = balance.getUsdValue(
      bank,
      MarginRequirementType.Equity
    );

    const liabUsdValue = BigNumber.max(
      liquidateeLiabUsdValue,
      liquidatorLiabPayoffCapacityUsd
    );

    if (liabUsdValue.gt(maxLiabilityPaydownUsdValue)) {
      maxLiabilityPaydownUsdValue = liquidatorLiabPayoffCapacityUsd;
      bestLiabAccountIndex = i;
    }
  }

  let maxCollateralUsd = new BigNumber(0);
  let bestCollateralIndex = 0;

  // Find biggest collateral account
  for (let i = 0; i < marginfiAccount.lendingAccount.length; i++) {
    const balance = marginfiAccount.lendingAccount[i];
    const bank = group.getBankByPk(balance.bankPk)!;

    const [collateralUsdValue, _] = balance.getUsdValue(
      bank,
      MarginRequirementType.Equity
    );
    if (collateralUsdValue.gt(maxCollateralUsd)) {
      maxCollateralUsd = collateralUsdValue;
      bestCollateralIndex = i;
    }
  }

  // This conversion is ignoring the liquidator discount, but the amounts still in legal bounds, as the liability paydown
  // is discounted meaning, the liquidation wont fail because of a too big paydown.
  const collateralToLiquidateUsdValue = BigNumber.min(
    maxCollateralUsd,
    maxLiabilityPaydownUsdValue
  );

  const collateralBankPk =
    marginfiAccount.lendingAccount[bestCollateralIndex].bankPk;
  const collateralBank = group.getBankByPk(collateralBankPk)!;
  const collateralQuantity = collateralBank.getQuantityFromUsdValue(
    collateralToLiquidateUsdValue,
    PriceBias.None
  );

  const liabBankPk = marginfiAccount.lendingAccount[bestCollateralIndex].bankPk;
  const liabBank = group.getBankByPk(liabBankPk)!;

  console.log(
    "Liquidating %d %s for %s",
    collateralQuantity,
    collateralBank.label,
    liabBank.label
  );
  const sig = await liquidatorAccount.lendingAccountLiquidate(
    marginfiAccount,
    collateralBank,
    collateralQuantity,
    liabBank
  );
  console.log("Liquidation tx: %s", sig);

  // Sell any non usd collateral and pay down any liability
  // Withdraw any non-usd collateral
  const mintsWithdrawn = await withdrawNonUsdcCollateral(
    liquidatorAccount,
    group
  );
  // Sell to collateral to usd
  await sellCollateralToUsdc(mintsWithdrawn, jupiter);
  // Buy liability with usd
  await buyLiab(liabBank, liquidatorAccount, jupiter);
  // Deposit usd + liability to marginfi account
  await depositLiabAndUsdc(liquidatorAccount, liabBank, usdcBank);
}

async function buyLiab(
  liabBank: Bank,
  marginfiAccount: MarginfiAccount,
  jup: Jupiter
) {
  await marginfiAccount.reload();
  const liabBalance = marginfiAccount.lendingAccount.find(
    (balance) => balance.bankPk == liabBank.publicKey
  )!;
  const [_, liabUsdValue] = liabBalance.getUsdValue(
    liabBank,
    MarginRequirementType.Equity
  );

  const usdcAmount = uiToNative(liabUsdValue, USDC_DECIMALS);
  const routes = await jup.computeRoutes({
    inputMint: USCD_MINT,
    outputMint: liabBank.mint,
    amount: JSBI.BigInt(usdcAmount),
    slippageBps: 10,
  });

  const bestRoute = routes.routesInfos[0];
  const trade = await jup.exchange({ routeInfo: bestRoute });
  await trade.execute();
}

async function depositLiabAndUsdc(
  marginfiAccount: MarginfiAccount,
  liabBank: Bank,
  usdcBank: Bank
) {
  const liabAta = await associatedAddress({
    mint: liabBank.mint,
    owner: wallet.publicKey,
  });
  const liabBalanceNative = (await connection.getTokenAccountBalance(liabAta))
    .value.amount;

  await marginfiAccount.deposit(
    nativeToUi(liabBalanceNative, liabBank.mintDecimals),
    liabBank
  );

  const usdcAta = await associatedAddress({
    mint: usdcBank.mint,
    owner: wallet.publicKey,
  });
  const usdcBalanceNative = (await connection.getTokenAccountBalance(usdcAta))
    .value.amount;

  await marginfiAccount.deposit(
    nativeToUi(usdcBalanceNative, USDC_DECIMALS),
    usdcBank
  );
}

async function sellCollateralToUsdc(mints: PublicKey[], jupiter: Jupiter) {
  for (let i = 0; i < mints.length; i++) {
    console.log("Swapping %s to USDC", mints[i]);
    const tokenAccountAta = await associatedAddress({
      mint: mints[i],
      owner: wallet.publicKey,
    });
    const balance = (await connection.getTokenAccountBalance(tokenAccountAta))
      .value.amount;

    const routes = await jupiter.computeRoutes({
      inputMint: mints[i],
      outputMint: USCD_MINT,
      amount: JSBI.BigInt(balance),
      slippageBps: 10,
    });

    const bestRoute = routes.routesInfos[0];

    const trade = await jupiter.exchange({ routeInfo: bestRoute });
    const res = await trade.execute();
    console.log("Tx signature: %s", res);
  }
}

async function withdrawNonUsdcCollateral(
  marginfiAccount: MarginfiAccount,
  group: MarginfiGroup
): Promise<PublicKey[]> {
  const mintsWithdrawn = [];
  for (let i = 0; i < marginfiAccount.lendingAccount.length; i++) {
    const balance = marginfiAccount.lendingAccount[i];
    const bank = group.getBankByPk(balance.bankPk)!;

    if (bank.mint.equals(USCD_MINT)) {
      continue;
    }

    const [collateralQuantity, _] = balance.getQuantity(bank);

    if (collateralQuantity.lte(1)) {
      continue;
    }

    const sig = await marginfiAccount.withdraw(collateralQuantity, bank);
    console.log("Withdraw tx: %s", sig);
    mintsWithdrawn.push(bank.mint);
  }

  return mintsWithdrawn;
}

async function main() {
  const config = await getConfig("devnet1");
  const client = await MarginfiClient.fetch(config, wallet, connection);
  const group = await MarginfiGroup.fetch(config, client.program);
  const liquidatorAccount = await MarginfiAccount.fetch(LIQUIDATOR_PK, client);
  const jupiter = await Jupiter.load({
    connection,
    cluster: "mainnet-beta",
    user: wallet.payer,
  });
  const round = async () => {
    const addresses = await client.getAllMarginfiAccountAddresses();

    for (let i = 0; i < addresses.length; i++) {
      await processAccount(
        group,
        client,
        liquidatorAccount,
        addresses[i],
        jupiter
      );
    }

    setTimeout(round, 10_000);
  };

  round();
}

main();
