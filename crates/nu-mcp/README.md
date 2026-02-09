# nu-mcp Crate

This crate provides support for MCP (Model Context Protocol) in Nushell.

## Feature Flag

The `mcp` feature flag controls whether MCP functionality is compiled into Nushell.

## Building with MCP Support

To build Nushell with MCP support, use:

```bash
cargo build --features mcp
```

## Running MCP Server

To run Nushell with the MCP server enabled:

```bash
nu --mcp
```

### Transport Options

The MCP server supports two transport modes:

- `stdio` (default): Standard I/O transport for local usage.
- `http`: Streamable HTTP with SSE for remote access.

To use HTTP transport:

```bash
nu --mcp --mcp-transport http
```

To specify a custom port (default is 8080):

```bash
nu --mcp --mcp-transport http --mcp-port 3000
```

If Nushell was built without the MCP feature and you attempt to use the `--mcp` flag, it will display an error message instructing you to recompile with the feature enabled.
