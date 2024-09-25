# Plugin Example

Crate with a simple example of the Plugin trait that needs to be implemented
in order to create a binary that can be registered into nushell declaration list

## `example config`

This subcommand demonstrates sending configuration from the nushell `$env.config` to a plugin.

To make use of the plugin after building `nushell` run:

```nushell
plugin add target/debug/nu_plugin_example
# or then either restart your current nushell session or run:
plugin use target/debug/nu_plugin_example
```

The configuration for the plugin lives in `$env.config.plugins.example`:

```nushell
$env.config = {
  plugins: {
    example: [
      some
      values
    ]
  }
}
```

To list plugin values run:

```nushell
example config
```

