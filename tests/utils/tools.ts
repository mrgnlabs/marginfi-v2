/**
 * Function to print bytes from a Buffer in groups with color highlighting for non-zero values
 * @param buffer - The Buffer to process
 * @param groupLength - The number of bytes in each group, usually 8
 * @param totalLength - The total number of bytes to process
 */
export const printBufferGroups = (
  buffer: Buffer,
  groupLength: number,
  totalLength: number
) => {
  for (let i = 0; i < totalLength; i += groupLength) {
    let group = [];
    for (let j = 0; j < groupLength; j++) {
      let value = buffer[i + j];
      if (value !== 0) {
        // Apply red color to non-zero values
        group.push(`\x1b[31m${value}\x1b[0m`);
      } else {
        group.push(value);
      }
    }
    console.log(`${i}-${i + groupLength - 1}: ${group.join(" ")}`);
  }
};
