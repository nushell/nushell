# new help
#
# todo: revert help commands param display to original +/-;
#       retain consolidated for --concise
# todo: shouldn't it be help command <string> rather than help command*s* <string>?  But it's also help commands <nothing>.  Do both?
# todo: missing '{command,module,alias,operator,extrn}-not-found error; currently returns empty list but no error
# todo: slow to format signature section!
# todo: extra blank lines between examples (commands)
# todo: `help module <any>` never works,  should be `help modules <any>`.  Same for commands, aliases, externs, operators
#       Both should work, but how do you splat a ...rest?
# doc: no error for `help module`, `help alaises` (sic) ... because we want `help <any>` to try to lookup something
# doc: now `help --find <thing>` that matches just 1 <thing> will return the (1 line) list, not expand the item.

# doc: --concise, not --verbose
# doc: match item is <regex>.  use .* not *
# doc: --find is a switch
# doc: (Flags section and Command Type
# doc: help -f <string> is unanchored match in multiple columns; 
#      help <string> is whole word match, only in name column
# doc: help aliases, help modules working
# doc: show-modules: display imported commands with (*) prefix (maybe color later)
# doc: only shows -h for custom cmds and for builtins that explicitly add it to signature
# doc: operatos info not searched for `help <operator>`, must use `help operators <operator>`
# fix: never use dark blue -- invisible on black screen

## themeing and formatting of various snippets

def variable_shape_color [] {
    ($env.config.color_config.shape_variable | default light_purple))
}

# <string> | colorize ...color => string
# colorize input, not forgetting to reset afterwards.
def colorize [...color_names: string] {
    $"($color_names | each {|c| (ansi $c)} | str join '')($in)(ansi reset)"
}

def fmt-error [] {
    ($in | colorize 'red')
}

def fmt-bold [] {
    ($in | colorize 'attr_bold')
}

def fmt-header [] {
    ($in | colorize green)
}

# format a placeholder token which is a data type: in <> and italicized
def fmt-token [ ] {
    ($"<($in)>" | fmt-type)
}

# format datatype name: italicized in blue
def fmt-type [ ] {
    ($in  | colorize attr_italic (variable_shape_color))
}

def pretty-cmd [] {
    ($in | colorize default_dimmed default_italic)
}

## whole line formatting helpers

# print section header and optional title line for the section
def print-help-header [
    text: string
    title?: string = "" # optional title
    --no_newline (-n)   # suppress the default leading NL
] {
    if not $no_newline {
        print ""
    }
    print $"($text | fmt-header) ($title)"
}

# print detail line under section (indented)
def print-help-detail [text: string, indent?:int = 2] {
    $text | lines | each {|l| print $"(' ' * $indent)($l)" }
}

# Print section header and content in one fell swoop.  Or skip it if empty.
def print-help-section [section:string,  # section name, skip printing if ''
                        body?:string = ""
] {
    if not ($body | is-empty) {
        if $section != "" {
            print-help-header $section
        } else {print ""}
        print-help-detail $body
    }
}

# Print section header and 1 line detail.  Detail is expected to be name and usage
def print-help-section-name [section:string, name:string, usage?:string = ""] {
    print-help-header $section
    mut l = ($name | fmt-bold)
    if not ($usage | is-empty) { 
        $l += $" - ($usage)"
    }
    print-help-detail $l 
}

# "command_type" based on signature bits (commands only)
def cmd_type [] {
    if $in.is_builtin {
        'builtin command'
    } else if $in.is_extern { # must test before custom
        'external command'
    } else if $in.is_custom {
        'custom command'
    } else if $in.is_plugin {
        'plugin'
    } else {'other command'}
}

## Errors

def throw-error [error: string, msg: string, span: record] {
    error make {
        msg: ($error | fmt-error)
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

## Completers

def get-all-operators [] { return ([
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
] | sort-by name)}

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

## views for individual items
## all depend on the scope record with 2 added fields: 
#   scope (section it came from)
#   kind (user-presentable description of item)

# show detail for one item in default view
def show-item [ $item: record    # item's scope record (name, signatures, ...)
] {
    
    match $item.scope {
        "aliases" => {
            show-alias $item
        }
        "commands" => {
            show-command $item
        }
        "modules" => {
            show-module $item
        }
    }
}


# show detail for one item in *concise* view
def show-item-concise [ item: record    # item's scope record (name, signatures, ...)
] {

    match $item.scope {
        "aliases" => {
            show-alias $item
        }
        "commands" => {
            show-command-concise $item
        }
        "modules" => {
            show-module $item
        }
    }
}

def show-module [module: record] {
    print-help-section-name "Module" $module.name $module.usage

    # categorize subcommands exported vs unexported
    let commands = $module.commands?
    if not ($commands | is-empty) {
        let all_commands = ($nu.scope.commands | select name)

        let sc = ($commands | each {|c|
            [ $c,
            #($all_commands | find --regex $"^($c)$|($module.name) ($c)$" | get name),
            ($all_commands | find --regex $"^($c)$|($module.name) ($c)$" | get name | par-each {|n| '(*)' + $n}),
            ] | flatten | uniq | str join ", "
        } | grid)
        print-help-header "Available commands" "(*) - currently imported"
        print-help-detail $sc
    } ##
    
    print-help-section "Exported aliases" $module.aliases?

    print-help-header "Exported environment"

    if ($module.env_block? | is-empty) {
        print-help-detail ('$nothing' | colorize (variable_shape_color))
    } else {
        print-help-detail (view source $module.env_block | nu-highlight)
    }
    "" # return string
}

def show-alias [alias: record ] {

    print-help-section-name "Alias" $alias.name $alias.usage
    print-help-section "Expansion" $alias.expansion
}

def show-extern [extern: record] {
    print-help-section-name "Extern" $extern.name $extern.usage
}


def show-operator [operator: record] {
    print-help-section-name Operator $operator.operator $operator.name
    print-help-section Description $operator.description
    print-help-section Type $operator.type
    print-help-section Precedence ($operator.precedence | into string)
}

# show concise help for single command.
def show-command-concise [
        command: record
] {
    print-help-section "Usage" 
    print-help-detail $"($command | cmd_type) ($command.name | fmt-bold) - ($command.usage)"
    
    let indent = ((($command.signatures | values | first | where parameter_type == "input" | first |
                                get syntax_shape | str length -g ) + 3) * " ") # indent flags, params... under verb
    let tab_stop = 40 # column to start descriptions

    for sig in ($command.signatures | transpose | get column1) {
        print ""
        for i in $sig {
            let lhs = (match $i.parameter_type {
                "input" => { 
                    $"($i.syntax_shape | fmt-token) | ($command.name | colorize attr_bold)"
                }
                "switch" | "named" => {
                    $"($indent)-($i.short_flag), --($i.parameter_name)" + (if ($i.parameter_type == "named") {" " + ($i.syntax_shape | fmt-token)} else {""})
                },
                "positional" => { 
                    $"($indent)($i.parameter_name): ($i.syntax_shape | fmt-type)"
                },
                "rest" => {
                    let rp = if ($i.parameter_name? | is-empty) or $i.parameter_name == '') {
                        'rest'
                     } else { 
                        $i.parameter_name
                    }
                    $"($indent)...($rp): ($i.syntax_shape | fmt-type)"
                 },
                "output" => { 
                    $"  => ($i.syntax_shape | fmt-token)"
                },
            }) 

            let rhs = ([
                " ",                # extra blank in case lhs overflows tab_stop
                $i.description,
                (if $i.is_optional and ($i.parameter_type == "positional" or $i.parameter_type == "rest") {$". Optional, default TBD"} else {""}),
            ] | str join "")
            
            print-help-detail (($lhs | fill --character " " --alignment left --width $tab_stop) + $rhs)
        }
    }

    print-help-section "" $command.extra_usage?

    let examples = ($command.examples | 
                        par-each {|r| ["", 
                                    $r.description,
                                    $"> ($r.example | nu-highlight)",
                                    ($r.result | str join "\n")
                                    ] | str join "\n"
                            } | str join "\n"
                        ) 
    print-help-section "Examples" $examples

    
    let subcommands = ($nu.scope.commands | where name =~ $"^($command.name) " | 
                            each {|r| $"($r.name | colorize teal) - ($r.usage)"} | str join "\n")
    print-help-section "Subcommands" $subcommands 
}

# show normal help for single command.
def show-command [
        command: record
] { 
    print-help-section-name "Usage" $command.name $command.usage
    
    let indent = (($command.signatures | values | first | where parameter_type == "input" | first |
                                get syntax_shape | str length -g ) + 3) * " "  # indent flags, params... under verb
    mut tab_stop = 40 # column to start descriptions

    for sig in ($command.signatures | transpose | get column1) {
        print ""
        for i in $sig {
            let lhs = (match $i.parameter_type {
                "input" => { 
                    $"($i.syntax_shape | fmt-token) | ($command.name | colorize attr_bold)"
                }
                "switch" | "named" => {
                    $"($indent)-($i.short_flag), --($i.parameter_name)" + (if ($i.parameter_type == "named") {" " + ($i.syntax_shape | fmt-token)} else {""})
                },
                "positional" => { 
                    $"($indent)($i.parameter_name): ($i.syntax_shape | fmt-type)"
                },
                "rest" => {
                    let rp = if ($i.parameter_name? | is-empty) or $i.parameter_name == '') {
                        'rest'
                     } else { 
                        $i.parameter_name
                    }
                    $"($indent)...($rp): ($i.syntax_shape | fmt-type)"
                 },
                "output" => { 
                    $"  => ($i.syntax_shape | fmt-token)"
                },
            }) 

            let rhs = ([
                " ",                # extra blank in case lhs overflows tab_stop
                $i.description,
                (if $i.is_optional and ($i.parameter_type == "positional" or $i.parameter_type == "rest") {$". Optional, default TBD"} else {""}),
            ] | str join "")
            
            print-help-detail (($lhs | fill --character " " --alignment left --width $tab_stop) + $rhs)
        }
    }

    print-help-section "" $command.extra_usage?

    let examples = ($command.examples | 
                        par-each {|r| ["", 
                                    ($r.description | str join "\n"),
                                    $"> ($r.example | str join "\n" | nu-highlight)",
                                    ($r.result | str join "\n")
                                    ] | str join "\n" 
                            } | str join "\n" 
                        ) 
    print-help-section "Examples" $examples
    
    print-help-section "Search terms" $command.search_terms

    print-help-section Module $command.module_name

    print-help-section  Category $command.category

    print-help-header "Details"
    print-help-detail $"Command type:      ($command | cmd_type)"
    print-help-detail $"Creates scope:     ($command.creates_scope)"
    print-help-detail $"Is parser keyword: ($command.is_keyword)"
    
    let subcommands = ($nu.scope.commands | where name =~ $"^($command.name) " | 
                            par-each {|r| $"($r.name | colorize teal) - ($r.usage)"} | str join "\n")
    print-help-section "Subcommands" $subcommands 

}

## item lookups

# find matching items in aliases, modules and/or commands
def lookup-matches [
        item: string        # regex pattern to match on
        find:bool          # true -> search in more than just name column
        --only:string=""    # one of 'modules', 'aliases', 'commands' to limit results to that kind
] {
    # prime search with list of columns in signatures to include in --find search.
    mut match_cols = if $find {
            { aliases: [name usage expansion],
              commands: [name module_name category usage  search_terms],
                            # and not: signatures examples is_* creates_scope extra_usage
              modules: [name usage commands aliases env_block]
            }
        } else {
            {aliases: [name],
            commands: [name],
            modules: [name]
            }
        }
    
    # restrict scope members searched, if requested
    if $only != "" { 
        $match_cols = ($match_cols | select $only)
    }

    # help with no target and no --find should list all possibilities (even though -f not specified)
    let target = (
        if $find or ($item == "") {
            ""
        } else if (not $find) {
            $"^($item)$"
        } else {
            $item
        }
    )

    # find matching entries in all scope kinds
    let retval = ($match_cols | items {| k v | 
        ($nu.scope | get $k | find --ignore-case --columns $v --regex $target) |
        insert scope $k |
        insert kind {|sc| match $k {
                            'modules' => 'module',
                            'aliases' => 'alias',
                            'commands' => ($sc | cmd_type)
                        } 
                    } |
        } | flatten
    )
    return $retval
}
    

## entry points

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
    ...item: string@"nu-complete list-modules"  # the name of module to get help on
    --find (-f) # show list of matches
] {
    let $found_items = (lookup-matches ($item | str join " ") $find --only modules)

    if $find or ($found_items | length) != 1 {
        $found_items | select name usage kind
    } else {
        show-module $found_items.0
        "" # return string
    }
}

# Show help on nushell aliases.
#
# Examples:
#     > let us define a bunch of aliases
#     > ```nushell
#     > # my foo alias
#     > old-alias foo = echo "this is foo"
#     >
#     > # my bar alias
#     > old-alias bar = echo "this is bar"
#     >
#     > # my baz alias
#     > old-alias baz = echo "this is baz"
#     >
#     > # a multiline alias
#     > old-alias multi = echo "this
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
    ...item: string@"nu-complete list-aliases"  # the name of alias to get help on
    --find (-f)  # show list of matches
] {
    let $found_items = (lookup-matches ($item | str join " ") $find --only aliases)

    if $find or ($found_items | length) != 1 {
        $found_items | select name usage kind
    } else {
        show-alias $found_items.0
        "" # return string
    }
}

# Show help on nushell externs.
export def "help externs" [
    ...item: string@"nu-complete list-externs"  # the name of extern to get help on
    --find (-f)     # show list of matches
] {

    let $found_items = (lookup-matches ($item | str join " ") $find --only commands | where is_extern == true)

    if $find or ($found_items | length) != 1 {
        $found_items | select name usage kind
    } else {
        show-command $found_items.0
        "" # return string
    }
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
    ...item: string@"nu-complete list-operators"  # the name of operator to get help on
    --find (-f)       # show list of matches
] {
    let operators = (get-all-operators)

    mut operator = ($item | str join " ")

    if (not $find) and $operator != "" {
        $operator = $"^($operator)$"
    }

    let found_operators = ($operators | find --regex $operator)

    if $find or ($found_operators | length) != 1 {
        $found_operators
    } else {
        show-operator ($found_operators | get 0)
        "" # return string
    }
}

# Show help on nushell commands. 
export def "help commands" [
    ...item: string@"nu-complete list-commands"  # the name of command to get help on
    --find (-f)     # show list of matches
    --concise (-c) # Show more compact display
] {
    let $found_items = (lookup-matches ($item | str join " ") $find --only commands)

    if $find or ($found_items | length) != 1 {
        $found_items | select name usage kind
    } else {
        if $concise {
            show-command-concise $found_items.0
        } else {
            show-command $found_items.0
        }
        "" # return string
    }
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
    ...item: string # the name or path of the item to get help on.
    --find (-f)     # match not just on name but other info as well
    --concise (-c)  # Show less detail
] {
    if ($item | is-empty) and (not $find) {
        print $"Welcome to Nushell.

Here are some tips to help you get started.
  * ('help -h' | pretty-cmd) or ('help help' | pretty-cmd) - show available ('help' | pretty-cmd) subcommands and examples
  * ('help commands' | pretty-cmd) - list all available commands
  * ('help <name>' | pretty-cmd) - display help about a particular command, alias, or module
  * ('help --find <text to search>' | pretty-cmd) - search through all help commands table

Nushell works on the idea of a "('pipeline' | colorize default_italic)". Pipelines are commands connected with the '|' character.
Each stage in the pipeline works together to load, parse, and display information to you.

('Examples' | colorize green):
    List the files in the current directory, sorted by size
    > ('ls | sort-by size' | nu-highlight)

    Get information about the current system
    > ('sys | get host' | nu-highlight)

    Get the processes on your system actively using CPU
    > ('ps | where cpu > 0' | nu-highlight)

You can also learn more at ('https://www.nushell.sh/book/' | colorize default_italic light_cyan_underline)"
        return
    }

    let $found_items = (lookup-matches ($item | str join " ") $find)

    # show list of found items (possibly empty)
    if $find or ($found_items | length) != 1 {
        $found_items | select name usage kind
    } else {
        # show detail on individual item, if only one, in concise or default view
        if $concise {
            show-item-concise   $found_items.0
        } else {
            show-item           $found_items.0
        }
    }
}
