The nushell extension runs nushell and external shell commands. Prefer it over
any other shell tool — nushell's structured pipelines let you filter results
without re-running the command.

## THE RULE

**Never cap output inside a pipeline you are running for the first time.** No
`head`, `tail`, `first N`, `last N`, `take N`, `head -c`, `-n 5`, or any other
size-limiter in the pipe. This rule is unconditional. `where` does not license
capping. "I'm already filtering" does not license capping. "The output might be
huge" does not license capping.

Every evaluation's full result is captured in `$history` automatically. The
tool response you see may be truncated to fit the inline size limit, but
nothing is lost — you always get a `history_index` you can use to slice the
full result afterwards. You cannot flood context by running a large command.
Stop trying to help.

```nu
# BAD — all of these cap the live pipeline
ls **/*.rs | first 20
cargo build o+e>| tail -50
curl https://api.example.com/huge.json | head -c 500
rg foo crates/ | lines | where $it =~ "err" | first 30   # where is fine, first 30 is not

# GOOD — run once, slice afterwards
cargo build o+e>| complete
# response: { history_index: 7, ... }
$history.7 | lines | where $it =~ '^error'
$history.7 | lines | where $it =~ '^error' | skip 30 | first 30   # paging a saved result is fine
```

The only time you should cap inside the command is when **generation itself**
costs something real — a remote API that bills per byte, a paid model call,
etc. Local commands: just run them.

## Response Format

Every evaluation returns a NUON record:
- `cwd` — current working directory after the command
- `history_index` — 0-based index into `$history` for this result. **This is
  your handle for re-slicing later.** Read it out of the response.
- `timestamp` — datetime when executed
- `output` — the command output (when it fits under the limit)
- `note` — present **instead of** `output` when truncated; tells you the history index

When you see `note`, the response is truncated but `$history.<that_index>` has
the full, untruncated result.

## `$history` — the ring buffer

`$history: list<any>` stores every prior evaluation's output. **Use the stable
index from the response, not `| last`** — each new evaluation pushes its own
entry, so `$history | last` on a second call refers to itself, not to the
command you originally ran.

```nu
$history.7           # full result of the evaluation whose response had history_index: 7
$history.0           # first command's output
$history | length    # number of entries currently held

# `$history | last` is only correct *before* you run anything else. As a habit
# it teaches the wrong reflex. Always re-index by the explicit history_index.
```

Limits (configurable via env):
- `NU_MCP_HISTORY_LIMIT` — max entries, default `100`. When exceeded the oldest
  entry is evicted (ring-buffer semantics — indices do not shift, older ones
  simply become unavailable).
- `NU_MCP_OUTPUT_LIMIT` — response truncation threshold, default `10kb`. Set to
  `0b` to disable truncation entirely. The full value is always in `$history`
  regardless of this setting.
- `NU_MCP_PROMOTE_AFTER` — how long a call may run before being auto-promoted to
  a background job, default `120sec`. Bump it before a known long-running
  command to keep it synchronous.

```nu
$env.NU_MCP_OUTPUT_LIMIT = 50kb     # bigger inline responses
$env.NU_MCP_OUTPUT_LIMIT = 0b       # never truncate
$env.NU_MCP_HISTORY_LIMIT = 200     # remember more entries
$env.NU_MCP_PROMOTE_AFTER = 10min   # don't promote this session's long builds
```

## Structured Output — prefer native commands

Native nushell commands return structured NUON (records/tables/lists) — do NOT
pipe them to `| to json`, they already are structured. Use `list_commands` to
discover commands and `command_help` to see flags / input / output types.

```nu
ps | columns                    # see what columns are available
ls | where size > 1mb | get name
sys cpu | length                # record has a length like any other
```

External commands return a `string`. Parse with `from json` / `from yaml` /
`from csv` / `lines` / `split column` as needed.

## String literals

| Form                | Example                | Escapes | Use case |
|---------------------|------------------------|---------|----------|
| Single-quoted       | `'hello'`              | none    | literal, Windows paths, SQL |
| Double-quoted       | `"a\nb"`               | `\n \t \" \\` | strings needing escapes |
| Raw                 | `r#'he said "hi"'#`    | none    | mixed quotes, multi-line |
| Bare word           | `hello`                | none    | command args (word chars only) |
| Backtick            | `` `my file.txt` ``    | none    | paths/globs with spaces |
| Interpolated        | `$"x=($var)"`          | per-quote | embedding variables |

Interpolation requires `$"..."` (or `$'...'`) **and** parentheses around the
expression:

```nu
let name = "world"
"hello $name"          # literal: "hello $name"   ← WRONG
$"hello ($name)"       # "hello world"            ← RIGHT
```

Flags with embedded variables — the **whole** flag must be one interpolated
string, or use `--key=value`:

```nu
# BAD
mysql -p $env.PASSWORD db       # becomes two separate args
# GOOD
mysql $"-p($env.PASSWORD)" db
mysql $"--password=($env.PASSWORD)" db
```

Use `char escape` / `char newline` / `char tab` for control characters — nushell
does **not** support `\uXXXX` escapes in strings. Strip ANSI color codes with
`ansi strip`, not regex replacement.

## Redirection — no `2>&1`

Nushell uses its own redirection syntax:

| Bash                       | Nushell                    |
|----------------------------|----------------------------|
| `cmd > file`               | `cmd o> file`              |
| `cmd >> file`              | `cmd o>> file`             |
| `cmd > /dev/null`          | `cmd \| ignore`            |
| `cmd 2>&1`                 | `cmd o+e>\| next_cmd`      |
| `cmd > /dev/null 2>&1`     | `cmd o+e>\| ignore`        |
| `cmd \| tee log \| other`  | `cmd \| tee { save log } \| other` |

## Bash → Nushell quick reference

| Bash                               | Nushell                                      |
|------------------------------------|----------------------------------------------|
| `mkdir -p path`                    | `mkdir path`                                 |
| `rm -rf path`                      | `rm -r path`                                 |
| `cat file`                         | `open --raw file`                            |
| `grep pat`                         | `where $it =~ pat` / `find pat`              |
| `sed 's/a/b/'`                     | `str replace a b`                            |
| `head -5` / `tail -5`              | `first 5` / `last 5`  *(only on a saved `$history.N` — never on the live pipeline)* |
| `for f in *.md; do ...; done`      | `ls *.md \| each { \|r\| ... }`              |
| `$(cmd)`                           | `(cmd)` in expressions, `...(cmd)` to splat  |
| `echo $PATH`                       | `$env.PATH` (Unix) / `$env.Path` (Windows)   |
| `echo $?`                          | `$env.LAST_EXIT_CODE`                        |
| `FOO=bar ./bin`                    | `FOO=bar ./bin`                              |
| `type foo`                         | `which foo`                                  |
| `cmd1 && cmd2`                     | `cmd1; cmd2`                                 |
| line continuation `\`              | wrap in `( ... )`                            |

## HTTP — already parsed

`http get|post|put|...` auto-parses JSON responses based on `Content-Type`. Do
NOT pipe to `from json`.

```nu
http get https://api.example.com/users | get 0.name    # just works
http get -H {Authorization: $"Bearer ($token)"} $url
http post -t application/json $url {key: "value"}
http post -H {X-API-Key: $key} $url (bytes build)      # empty body
http get --raw $url | from json                        # opt out of auto-parse
```

Note: `-t json` does **not** work — pass the full MIME type (`application/json`).

## Parallelism

`par-each` runs closures across threads; use it whenever order doesn't matter
and the work is non-trivial.

```nu
ls **/*.rs | par-each { |f| open $f.name | lines | length }
ls **/*.log | par-each --threads 8 { |f| $f | ... }
```

Use plain `each` when order must be preserved or side effects must be serial.

## Globs and file discovery

Prefer `glob` over `find` / `ls -r`: nushell's `ls **/*` traverses hidden
directories too, which blows up output. Use `command_help glob` for details.

## Polars (if loaded)

For parquet/jsonl/ndjson/csv/avro, `polars` is dramatically faster than native
nushell or external tools. Start with `plugin use polars`.

```nu
polars open data.parquet | polars select name status | polars save out.parquet
ps | polars into-df | polars collect
polars open x.parquet | polars into-nu            # back to nushell table
```

## Long-running commands and background jobs

Evaluations that run longer than the promote-after threshold (or are cancelled
by the client) are auto-promoted to background jobs. The full (non-truncated)
output is delivered to the main thread's mailbox on completion. See the
`evaluate` tool description for the default and how to override it.

```nu
# You'll see an error like:
# "Operation promoted to background job (id: 1). Use `job list` to see it and `job recv` to get the result."

job list                         # check if still running
job recv                         # blocks until the result arrives (FULL output)
job recv --timeout 60sec         # bounded wait
job kill 1                       # cancel
```

Promoted jobs bypass `$history` — their full output arrives via `job recv`, not
as a history entry.

For manual backgrounding:

```nu
job spawn { uvicorn main:app }
job spawn --tag web-server { ... }

# get a result back to the main thread (id 0)
job spawn { ls | job send 0 }; job recv
job spawn { some-cmd | job send 0 }; job recv --timeout 5sec
job spawn { ^nc -vz -w5 host 5432 o+e>| job send 0 }; job recv --timeout 10sec

# tagged messages — only received when you filter by the same tag
job spawn { "done" | job send 0 --tag 1 }
job recv --tag 1
```

Gotchas: there is no `job ls` (use `job list`). `job recv` reads only the
current job's mailbox and takes no id. `job send` always takes a target id;
the main thread is `0`.

## Other quick tips

- Use `detect columns` to structure columnar CLI output (e.g.
  `launchctl list | detect columns`, `df | detect columns`) instead of hand-rolled
  `parse` patterns.
- Variables and env changes persist across tool calls (REPL semantics). Set
  `let x = ...` or `$env.FOO = ...` in one call, read it in the next.
- External processes do **not** inherit `let`-bound variables — only
  environment variables.
- `use list_commands` to search and `command_help <name>` for signatures.
