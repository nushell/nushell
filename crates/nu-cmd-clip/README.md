# nu-cmd-clip

Built-in clipboard commands for Nushell.

## Commands

- `clip copy`: Copies pipeline input into the clipboard.
- `clip paste`: Reads current clipboard content.

`clip copy` serializes non-string values as JSON. `clip paste` tries to parse JSON by default and falls back to string output.
