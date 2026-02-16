# nu-cmd-clip

Built-in clipboard commands for Nushell using arboard.

## Commands

- `clip copy`: Copies pipeline input into the clipboard.
- `clip paste`: Reads current clipboard content.

`clip copy` serializes non-string values as JSON. `clip paste` tries to parse JSON by default and falls back to string output.

## Features

- `use-wayland`: Enables `arboard/wayland-data-control` for more reliable Wayland clipboard behavior.

## Linux Daemon Mode

On Linux, `clip copy` uses daemon mode by default (a background thread keeps clipboard ownership alive).

You can disable daemon mode with config:

```nu
$env.config.plugins.clip.NO_DAEMON = true
```

Also supported for compatibility:

```nu
$env.config.plugins.clipboard.NO_DAEMON = true
$env.config.plugins.nu_plugin_clipboard.NO_DAEMON = true
```
