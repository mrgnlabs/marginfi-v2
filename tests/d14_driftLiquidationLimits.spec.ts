import { assert } from "chai";
import { BN } from "@coral-xyz/anchor";
import {
  setupDriftLiqEnv,
  DriftLiqEnv,
  runLiquidationScenario,
  ScenarioResult,
  PASSING_DRIFT_COUNT,
} from "./utils/drift-liq-helpers";
import { verbose } from "./rootHooks";

describe("d14: Drift liquidation limits", () => {
  let env: DriftLiqEnv;

  before(async function () {
    this.timeout(300_000);

    // Create all 31 banks + LUT once
    env = await setupDriftLiqEnv();

    // Verify setup (PASSING_DRIFT_COUNT + 1 drift banks for the failing test)
    assert.equal(env.driftBanks.length, PASSING_DRIFT_COUNT + 1, `Should have ${PASSING_DRIFT_COUNT + 1} drift banks`);
    assert.equal(env.regularBanks.length, 15, "Should have 15 regular banks");
    assert.ok(env.liabBank, "Should have 1 liab bank");
    assert.ok(env.lutAddress, "Should have LUT address");

    if (verbose) {
      console.log("\n📊 Setup Complete:");
      console.log(`   Drift banks: ${env.driftBanks.length}`);
      console.log(`   Regular banks: ${env.regularBanks.length}`);
      console.log(`   Liab bank: ${env.liabBank}`);
      console.log(`   LUT: ${env.lutAddress}`);
    }
  });

  it("tests liquidation across drift/regular combinations", async function () {
    this.timeout(600_000); // 10 minutes for all scenarios

    const results: ScenarioResult[] = [];

    // Test combinations: driftCount from 1 to PASSING_DRIFT_COUNT, regularCount = 15 - driftCount
    // Total positions = driftCount + regularCount + 1 borrow = 16
    for (let driftCount = 1; driftCount <= PASSING_DRIFT_COUNT; driftCount++) {
      const regularCount = 15 - driftCount;

      if (verbose) {
        console.log(
          `\n🔄 Testing: ${driftCount} drift + ${regularCount} regular + 1 borrow`
        );
      }

      const result = await runLiquidationScenario(env, {
        driftCount,
        regularCount,
        scenarioIndex: driftCount - 1,
      });

      results.push(result);

      if (verbose) {
        const status = result.success
          ? "✅ SUCCESS"
          : `❌ FAILED (${result.errorType})`;
        console.log(`   Result: ${status}`);
        if (result.cuUsed) {
          console.log(`   CU Used: ${result.cuUsed}`);
        }
        if (result.errorMessage) {
          console.log(`   Error: ${result.errorMessage}`);
        }
      }
    }

    // Print summary table
    console.log("\n" + "=".repeat(80));
    console.log("📊 LIQUIDATION LIMITS SUMMARY");
    console.log("=".repeat(80));
    console.log(
      "Drift | Regular | Borrow | Total | Status     | CU Used    | Error"
    );
    console.log("-".repeat(80));

    for (const r of results) {
      const status = r.success ? "SUCCESS" : "FAILED";
      const cu = r.cuUsed ? r.cuUsed.toString().padStart(10) : "N/A".padStart(10);
      const error = r.errorType || "-";
      const total = r.driftCount + r.regularCount + 1;
      console.log(
        `${r.driftCount.toString().padStart(5)} | ` +
          `${r.regularCount.toString().padStart(7)} | ` +
          `${"1".padStart(6)} | ` +
          `${total.toString().padStart(5)} | ` +
          `${status.padEnd(10)} | ` +
          `${cu} | ` +
          `${error}`
      );
    }
    console.log("=".repeat(80));

    // Verify we ran all scenarios
    assert.equal(results.length, PASSING_DRIFT_COUNT, `Should have run ${PASSING_DRIFT_COUNT} scenarios`);
  });

  it("fails with too many drift positions due to active bank limit", async function () {
    this.timeout(120_000);

    const driftCount = PASSING_DRIFT_COUNT + 1;
    const regularCount = 15 - driftCount;

    if (verbose) {
      console.log(
        `\n🔄 Testing: ${driftCount} drift + ${regularCount} regular + 1 borrow (expecting failure)`
      );
    }

    const result = await runLiquidationScenario(env, {
      driftCount,
      regularCount,
      scenarioIndex: 99,
    });

    assert.equal(result.success, false, `${driftCount} drift positions should fail`);
    if (verbose) {
      console.log(`   Error type: ${result.errorType}`);
      console.log(`   Error message: ${result.errorMessage}`);
    }
  });
});
