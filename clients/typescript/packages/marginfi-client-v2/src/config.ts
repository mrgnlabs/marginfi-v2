import { PublicKey } from "@solana/web3.js";
import { Environment, MarginfiConfig, MarginfiDedicatedConfig } from "./types";

/**
 * Define marginfi-specific config per profile
 *
 * @internal
 */
function getMarginfiConfig(
  environment: Environment,
  overrides?: Partial<
    Omit<MarginfiDedicatedConfig, "environment" | "mango" | "zo">
  >
): MarginfiDedicatedConfig {
  switch (environment) {
    case Environment.MAINNET:
      return {
        environment,
        programId: overrides?.programId || PublicKey.default,
        groupPk: overrides?.groupPk || PublicKey.default,
      };
    case Environment.DEVNET:
      return {
        environment,
        programId:
          overrides?.programId ||
          new PublicKey("HfHBtENWH9C27kXMwP62WCSMm734kzKj9YnzUaHPzk6i"),
        groupPk:
          overrides?.groupPk ||
          new PublicKey("2XGMxsZkV6zyJ6kbdurqM19hXLvTPSXaAmr6MBTpGpzr"),
      };
    default:
      throw Error(`Unknown environment ${environment}`);
  }
}

/**
 * Retrieve config per environment
 */
export async function getConfig(
  environment: Environment,
  overrides?: Partial<Omit<MarginfiConfig, "environment">>
): Promise<MarginfiConfig> {
  return {
    ...getMarginfiConfig(environment, overrides),
  };
}
