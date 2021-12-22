# Environment Variables (addition explaining the Values part)

(this is supposed to go to the Environment book chapter)

## Environment Variables Are Values

Since Nushell extends the idea of classic shells "Everything is a text" into "Everything is a structured data", it feels natural to apply this philosophy to environment variables as well.
In Nushell, environment variables can hold any value, not just a string.

Since the host environment (i.e., the OS Nushell is running in) treats environment variables as strings, we need a way to convert them between strings and arbitrary values seamlessly.
In general, there are two places when Nushell needs to interact with the host environment:
1. On startup, Nushell inherits the host environment variables.
2. When running an external program that is not part of Nushell's commands, the program expects environment variables to be strings.
3. When an environment variable is passed to an extenal library in the Nushell's codebase (this includes plugins as well). These variables are listed later in this section.

## Configuration

By default, if you do not configure anything, all environment variables are imported as strings on startup and then directly passed to any external program we might be spawning.
However, you can configure selected any environment variable to be converted to/from any value:

```
# config.nu

let config = {
    ... other config ...
    env_conversions: {
        FOO: {
            from_string: {|s| $s | split row ':' }
            to_string: {|v| $v | str collect ':' }
        }
    }
}
```

The above snippet will configure Nushell to run the `from_string` block with the `FOO` environment variable value as an argument on startup.
Whenever we run some external tool, the `to_string` block will be called with `FOO` as the argument and the result passed to the tool.
You can test the conversions by manually calling them:

```
> let-env FOO = "a:b:c"

> let list = (do $config.env_conversions.from_string $nu.env.FOO)

> $list
╭───┬───╮
│ 0 │ a │
│ 1 │ b │
│ 2 │ c │
╰───┴───╯

> do $config.env_conversions.to_string $list
a:b:c
```

To verify the conversion works on startup, you can first set up `FOO`, then launch a new instance of Nushell (denoted as `>>`):
```
> let-env FOO = "a:b:c"

> nu

>> $nu.env.FOO
╭───┬───╮
│ 0 │ a │
│ 1 │ b │
│ 2 │ c │
╰───┴───╯
```

To verify we're sending the correct value to an external tool, we would need to make a small program or script that prints its environment variables.
This is not hard, but we have a built-in command `env` to help.
Let's continue the previous session:

```
>> env
╭────┬───────────────────┬───────────────┬───────────────────┬───────────────────╮
│ #  │       name        │     type      │       value       │        raw        │
├────┼───────────────────┼───────────────┼───────────────────┼───────────────────┤
│  0 │ ...               │ ...           │ ...               │ ...               │
│  X │ FOO               │ list<unknown> │ [list 3 items]    │ a:b:c             │
│  Y │ ...               │ ...           │ ...               │ ...               │
╰────┴───────────────────┴───────────────┴───────────────────┴───────────────────╯
```

The `env` command will print every environment variable, its value and a type and also the translated value as a string under the `raw` column.
The `raw` values is the values external tools will see when spawned from Nushell.

## Special Variables

Out of the box, Nushell ships with several environment variables serving a special purpose:
* `PROMPT_COMMAND` (block): To set the prompt. Every time Nushell REPL enters a new line, it will run the block stored as its value and set the result as the prompt.
* `PATH`/`Path`: Not yet used except passthrough to externals but is planned to support both its string and list forms.
* `LS_COLORS`: Sets up file coloring rules when running `ls` or `grid`. Supports `env_conversions` settings.


## Breaking Changes

* Setting environment variable to `$nothing` will no longer remove it -- it will be `$nothing`. Instead, you can use `hide $nu.env.FOO`.
* `$nu.env.PROMPT_COMMAND` is a block instead of a string containing the source of the command to run. You can put this into your `config.nu`, for example: `let-env PROMPT_COMMAND = { echo "foo" }`.

## Future Directions

* We might add default conversion of PATH/Path environment variables between a list and a string.
* We can make Nushell recognize both PATH and Path (and throw an error if they are both set and have different values?).
