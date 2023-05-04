def error-fmt [] {
    $"(ansi red)($in)(ansi reset)"
}

def throw-error [error: string, msg: string, span: record] {
    error make {
        msg: ($error | error-fmt)
        label: {
            text: $msg
            start: $span.start
            end: $span.end
        }
    }
}

def module-not-found-error [span: record] {
    throw-error "std::help::module_not_found" "module not found" $span
}

def alias-not-found-error [span: record] {
    throw-error "std::help::alias_not_found" "alias not found" $span
}

def extern-not-found-error [span: record] {
    throw-error "std::help::extern_not_found" "extern not found" $span
}

def operator-not-found-error [span: record] {
    throw-error "std::help::operator_not_found" "operator not found" $span
}

def command-not-found-error [span: record] {
    throw-error "std::help::command_not_found" "command not found" $span
}

def get-all-operators [] { return [
    [type, operator, name, description, precedence];

    [Assignment, =, Assign, "Assigns a value to a variable.", 10]
    [Assignment, +=, PlusAssign, "Adds a value to a variable.", 10]
    [Assignment, ++=, AppendAssign, "Appends a list or a value to a variable.", 10]
    [Assignment, -=, MinusAssign, "Subtracts a value from a variable.", 10]
    [Assignment, *=, MultiplyAssign, "Multiplies a variable by a value.", 10]
    [Assignment, /=, DivideAssign, "Divides a variable by a value.", 10]
    [Comparison, ==, Equal, "Checks if two values are equal.", 80]
    [Comparison, !=, NotEqual, "Checks if two values are not equal.", 80]
    [Comparison, <, LessThan, "Checks if a value is less than another.", 80]
    [Comparison, <=, LessThanOrEqual, "Checks if a value is less than or equal to another.", 80]
    [Comparison, >, GreaterThan, "Checks if a value is greater than another.", 80]
    [Comparison, >=, GreaterThanOrEqual, "Checks if a value is greater than or equal to another.", 80]
    [Comparison, =~, RegexMatch, "Checks if a value matches a regular expression.", 80]
    [Comparison, !~, NotRegexMatch, "Checks if a value does not match a regular expression.", 80]
    [Comparison, in, In, "Checks if a value is in a list or string.", 80]
    [Comparison, not-in, NotIn, "Checks if a value is not in a list or string.", 80]
    [Comparison, starts-with, StartsWith, "Checks if a string starts with another.", 80]
    [Comparison, ends-with, EndsWith, "Checks if a string ends with another.", 80]
    [Comparison, not, UnaryNot, "Negates a value or expression.", 0]
    [Math, +, Plus, "Adds two values.", 90]
    [Math, ++, Append, "Appends two lists or a list and a value.", 80]
    [Math, -, Minus, "Subtracts two values.", 90]
    [Math, *, Multiply, "Multiplies two values.", 95]
    [Math, /, Divide, "Divides two values.", 95]
    [Math, //, FloorDivision, "Divides two values and floors the result.", 95]
    [Math, mod, Modulo, "Divides two values and returns the remainder.", 95]
    [Math, **, "Pow ", "Raises one value to the power of another.", 100]
    [Bitwise, bit-or, BitOr, "Performs a bitwise OR on two values.", 60]
    [Bitwise, bit-xor, BitXor, "Performs a bitwise XOR on two values.", 70]
    [Bitwise, bit-and, BitAnd, "Performs a bitwise AND on two values.", 75]
    [Bitwise, bit-shl, ShiftLeft, "Shifts a value left by another.", 85]
    [Bitwise, bit-shr, ShiftRight, "Shifts a value right by another.", 85]
    [Boolean, and, And, "Checks if two values are true.", 50]
    [Boolean, or, Or, "Checks if either value is true.", 40]
    [Boolean, xor, Xor, "Checks if one value is true and the other is false.", 45]
]}

def "nu-complete list-aliases" [] {
    $nu.scope.aliases | select name usage | rename value description
}

def "nu-complete list-modules" [] {
    $nu.scope.modules | select name usage | rename value description
}

def "nu-complete list-operators" [] {
    let completions = (
        get-all-operators
        | select name description
        | rename value description
    )
    $completions
}

def "nu-complete list-commands" [] {
    $nu.scope.commands | select name usage | rename value description
}

def "nu-complete list-externs" [] {
    $nu.scope.commands | where is_extern | select name usage | rename value description
}

def print-help-header [
    text: string
    --no-newline (-n): bool
] {
    let header = $"(ansi green)($text)(ansi reset):"

    if $no_newline {
        print -n $header
    } else {
        print $header
    }
}

def show-module [module: record] {
    if not ($module.usage? | is-empty) {
        print $module.usage
        print ""
    }

    print-help-header -n "Module"
    print $" ($module.name)"
    print ""

    if not ($module.commands? | is-empty) {
        print-help-header "Exported commands"
        print -n "    "

        let commands_string = (
            $module.commands
            | each {|command|
                $"($command) (char lparen)($module.name) ($command)(char rparen)"
            }
            | str join ", "
        )

        print $commands_string
        print ""
    }

    if not ($module.aliases? | is-empty) {
        print-help-header -n "Exported aliases:"
        print $module.aliases
        print ""
    }

    if ($module.env_block? | is-empty) {
        print $"This module (ansi cyan)does not export(ansi reset) environment."
    } else {
        print $"This module (ansi cyan)exports(ansi reset) environment."
        view source $module.env_block
    }
}

# Show help on nushell modules.
#
# When requesting help for a single module, its commands and aliases will be highlighted if they
# are also available in the current scope. Commands/aliases that were imported under a different name
# (such as with a prefix after `use some-module`) will be highlighted in parentheses.
#
# Examples:
#     > let us define some example modules to play with
#     > ```nushell
#     > # my foo module
#     > module foo {
#     >     def bar [] { "foo::bar" }
#     >     export def baz [] { "foo::baz" }
#     >
#     >     export-env {
#     >         let-env FOO = "foo::FOO"
#     >     }
#     > }
#     >
#     > # my bar module
#     > module bar {
#     >     def bar [] { "bar::bar" }
#     >     export def baz [] { "bar::baz" }
#     >
#     >     export-env {
#     >         let-env BAR = "bar::BAR"
#     >     }
#     > }
#     >
#     > # my baz module
#     > module baz {
#     >     def foo [] { "baz::foo" }
#     >     export def bar [] { "baz::bar" }
#     >
#     >     export-env {
#     >         let-env BAZ = "baz::BAZ"
#     >     }
#     > }
#     > ```
#
#     show all aliases
#     > help modules
#     ╭───┬──────┬──────────┬────────────────┬──────────────┬───────────────╮
#     │ # │ name │ commands │    aliases     │  env_block   │     usage     │
#     ├───┼──────┼──────────┼────────────────┼──────────────┼───────────────┤
#     │ 0 │ bar  │ [baz]    │ [list 0 items] │ <Block 1331> │ my bar module │
#     │ 1 │ baz  │ [bar]    │ [list 0 items] │ <Block 1335> │ my baz module │
#     │ 2 │ foo  │ [baz]    │ [list 0 items] │ <Block 1327> │ my foo module │
#     ╰───┴──────┴──────────┴────────────────┴──────────────┴───────────────╯
#
#     search for string in module names
#     > help modules --find ba
#     ╭───┬──────┬─────────────┬────────────────┬──────────────┬───────────────╮
#     │ # │ name │  commands   │    aliases     │  env_block   │     usage     │
#     ├───┼──────┼─────────────┼────────────────┼──────────────┼───────────────┤
#     │ 0 │ bar  │ ╭───┬─────╮ │ [list 0 items] │ <Block 1331> │ my bar module │
#     │   │      │ │ 0 │ baz │ │                │              │               │
#     │   │      │ ╰───┴─────╯ │                │              │               │
#     │ 1 │ baz  │ ╭───┬─────╮ │ [list 0 items] │ <Block 1335> │ my baz module │
#     │   │      │ │ 0 │ bar │ │                │              │               │
#     │   │      │ ╰───┴─────╯ │                │              │               │
#     ╰───┴──────┴─────────────┴────────────────┴──────────────┴───────────────╯
#
#     search help for single module
#     > help modules foo
#     my foo module
#
#     Module: foo
#
#     Exported commands:
#         baz [foo baz]
#
#     This module exports environment.
#     {
#             let-env FOO = "foo::FOO"
#         }
#
#     search for a module that does not exist
#     > help modules "does not exist"
#     Error:
#       × std::help::module_not_found
#        ╭─[entry #21:1:1]
#      1 │ help modules "does not exist"
#        ·              ────────┬───────
#        ·                      ╰── module not found
#        ╰────
export def "help modules" [
    ...module: string@"nu-complete list-modules"  # the name of module to get help on
    --find (-f): string  # string to find in module names
] {
    let modules = $nu.scope.modules

    let module = ($module | str join " ")

    if not ($find | is-empty) {
        let found_modules = ($modules | find $find --columns [name usage])

        if ($found_modules | length) == 1 {
            show-module ($found_modules | get 0)
        } else {
            $found_modules
        }
    } else if not ($module | is-empty) {
        let found_module = ($modules | where name == $module)

        if ($found_module | is-empty) {
            module_not_found_error (metadata $module | get span)
        }

        show-module ($found_module | get 0)
    } else {
        $modules
    }
}

def show-alias [alias: record] {
    if not ($alias.usage? | is-empty) {
        print $alias.usage
        print ""
    }

    print-help-header -n "Alias"
    print $" ($alias.name)"
    print ""
    print-help-header "Expansion"
    print $"  ($alias.expansion)"
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
export def "help aliases" [
    ...alias: string@"nu-complete list-aliases"  # the name of alias to get help on
    --find (-f): string  # string to find in alias names
] {
    let aliases = ($nu.scope.aliases | sort-by name)

    let alias = ($alias | str join " ")

    if not ($find | is-empty) {
        let found_aliases = ($aliases | find $find --columns [name usage])

        if ($found_aliases | length) == 1 {
            show-alias ($found_aliases | get 0)
        } else {
            $found_aliases
        }
    } else if not ($alias | is-empty) {
        let found_alias = ($aliases | where name == $alias)

        if ($found_alias | is-empty) {
            alias-not-found-error (metadata $alias | get span)
        }

        show-alias ($found_alias | get 0)
    } else {
        $aliases
    }
}

def show-extern [extern: record] {
    if not ($extern.usage? | is-empty) {
        print $extern.usage
        print ""
    }

    print-help-header -n "Extern"
    print $" ($extern.name)"
}

# Show help on nushell externs.
export def "help externs" [
    ...extern: string@"nu-complete list-externs"  # the name of extern to get help on
    --find (-f): string  # string to find in extern names
] {
    let externs = (
        $nu.scope.commands
        | where is_extern
        | select name module_name usage
        | sort-by name
        | str trim
    )

    let extern = ($extern | str join " ")

    if not ($find | is-empty) {
        let found_externs = ($externs | find $find --columns [name usage])

        if ($found_externs | length) == 1 {
            show-extern ($found_externs | get 0)
        } else {
            $found_externs
        }
    } else if not ($extern | is-empty) {
        let found_extern = ($externs | where name == $extern)

        if ($found_extern | is-empty) {
            extern-not-found-error (metadata $extern | get span)
        }

        show-extern ($found_extern | get 0)
    } else {
        $externs
    }
}

def show-operator [operator: record] {
    print-help-header "Description"
    print $"    ($operator.description)"
    print ""
    print-help-header -n "Operator"
    print ($" ($operator.name) (char lparen)(ansi cyan_bold)($operator.operator)(ansi reset)(char rparen)")
    print-help-header -n "Type"
    print $" ($operator.type)"
    print-help-header -n "Precedence"
    print $" ($operator.precedence)"
}

# Show help on nushell operators.
#
# Examples:
#     search for string in operators names
#     > help operators --find Bit
#     ╭───┬─────────┬──────────┬────────┬───────────────────────────────────────┬────────────╮
#     │ # │  type   │ operator │  name  │              description              │ precedence │
#     ├───┼─────────┼──────────┼────────┼───────────────────────────────────────┼────────────┤
#     │ 0 │ Bitwise │ bit-and  │ BitAnd │ Performs a bitwise AND on two values. │         75 │
#     │ 1 │ Bitwise │ bit-or   │ BitOr  │ Performs a bitwise OR on two values.  │         60 │
#     │ 2 │ Bitwise │ bit-xor  │ BitXor │ Performs a bitwise XOR on two values. │         70 │
#     ╰───┴─────────┴──────────┴────────┴───────────────────────────────────────┴────────────╯
#
#     search help for single operator
#     > help operators NotRegexMatch
#     Description:
#         Checks if a value does not match a regular expression.
#
#     Operator: NotRegexMatch (!~)
#     Type: Comparison
#     Precedence: 80
#
#     search for an operator that does not exist
#     > help operator "does not exist"
#     Error:
#       × std::help::operator_not_found
#        ╭─[entry #21:1:1]
#      1 │ help operator "does not exist"
#        ·               ────────┬───────
#        ·                       ╰── operator not found
#        ╰────
export def "help operators" [
    ...operator: string@"nu-complete list-operators"  # the name of operator to get help on
    --find (-f): string  # string to find in operator names
] {
    let operators = (get-all-operators)

    let operator = ($operator | str join " ")

    if not ($find | is-empty) {
        let found_operators = ($operators | find $find --columns [type name])

        if ($found_operators | length) == 1 {
            show-operator ($found_operators | get 0)
        } else {
            $found_operators
        }
    } else if not ($operator | is-empty) {
        let found_operator = ($operators | where name == $operator)

        if ($found_operator | is-empty) {
            operator-not-found-error (metadata $operator | get span)
        }

        show-operator ($found_operator | get 0)
    } else {
        $operators
    }
}

def show-command [command: record] {
    if not ($command.usage? | is-empty) {
        print $command.usage
    }
    if not ($command.extra_usage? | is-empty) {
        print ""
        print $command.extra_usage
    }

    if not ($command.search_terms? | is-empty) {
        print ""
        print-help-header -n "Search terms"
        print $" ($command.search_terms)"
    }

    if not ($command.module_name? | is-empty) {
        print ""
        print-help-header -n "Module"
        print $" ($command.module_name)"
    }

    if not ($command.category? | is-empty) {
        print ""
        print-help-header -n "Category"
        print $" ($command.category)"
    }

    print ""
    print "This command:"
    if ($command.creates_scope) {
        print $"- (ansi cyan)does create(ansi reset) a scope."
    } else {
        print $"- (ansi cyan)does not create(ansi reset) a scope."
    }
    if ($command.is_builtin) {
        print $"- (ansi cyan)is(ansi reset) a built-in command."
    } else {
        print $"- (ansi cyan)is not(ansi reset) a built-in command."
    }
    if ($command.is_sub) {
        print $"- (ansi cyan)is(ansi reset) a subcommand."
    } else {
        print $"- (ansi cyan)is not(ansi reset) a subcommand."
    }
    if ($command.is_plugin) {
        print $"- (ansi cyan)is part(ansi reset) of a plugin."
    } else {
        print $"- (ansi cyan)is not part(ansi reset) of a plugin."
    }
    if ($command.is_custom) {
        print $"- (ansi cyan)is(ansi reset) a custom command."
    } else {
        print $"- (ansi cyan)is not(ansi reset) a custom command."
    }
    if ($command.is_keyword) {
        print $"- (ansi cyan)is(ansi reset) a keyword."
    } else {
        print $"- (ansi cyan)is not(ansi reset) a keyword."
    }

    let signatures = ($command.signatures | transpose | get column1)

    if not ($signatures | is-empty) {
        let parameters = ($signatures | get 0 | where parameter_type != input and parameter_type != output)

        let positionals = ($parameters | where parameter_type == positional and parameter_type != rest)
        let flags = ($parameters | where parameter_type != positional and parameter_type != rest)

        print ""
        print-help-header "Usage"
        print -n "  > "
        print -n $"($command.name) "
        if not ($flags | is-empty) {
            print -n $"{flags} "
        }
        for param in $positionals {
            print -n $"<($param.parameter_name)> "
        }
        print ""
    }

    let subcommands = ($nu.scope.commands | where name =~ $"^($command.name) " | select name usage)
    if not ($subcommands | is-empty) {
        print ""
        print-help-header "Subcommands"
        for subcommand in $subcommands {
            print $"  (ansi teal)($subcommand.name)(ansi reset) - ($subcommand.usage)"
        }
    }

    if not ($signatures | is-empty) {
        let parameters = ($signatures | get 0 | where parameter_type != input and parameter_type != output)

        let positionals = ($parameters | where parameter_type == positional and parameter_type != rest)
        let flags = ($parameters | where parameter_type != positional and parameter_type != rest)
        let is_rest = (not ($parameters | where parameter_type == rest | is-empty))

        print ""
        print-help-header "Flags"
        print $"  (ansi teal)-h(ansi reset), (ansi teal)--help(ansi reset) - Display the help message for this command"
        for flag in $flags {
            print -n $"  (ansi teal)-($flag.short_flag)(ansi reset), (ansi teal)--($flag.parameter_name)(ansi reset)"
            if not ($flag.syntax_shape | is-empty) {
                print -n $" <(ansi light_blue)($flag.syntax_shape)(ansi reset)>"
            }
            print $" - ($flag.description)"
        }

        print ""
        print-help-header "Signatures"
        for signature in $signatures {
           let input = ($signature | where parameter_type == input | get 0)
           let output = ($signature | where parameter_type == output | get 0)

           print -n $"  <($input.syntax_shape)> | ($command.name)"
           for positional in $positionals {
               print -n $" <($positional.syntax_shape)>"
           }
           print $" -> <($output.syntax_shape)>"
        }

        if (not ($positionals | is-empty)) or $is_rest {
            print ""
            print-help-header "Parameters"
            for positional in $positionals {
                print -n "  "
                if ($positional.is_optional) {
                    print -n "(optional) "
                }
                print $"(ansi teal)($positional.parameter_name)(ansi reset) <(ansi light_blue)($positional.syntax_shape)(ansi reset)>: ($positional.description)"
            }

            if $is_rest {
                let rest = ($parameters | where parameter_type == rest | get 0)
                print $"  ...(ansi teal)rest(ansi reset) <(ansi light_blue)($rest.syntax_shape)(ansi reset)>: ($rest.description)"
            }
        }
    }

    if not ($command.examples | is-empty) {
        print ""
        print-help-header -n "Examples"
        for example in $command.examples {
            print ""
            print $"  ($example.description)"
            print $"  > ($example.example | nu-highlight)"
            if not ($example.result | is-empty) {
                for line in (
                    $example.result | table | if ($example.result | describe) == "binary" { str join } else { lines }
                ) {
                    print $"  ($line)"
                }
            }
        }
    }

    print ""
}

# Show help on nushell commands.
export def "help commands" [
    ...command: string@"nu-complete list-commands"  # the name of command to get help on
    --find (-f): string  # string to find in command names and usage
] {
    let commands = ($nu.scope.commands | where not is_extern | reject is_extern | sort-by name)

    let command = ($command | str join " ")

    if not ($find | is-empty) {
        let found_commands = ($commands | find $find --columns [name usage search_terms])

        if ($found_commands | length) == 1 {
            show-command ($found_commands | get 0)
        } else {
            $found_commands | select name category usage signatures search_terms
        }
    } else if not ($command | is-empty) {
        let found_command = ($commands | where name == $command)

        if ($found_command | is-empty) {
            command-not-found-error (metadata $command | get span)
        }

        show-command ($found_command | get 0)
    } else {
        $commands | select name category usage signatures search_terms
    }
}

def pretty-cmd [] {
    let cmd = $in
    $"(ansi default_dimmed)(ansi default_italic)($cmd)(ansi reset)"
}

# Display help information about different parts of Nushell.
#
# `help word` searches for "word" in commands, aliases and modules, in that order.
#
# Examples:
#   show help for single command, alias, or module
#   > help match
#
#   show help for single sub-command, alias, or module
#   > help str lpad
#
#   search for string in command names, usage and search terms
#   > help --find char
export def main [
    ...item: string  # the name of the help item to get help on
    --find (-f): string  # string to find in help items names and usage
] {
    if ($item | is-empty) and ($find | is-empty) {
        print $"Welcome to Nushell.

Here are some tips to help you get started.
  * ('help -h' | pretty-cmd) or ('help help' | pretty-cmd) - show available ('help' | pretty-cmd) subcommands and examples
  * ('help commands' | pretty-cmd) - list all available commands
  * ('help <name>' | pretty-cmd) - display help about a particular command, alias, or module
  * ('help --find <text to search>' | pretty-cmd) - search through all help commands table

Nushell works on the idea of a "(ansi default_italic)pipeline(ansi reset)". Pipelines are commands connected with the '|' character.
Each stage in the pipeline works together to load, parse, and display information to you.

(ansi green)Examples(ansi reset):
    List the files in the current directory, sorted by size
    > ('ls | sort-by size' | nu-highlight)

    Get information about the current system
    > ('sys | get host' | nu-highlight)

    Get the processes on your system actively using CPU
    > ('ps | where cpu > 0' | nu-highlight)

You can also learn more at (ansi default_italic)(ansi light_cyan_underline)https://www.nushell.sh/book/(ansi reset)"
        return
    }

    let item = ($item | str join " ")

    let commands = (try { help commands $item --find $find })
    if not ($commands | is-empty) { return $commands }

    let aliases = (try { help aliases $item --find $find })
    if not ($aliases | is-empty) { return $aliases }

    let modules = (try { help modules $item --find $find })
    if not ($modules | is-empty) { return $modules }
}
