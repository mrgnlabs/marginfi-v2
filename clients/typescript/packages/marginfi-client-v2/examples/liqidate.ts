import { Connection, PublicKey } from "@solana/web3.js";
import BigNumber from "bignumber.js";
import { AccountType, getConfig, MarginfiClient, NodeWallet } from "../src";
import MarginfiAccount, { MarginRequirementType } from "../src/account";
import { PriceBias } from "../src/bank";

const LIQUIDATOR_PK = new PublicKey("");

async function main() {
  const connection = new Connection(
    "https://devnet.genesysgo.net/",
    "confirmed"
  );
  const wallet = NodeWallet.local();
  const config = await getConfig("devnet1");
  const client = await MarginfiClient.fetch(config, wallet, connection);

  const programAddresses = await client.getAllProgramAccountAddresses(
    AccountType.MarginfiGroup
  );
  console.log(programAddresses.map((key) => key.toBase58()));

  const liquidatorAccount = await MarginfiAccount.fetch(LIQUIDATOR_PK, client);
  const marginfiAccount = await MarginfiAccount.fetch(
    "6tgsmyfNHVzZaDJ6bjSVrKBGVsrgpqHNzr7WDz3BeT7t",
    client
  );

  const group = marginfiAccount.group;

  const bankLabel1 = "SOL";
  const bank1 = group.getBankByLabel(bankLabel1);
  if (!bank1) throw Error(`${bankLabel1} bank not found`);

  const bankLabel2 = "USDC";
  const bank2 = group.getBankByLabel(bankLabel2);
  if (!bank2) throw Error(`${bankLabel2} bank not found`);

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
      bank.getLiabilityQuantity(balance.liabilityShares),
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

main();
