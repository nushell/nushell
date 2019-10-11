# lines
This command takes a string from a pipeline as input, and returns a table where each line of the input string is a row in the table. Empty lines are ignored. This command is capable of feeding other commands, such as `nth`, with its output.

## Usage
```shell
> [input-command] | lines
```

## Examples 
Basic usage:
```shell
> printf "Hello\nWorld!\nLove, nushell." | lines
━━━┯━━━━━━━━━━━━━━━━
 # │ value 
───┼────────────────
 0 │ Hello 
 1 │ World! 
 2 │ Love, nushell. 
━━━┷━━━━━━━━━━━━━━━━
```

One useful application is piping the contents of file into `lines`. This example extracts a certain line from a given file.
```shell
> cat lines.md | lines | nth 6
## Examples
```

Similarly to this example, `lines` can be used to extract certain portions of or apply transformations to data returned by any program which returns a string.
