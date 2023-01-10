import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import BigNumber from "bignumber.js";

import {
  NodeWallet,
  getConfig,
  MarginfiClient,
  AccountType,
  loadKeypair,
  MarginfiGroup,
} from "@mrgnlabs/marginfi-client-v2";
import MarginfiAccount, {
  MarginRequirementType,
} from "@mrgnlabs/marginfi-client-v2/src/account";
import { PriceBias } from "@mrgnlabs/marginfi-client-v2/src/bank";

const LIQUIDATOR_PK = new PublicKey(process.env.LIQUIDATOR_PK);
const connection = new Connection(process.env.RPC_ENDPOINT, "confirmed");
const wallet = new NodeWallet(loadKeypair(process.env.KEYPAIR_PATH));
const MARGINFI_GROUP_PK = new PublicKey(process.env.MARGINFI_GROUP_PK);

async function processAccount(
  group: MarginfiGroup,
  client: MarginfiClient,
  liquidatorAccount: MarginfiAccount,
  marginfiAccountAddress: PublicKey
) {
  const marginfiAccount = await MarginfiAccount.fetch(
    marginfiAccountAddress,
    client
  );
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
    const liquidateeLiabUsdValue = bank.getUsdValue(
      bank.getLiabilityValue(balance.liabilityShares),
      PriceBias.None
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
}
async function main() {
  const config = await getConfig("devnet1");
  const client = await MarginfiClient.fetch(config, wallet, connection);
  const group = await MarginfiGroup.fetch(config, client.program);
  const liquidatorAccount = await MarginfiAccount.fetch(LIQUIDATOR_PK, client);

  const round = async () => {
    const addresses = await client.getAllMarginfiAccountAddresses();

    for (let i = 0; i < addresses.length; i++) {
      await processAccount(group, client, liquidatorAccount, addresses[i]);
    }

    setTimeout(round, 10_000);
  };

  round();
}

main();
