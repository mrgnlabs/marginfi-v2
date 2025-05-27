import { Program } from "@coral-xyz/anchor";
import { MarginfiAccountRaw } from "@mrgnlabs/marginfi-client-v2";
import { wrappedI80F48toBigNumber } from "@mrgnlabs/mrgn-common";
import { PublicKey } from "@solana/web3.js";
import { BanksTransactionResultWithMeta } from "solana-bankrun";
import { Marginfi } from "target/types/marginfi";
import { bankrunProgram } from "tests/rootHooks";

/**
 * Function to print bytes from a Buffer in groups with column labels and color highlighting for non-zero values
 * @param buffer - The Buffer to process
 * @param groupLength - The number of bytes in each group, usually 8 or 16
 * @param totalLength - The total number of bytes to process
 * @param skipEmptyRows - If a row is all-zero, it will not print
 */
export const printBufferGroups = (
  buffer: Buffer,
  groupLength: number,
  totalLength: number,
  skipEmptyRows: boolean = true
) => {
  // Print the column headers
  let columnHeader = "        |";
  for (let col = 0; col < groupLength; col++) {
    if (col < groupLength - 1) {
      columnHeader += col.toString().padStart(3, " ").padEnd(6, " ");
    } else {
      // No end padding for the last column
      columnHeader += col.toString().padStart(3, " ");
    }
  }
  console.log(columnHeader);

  // Function to calculate RGB color based on row index
  const calculateGradientColor = (startIndex) => {
    const maxIndex = 255 * 3;
    const normalizedIndex = startIndex % maxIndex;

    let r = 0,
      g = 0,
      b = 0;

    if (normalizedIndex < 255) {
      b = 255;
      g = normalizedIndex;
    } else if (normalizedIndex < 510) {
      g = 255;
      b = 510 - normalizedIndex;
    } else {
      g = 765 - normalizedIndex;
      r = normalizedIndex - 510;
    }

    return `\x1b[38;2;${r};${g};${b}m`;
  };

  // Print the buffer content
  for (let i = 0; i < totalLength; i += groupLength) {
    let group = [];
    let allZero = true;

    for (let j = 0; j < groupLength; j++) {
      let value = buffer[i + j];
      let valueStr =
        value !== undefined ? value.toString().padStart(3, " ") : "   ";
      if (value !== 0) {
        allZero = false;
      }
      if (value !== 0 && value !== undefined) {
        // Apply red color to non-zero values
        group.push(`\x1b[31m${valueStr}\x1b[0m`);
      } else {
        group.push(valueStr);
      }
    }

    // Skip printing if the entire group is zero
    if (!allZero || !skipEmptyRows) {
      const color = calculateGradientColor(i);
      const label = `${i.toString().padStart(3, " ")}-${(i + groupLength - 1)
        .toString()
        .padStart(3, " ")}`;
      console.log(`${color}${label}\x1b[0m | ${group.join(" | ")}`);
    }
  }
};

export const dumpBankrunLogs = (result: BanksTransactionResultWithMeta) => {
  for (let i = 0; i < result.meta.logMessages.length; i++) {
    console.log(i + " " + result.meta.logMessages[i]);
  }
};

/**
 * Decode an f64 from 8 bytes (little‑endian).
 * @param bytes - either a Uint8Array or number[] of length 8
 * @returns the decoded number
 */
export function bytesToF64(bytes: Uint8Array | number[]): number {
  // Normalize to a Uint8Array
  const u8: Uint8Array =
    bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);

  if (u8.length !== 8) {
    throw new Error(`Invalid length ${u8.length}, expected exactly 8 bytes`);
  }

  // Create a DataView on the buffer (little‑endian)
  const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
  return dv.getFloat64(0, /* littleEndian */ true);
}

/**
 * Print account balances in a pretty table. If you're getting a type error here, due to a different
 * client version. feel free to ts-ignore it.
 */
export function dumpAccBalances(
  account: MarginfiAccountRaw 
) {
  let balances = account.lendingAccount.balances;
  let activeBalances = [];
  for (let i = 0; i < balances.length; i++) {
    if (balances[i].active == 0) {
      activeBalances.push({
        "Bank PK": "empty",
        Tag: "-",
        "Liab Shares ": "-",
        "Asset Shares": "-",
        // Emissions: "-",
      });
      continue;
    }

    activeBalances.push({
      "Bank PK": balances[i].bankPk.toString(),
      // Tag: balances[i].bankAssetTag,
      "Liab Shares ": formatNumber(
        wrappedI80F48toBigNumber(balances[i].liabilityShares)
      ),
      "Asset Shares": formatNumber(
        wrappedI80F48toBigNumber(balances[i].assetShares)
      ),
      // Emissions: formatNumber(
      //   wrappedI80F48toBigNumber(balances[i].emissionsOutstanding)
      // ),
    });

    function formatNumber(num) {
      const number = parseFloat(num).toFixed(4);
      return number === "0.0000" ? "-" : number;
    }
  }
  console.table(activeBalances);
}
