use error.nu alias-not-found-error
use common.nu build-help-header

def "nu-complete list-aliases" [] {
    $nu.scope.aliases | select name usage | rename value description
}

def build-alias-page [alias: record] {
    let usage = (if not ($alias.usage? | is-empty) {[
        $alias.usage
        ""
    ]} else { [] })

    let rest = [
        (build-help-header -n "Alias")
        $"  ($alias.name)"
        ""
        (build-help-header -n "Expansion")
        $"  ($alias.expansion)"
    ]

    [$usage $rest] | flatten | str join "\n"
}

# Show help on nushell aliases.
#
# Examples:
#     > let us define a bunch of aliases
#     > ```nushell
#     > # my foo alias
#     > alias foo = echo "this is foo"
#     >
#     > # my bar alias
#     > alias bar = echo "this is bar"
#     >
#     > # my baz alias
#     > alias baz = echo "this is baz"
#     >
#     > # a multiline alias
#     > alias multi = echo "this
#     > is
#     > a
#     > multiline
#     > string"
#     > ```
#
#     show all aliases
#     > help aliases
#     ╭───┬───────┬────────────────────┬───────────────────╮
#     │ # │ name  │     expansion      │       usage       │
#     ├───┼───────┼────────────────────┼───────────────────┤
#     │ 0 │ bar   │ echo "this is bar" │ my bar alias      │
#     │ 1 │ baz   │ echo "this is baz" │ my baz alias      │
#     │ 2 │ foo   │ echo "this is foo" │ my foo alias      │
#     │ 3 │ multi │ echo "this         │ a multiline alias │
#     │   │       │ is                 │                   │
#     │   │       │ a                  │                   │
#     │   │       │ multiline          │                   │
#     │   │       │ string"            │                   │
#     ╰───┴───────┴────────────────────┴───────────────────╯
#
#     search for string in alias names
#     > help aliases --find ba
#     ╭───┬──────┬────────────────────┬──────────────╮
#     │ # │ name │     expansion      │    usage     │
#     ├───┼──────┼────────────────────┼──────────────┤
#     │ 0 │ bar  │ echo "this is bar" │ my bar alias │
#     │ 1 │ baz  │ echo "this is baz" │ my baz alias │
#     ╰───┴──────┴────────────────────┴──────────────╯
#
#     search help for single alias
#     > help aliases multi
#     a multiline alias
#
#     Alias: multi
#
#     Expansion:
#       echo "this
#     is
#     a
#     multiline
#     string"
#
#     search for an alias that does not exist
#     > help aliases "does not exist"
#     Error:
#       × std::help::alias_not_found
#        ╭─[entry #21:1:1]
#      1 │ help aliases "does not exist"
#        ·              ────────┬───────
#        ·                      ╰── alias not found
#        ╰────
export def main [
    ...alias: string@"nu-complete list-aliases"  # the name of alias to get help on
    --find (-f): string  # string to find in alias names
] {
    let aliases = ($nu.scope.aliases | sort-by name)

    if not ($find | is-empty) {
        $aliases | find $find --columns [name usage]
    } else if not ($alias | is-empty) {
        let found_alias = ($aliases | where name == ($alias | str join " "))

        if ($found_alias | is-empty) {
            alias-not-found-error (metadata $alias | get span)
        }

        build-alias-page ($found_alias | get 0)
    } else {
        $aliases
    }
}
