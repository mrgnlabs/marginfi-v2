/**
 * Function to print bytes from a Buffer in groups with column labels and color highlighting for
 * non-zero values.
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
  let columnHeader = "    |";
  for (let col = 0; col < groupLength; col++) {
    columnHeader += col.toString().padStart(3, " ") + " ";
  }
  console.log(columnHeader);
  console.log("----|" + "-".repeat(groupLength * 4 - 1));

  // Print the buffer content
  for (let i = 0; i < totalLength; i += groupLength) {
    let group = [];
    let allZero = true; // to skip printing if the entire row is zero

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

    if (!allZero && skipEmptyRows) {
      console.log(
        `${i.toString().padStart(3, " ")}-${(i + groupLength - 1)
          .toString()
          .padStart(3, " ")} | ${group.join(" | ")}`
      );
    }
  }
};
