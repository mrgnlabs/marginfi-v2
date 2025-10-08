## Build Notes

- Can only build the program or check if it's building using `anchor build -p marginfi -- --no-default-features`

## Test Notes

- When running tests for any purpose (checking if tests pass, debugging failures, etc.), use the @agent-test-runner-analyzer agent
- The test-runner-analyzer agent will handle running the full test suite and extracting relevant results for specific tests

## TypeScript Error Checking

- To check TypeScript errors, use the MCP IDE diagnostics tool:
  - For a specific file: `mcp__ide__getDiagnostics` with `uri` parameter (e.g., `file:///root/projects/kamino-integration/tests/utils/types.ts`)
  - For all open files: `mcp__ide__getDiagnostics` without parameters
- Do NOT use `npx tsc --noEmit` as it's not configured correctly for this project
