import { BanksTransactionResultWithMeta } from "solana-bankrun";

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
