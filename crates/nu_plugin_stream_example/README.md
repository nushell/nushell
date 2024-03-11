# Streaming Plugin Example

Crate with a simple example of the `StreamingPlugin` trait that needs to be implemented
in order to create a binary that can be registered into nushell declaration list

## `stream_example seq`

This command demonstrates generating list streams. It generates numbers from the first argument
to the second argument just like the builtin `seq` command does.

Examples:

> ```nushell
> stream_example seq 1 10
> ```

    [1 2 3 4 5 6 7 8 9 10]

> ```nushell
> stream_example seq 1 10 | describe
> ```

    list<int> (stream)

## `stream_example sum`

This command demonstrates consuming list streams. It consumes a stream of numbers and calculates the
sum just like the builtin `math sum` command does.

Examples:

> ```nushell
> seq 1 5 | stream_example sum
> ```

    15

## `stream_example collect-external`

This command demonstrates transforming streams into external streams. The list (or stream) of
strings on input will be concatenated into an external stream (raw input) on stdout.

> ```nushell
> [Hello "\n" world how are you] | stream_example collect-external
> ````

    Hello
    worldhowareyou

## `stream_example for-each`

This command demonstrates executing closures on values in streams. Each value received on the input
will be printed to the plugin's stderr. This works even with external commands.

> ```nushell
> ls | get name | stream_example for-each { |f| ^file $f }
> ```

    CODE_OF_CONDUCT.md: ASCII text

    CONTRIBUTING.md: ASCII text, with very long lines (303)

    ...
