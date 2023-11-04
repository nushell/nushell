# Plugin configuration example

This demonstrates sending configuration from a nushell config to a plugin.

To register from after building `nushell` run:

```nushell
register target/debug/nu_plugin_config
```

The configuration for the plugin lives in `$env.config.plugins.config`:

```nushell
$env.config = {
  plugins: {
    config: [
      some
      values
    ]
  }
}
```

To list plugin values run:

```nushell
nu-plugin-config
```

Or:

```nushell
nu-plugin-config child
```
