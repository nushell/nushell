The nushell extension gives you run nushell specific commands and other shell commands.
This extension should be preferred over other tools for running shell commands as it can run both nushell commands and other shell commands.

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

**String interpolation:** Use `$"...(expr)..."` syntax. Variables/expressions must be in parentheses inside `$"..."` strings.
```nu
# BAD - regular strings don't interpolate variables
let name = "world"; echo "hello $name"   # Prints literal: hello $name

# GOOD - use $"..." with parentheses around expressions
let name = "world"; echo $"hello ($name)"       # Prints: hello world
ls $"($env.HOME)/Documents"                     # Expands path correctly
cargo build $"--jobs=(sys cpu | length)"        # Dynamic flag value
```

**Command-line flags with variables:** When building flags that include variable values, the entire flag must be an interpolated string.
```nu
# BAD - mixing bash-style and nushell syntax
mysql -p"$env.DATABASE_PASSWORD" mydb           # ERROR: $env not expanded
mysql -p $env.DATABASE_PASSWORD mydb            # ERROR: password becomes separate arg

# GOOD - entire flag as interpolated string
mysql $"-p($env.DATABASE_PASSWORD)" mydb        # Password correctly embedded
curl -H $"Authorization: Bearer ($token)"       # Header with variable

# GOOD - alternative: use flag=value syntax if supported
mysql $"--password=($env.DATABASE_PASSWORD)" mydb
```

**String types:** Nushell has several string formats. Inside any quoted string, `*` and other special characters are literal.

| Format | Syntax | Escapes | Use case |
|--------|--------|---------|----------|
| Single-quoted | `'hello'` | None | Simple strings, Windows paths |
| Double-quoted | `"hello\n"` | `\n \t \" \\` etc. | Strings needing escape sequences |
| Raw string | `r#'hello'#` | None | Strings with `'` or `"`, multi-line |
| Bare word | `hello` | None | Command arguments (word chars only) |
| Backtick | `` `hello world` `` | None | Paths/args with spaces, globs |
| Interpolated | `$"($var)"` | Depends on quotes | Embedding variables/expressions |

```nu
# Single-quoted: completely literal
'C:\path\to\file'                               # Backslashes literal
'SELECT * FROM users'                           # * is literal

# Double-quoted: C-style escapes
"Line one\nLine two"                            # \n = newline
"Say \"hello\""                                 # \" = literal quote

# Raw strings: literal, can contain single quotes
r#'It's a "test"'#                              # No escaping needed
r##'Contains r#'nested'#'##                     # Add more # to nest

# Bare words: unquoted, only "word" characters
print hello                                     # hello is a string
[foo bar baz]                                   # list of strings

# Backtick strings: bare words with spaces, useful for paths/globs
ls `./my directory`                             # Path with space
ls `**/*.rs`                                    # Glob pattern

# Interpolation: $"..." (with escapes) or $'...' (literal)
let name = "world"
$"Hello ($name)!"                               # => Hello world!
$'Path: ($env.HOME)'                            # Single-quoted interpolation
$"2 + 2 = (2 + 2)"                              # Expressions work too
```

**Prefer raw strings** (`r#'...'#`) for multi-line content or when mixing quote styles to avoid escaping.

**ANSI escape codes:** Use `ansi strip` to remove ANSI color/formatting codes from output. Do NOT use `\u001b` or similar unicode escapes - nushell doesn't support that syntax.
```nu
# BAD - nushell doesn't support \uXXXX unicode escapes
$output | str replace -a "\u001b" ""        # ERROR: invalid unicode escape

# GOOD - use ansi strip to remove ANSI codes
$output | ansi strip                         # Removes all ANSI escape sequences
^rg pattern | ansi strip                     # Strip colors from external command output

# To produce special characters, use the char command
char escape                                  # ESC character (0x1b)
char newline                                 # Newline
char tab                                     # Tab
```

**Stderr redirection:** Use `o+e>` or `out+err>` instead of bash-style `2>&1`.
```nu
# BAD - bash syntax doesn't work in nushell
command 2>&1                                    # ERROR: use 'out+err>' instead
command 2>/dev/null                             # ERROR: not valid nushell

# GOOD - nushell redirection syntax
command o+e>| other_command                     # Redirect stderr to stdout, pipe
command o+e>| ignore                            # Discard both stdout and stderr
```

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

**Parallel iteration:** Prefer `par-each` over `each` for better performance. `par-each` runs closures in parallel across multiple threads.
```nu
# BAD - sequential processing
ls **/*.rs | each { |f| wc -l $f.name }

# GOOD - parallel processing (much faster for I/O or CPU-bound work)
ls **/*.rs | par-each { |f| wc -l $f.name }

# GOOD - with thread count control
ls **/*.rs | par-each --threads 8 { |f| wc -l $f.name }
```

**When to use `each` instead:**
- Order must be preserved exactly (par-each returns results in completion order)
- Side effects must happen sequentially
- Very small lists where parallelization overhead exceeds benefit

## Background Jobs

Use `job spawn` to run commands in the background. This is the idiomatic nushell replacement for bash's `command &`.

```nu
# Spawn a background job (returns job ID immediately)
job spawn { sleep 5sec; echo "done" }

# Spawn with a descriptive tag
job spawn --tag "web-server" { uvicorn main:app }

# List all running background jobs
job list

# Kill a background job by ID
job kill 1
```

**Getting output from background jobs:** Use the mailbox system with `job send` and `job recv`.
`job recv` reads from the *current job's mailbox* only. It does not take a job ID.
The main thread always has job ID `0`, so background jobs should `job send 0`.

```nu
# Spawn job that sends result back to main thread
job spawn { ls | job send 0 }

# Wait and receive the result
job recv

# One-liner version
job spawn { ls | job send 0 }; job recv

# With timeout (to avoid blocking forever)
job spawn { some-command | job send 0 }; job recv --timeout 5sec

# Capture stderr too (external command)
job spawn { ^nc -vz -w 5 51.81.221.204 5432 o+e>| job send 0 }; job recv --timeout 10sec
```

**Inter-job communication and tags:** Jobs can send messages to each other using tags as filters.
Tags are integers. `job send --tag N` attaches a tag to the message.
`job recv --tag N` only receives messages with that exact tag.
Untagged messages are only received by `job recv` without a `--tag` filter.

```nu
# Send with a tag for filtering
job spawn { "result" | job send 0 --tag 1 }
job recv --tag 1    # Only receives messages with tag 1

# Get current job's ID from within a job
job spawn { let my_id = job id; ... }
```

**Job management commands:**
- `job spawn { ... }` - Start a background job, returns job ID
- `job list` - List all running jobs
- `job kill <id>` - Terminate a job
- `job send <id>` - Send data to a job's mailbox
- `job recv` - Receive data from mailbox (blocks until message arrives)
- `job id` - Get current job's ID
- `job tag <id> <tag>` - Add/change a job's description tag

**Common gotchas:**
- There is no `job ls`. Use `job list`.
- `job recv` does not accept a job id or `--id`. It only reads from the current job's mailbox.
 - `job send` always takes a target job id. The main thread id is `0`.
 - `job recv --tag N` will ignore untagged messages and messages with other tags.

To find a nushell command or to see all available commands use the list_commands tool.
To learn more about how to use a command, use the command_help tool.
You can use the eval tool to run any command that would work on the relevant operating system.
Use the eval tool as needed to locate files or interact with the project.
