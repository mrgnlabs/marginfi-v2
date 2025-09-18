## Build Notes
- Can only build the program using `anchor build -p marginfi`

## Test Notes
- Can only run `anchor test --skip-build` when running tests
- It is not possible to run individual tests, tests must be run as whole using the `anchor test --skip-build` command. If needed use `2>&1 |` to pipe the output and grep for specific test related outputs

## TypeScript Error Checking
- To check TypeScript errors, use the MCP IDE diagnostics tool:
  - For a specific file: `mcp__ide__getDiagnostics` with `uri` parameter (e.g., `file:///root/projects/kamino-integration/tests/utils/types.ts`)
  - For all open files: `mcp__ide__getDiagnostics` without parameters
- Do NOT use `npx tsc --noEmit` as it's not configured correctly for this project