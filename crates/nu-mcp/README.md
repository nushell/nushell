# nu-mcp Crate

This crate provides support for MCP (Model Context Protocol) in Nushell.

## Feature Flag

The `mcp` feature flag controls whether MCP functionality is compiled into Nushell. By default, this feature is not enabled.

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

If Nushell was built without the MCP feature and you attempt to use the `--mcp` flag, it will display an error message instructing you to recompile with the feature enabled.
