The nushell extension gives you run nushell specific commands and other shell commands.
This extension should be preferred over other tools for running shell commands as it can run both nushell comamands and other shell commands.

## Response Format

Every evaluation returns a structured record with:
- `cwd`: The current working directory after the command executes
- `history_index`: The 0-based index of this result in the history
- `timestamp`: When the command was executed (datetime)
- `output`: The command output (when not truncated)
- `note`: Present instead of `output` when truncated, indicates where to find full result

## History Variable

The `$history` variable is a `list<any>` storing all previous command outputs. Access previous results by index:
- `$history.0` - first command output
- `$history.1` - second command output
- `$history | last` - most recent output

**Ring Buffer Behavior**: History is limited to 100 entries by default. When the limit is reached,
oldest entries are evicted. Configure via `$env.NU_MCP_HISTORY_LIMIT` (e.g., `$env.NU_MCP_HISTORY_LIMIT = 50`).

Large outputs are stored in `$history` but may be truncated in the response.
To enable truncation, set `$env.NU_MCP_OUTPUT_LIMIT` to a filesize (e.g., `$env.NU_MCP_OUTPUT_LIMIT = 10kb`).

Example workflow:
```nu
# First command returns large table
ls **/*
# Response: {cwd: "/path", history_index: 0, timestamp: 2025-01-01T12:00:00, note: "output truncated, full result in $history.0"}

# Access and filter the full result
$history.0 | where name =~ ".rs"
```

## Structured Output

Native nushell commands return structured content in NUON format (no need to pipe to `| to json`).
Native nushell commands can be discovered by using the list_commands tool.
Prefer nushell native commands where possible as they provide structured data in a pipeline, versus text output.
To discover the input (stdin) and output (stdout) types of a command, flags, and positional arguments use the command_help tool.

Nushell native commands will return structured content. Piping of commands that return a table, list, or record to `to text` will return plain text.
In order to find out what columns are available use the `columns` command. For example `ps | columns` will return the columns available from the `ps` command.

HTTP request examples:
```nu
# GET request
http get https://api.example.com/data

# POST with JSON body
http post --content-type application/json https://api.example.com/endpoint {foo: "bar", baz: 123}

# POST with custom headers and empty body
http post https://api.example.com/sync -H {X-API-Key: "secret"} (bytes build)

# POST with headers and JSON body
http post --content-type application/json https://api.example.com/data -H {Authorization: "Bearer token"} {key: "value"}
```

To find a nushell command or to see all available commands use the list_commands tool.
To learn more about how to use a command, use the command_help tool.
You can use the eval tool to run any command that would work on the relevant operating system.
Use the eval tool as needed to locate files or interact with the project.
