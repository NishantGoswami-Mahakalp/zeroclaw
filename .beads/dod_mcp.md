## Why
MCP (Model Context Protocol) is becoming the de facto standard for AI tool protocols. Anthropic donated it to the Linux Foundation with 200+ implementations. ZeroClaw's trait architecture is ideal for this.

## What
- Implement MCP server to expose ZeroClaw tools/memory via MCP protocol
- Implement MCP client to consume external MCP tools  
- Register in provider infrastructure

## Strategic Fit
- High differentiation - unique among Rust frameworks
- Ecosystem lock-in opportunity
- Leverages existing trait architecture

## Definition of Done

### Technical Requirements
- MCP server implementation in src/mcp/server.rs
  - Implement Server struct with run method
  - Implement ServerHandlers trait for tool/resource handling
  - Support stdio and HTTP transport modes
  - Handle JSON-RPC 2.0 message parsing
- MCP client implementation in src/mcp/client.rs
  - Implement Client struct with connect/disconnect
  - Implement tool calling via MCP protocol
  - Implement resource reading via MCP protocol
  - Handle connection lifecycle and reconnection
- MCP types in src/mcp/types.rs
  - Implement MCP protocol structs (Initialize, Tools, Resources, etc.)
  - Implement JSON-RPC request/response types
  - Implement tool result conversion
- Integration with existing ZeroClaw
  - MCP tools wrapper in src/tools/mcp.rs
  - MCP resources expose ZeroClaw memory
  - Config schema extension for MCP settings

### Testing Requirements
- Unit tests for MCP message serialization/deserialization
- Integration tests for MCP server lifecycle
- Integration tests for MCP client tool calling
- E2E test with real MCP server (e.g., filesystem MCP)
- Test both stdio and HTTP transport modes
- Test error handling and reconnection scenarios

### Documentation Requirements
- Add MCP section to docs/config-reference.md
- Add MCP usage guide to docs/commands-reference.md
- Add provider/memory MCP integration to docs/providers-reference.md
- Update README.md with MCP capabilities

### Validation Requirements
- cargo fmt --all -- --check passes
- cargo clippy --all-targets -- -D clippy::correctness passes
- cargo test --locked passes
- Build succeeds with all features
- No new dependencies without justification
