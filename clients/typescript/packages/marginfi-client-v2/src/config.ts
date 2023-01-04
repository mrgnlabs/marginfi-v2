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
          new PublicKey("H7UazmGqtrVS8HH1TSouQdCr2o4aSgNQt6n2hJUdicz3"),
        groupPk:
          overrides?.groupPk ||
          new PublicKey("oA73UQvsdN1BiSavPSBupkgkhateeLd6g8NrU7JUxhr"),
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
