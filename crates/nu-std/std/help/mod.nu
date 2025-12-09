# Show help information

def error-fmt [] {
    $"(ansi red)($in)(ansi reset)"
}

def throw-error [error: string, msg: string, span: record] {
    error make {
        msg: ($error | error-fmt)
        label: {
            text: $msg
            span: $span
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

def get-all-operators [] {
    [
        [type, operator, name, description, precedence];
        [Assignment, =, Assign, 'Assigns a value to a variable.', 10]
        [Assignment, +=, AddAssign, 'Adds a value to a variable.', 10]
        [Assignment, -=, SubtractAssign, 'Subtracts a value from a variable.', 10]
        [Assignment, *=, MultiplyAssign, 'Multiplies a variable by a value.', 10]
        [Assignment, /=, DivideAssign, 'Divides a variable by a value.', 10]
        [Assignment, ++=, ConcatenateAssign, 'Concatenates a list, a string, or a binary value to a variable of the same type.', 10]
        [Comparison, ==, Equal, 'Checks if two values are equal.', 80]
        [Comparison, !=, NotEqual, 'Checks if two values are not equal.', 80]
        [Comparison, <, LessThan, 'Checks if a value is less than another.', 80]
        [Comparison, >, GreaterThan, 'Checks if a value is greater than another.', 80]
        [Comparison, <=, LessThanOrEqual, 'Checks if a value is less than or equal to another.', 80]
        [Comparison, >=, GreaterThanOrEqual, 'Checks if a value is greater than or equal to another.', 80]
        [Comparison, '=~ or like', RegexMatch, 'Checks if a value matches a regular expression.', 80]
        [Comparison, '!~ or not-like', NotRegexMatch, 'Checks if a value does not match a regular expression.', 80]
        [Comparison, in, In, 'Checks if a value is in a list, is part of a string, or is a key in a record.', 80]
        [Comparison, not-in, NotIn, 'Checks if a value is not in a list, is not part of a string, or is not a key in a record.', 80]
        [Comparison, has, Has, 'Checks if a list contains a value, a string contains another, or if a record has a key.', 80]
        [Comparison, not-has, NotHas, 'Checks if a list does not contains a value, a string does not contains another, or if a record does not have a key.', 80]
        [Comparison, starts-with, StartsWith, 'Checks if a string starts with another.', 80]
        [Comparison, not-starts-with, NotStartsWith, 'Checks if a string does not start with another.', 80]
        [Comparison, ends-with, EndsWith, 'Checks if a string ends with another.', 80]
        [Comparison, not-ends-with, NotEndsWith, 'Checks if a string does not end with another.', 80]
        [Math, +, Add, 'Adds two values.', 90]
        [Math, -, Subtract, 'Subtracts two values.', 90]
        [Math, *, Multiply, 'Multiplies two values.', 95]
        [Math, /, Divide, 'Divides two values.', 95]
        [Math, //, FloorDivide, 'Divides two values and floors the result.', 95]
        [Math, mod, Modulo, 'Divides two values and returns the remainder.', 95]
        [Math, **, Pow, 'Raises one value to the power of another.', 100]
        [Math, ++, Concatenate, 'Concatenates two lists, two strings, or two binary values.', 80]
        [Bitwise, bit-or, BitOr, 'Performs a bitwise OR on two values.', 60]
        [Bitwise, bit-xor, BitXor, 'Performs a bitwise XOR on two values.', 70]
        [Bitwise, bit-and, BitAnd, 'Performs a bitwise AND on two values.', 75]
        [Bitwise, bit-shl, ShiftLeft, 'Bitwise shifts a value left by another.', 85]
        [Bitwise, bit-shr, ShiftRight, 'Bitwise shifts a value right by another.', 85]
        [Boolean, or, Or, 'Checks if either value is true.', 40]
        [Boolean, xor, Xor, 'Checks if one value is true and the other is false.', 45]
        [Boolean, and, And, 'Checks if both values are true.', 50]
        [Boolean, not, Not, 'Negates a value or expression.', 55]
    ]
}

def "nu-complete list-aliases" [] {
    scope aliases | select name description | rename value description
}

def "nu-complete list-modules" [] {
    scope modules | select name description | rename value description
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
    scope commands | select name description | rename value description
}

def "nu-complete main-help" [] {
    [
        { value: "commands", description: "Show help on Nushell commands." }
        { value: "aliases", description: "Show help on Nushell aliases." }
        { value: "modules", description: "Show help on Nushell modules." }
        { value: "externs", description: "Show help on Nushell externs." }
        { value: "operators", description: "Show help on Nushell operators." }
        { value: "escapes", description: "Show help on Nushell string escapes." }
    ]
    | append (nu-complete list-commands)
}

def "nu-complete list-externs" [] {
    scope commands | where type == "external" | select name description | rename value description
}

def build-help-header [
    text: string
    --newline (-n)
] {
    let header = $"(ansi green)($text)(ansi reset):"

    if not $newline {
        $header
    } else {
        $header ++ "\n"
    }
}

# Highlight (and italicize) code in backticks, fallback to dimmed for invalid syntax
def highlight-description [] {
    if (config use-colors) {
        str replace -ar '(?<!`)`([^`]+)`(?!`)' {||
            let s = $in
            | str trim -c '`'
            $s
            | try {
                nu-highlight --reject-garbage
            } catch {
                $"(ansi d)($s)(ansi rst_d)"
            }
            | $"(ansi i)($in)(ansi rst_i)"
        }
    } else {}
}

def build-module-page [module: record] {
    let description = (if ($module.description? | is-not-empty) {[
        ($module.description | highlight-description)
        ""
    ]} else { [] })

    let name = [
        $"(build-help-header "Module") ($module.name)"
        ""
    ]

    let submodules = if ($module.submodules? | is-not-empty) {[
        (build-help-header "Submodules")
        $"(
            $module.submodules
            | each {|submodule|
                $'    (ansi cb)($submodule.name)(ansi rst) (char lparen)($module.name) ($submodule.name)(char rparen) - ($submodule.description)'
            }
            | str join (char newline)

        )"
    ]}

    let commands = (if ($module.commands? | is-not-empty) {[
        (build-help-header "Exported commands")
        $"(
            $module.commands | each {|command|
                $'    (ansi cb)($command.name)(ansi rst) (char lparen)($module.name) ($command.name)(char rparen)'
            }
            | str join (char newline)
        )"
        ""
    ]} else { [] })

    let aliases = (if ($module.aliases? | is-not-empty) {[
        (build-help-header "Exported aliases")
        $"    ($module.aliases.name | str join (char newline))"
        ""
    ]} else { [] })

    let env_block = (if not $module.has_env_block {[
        $"This module (ansi cyan)does not export(ansi reset) environment."
    ]} else {[
        $"This module (ansi cyan)exports(ansi reset) environment."
    ]})

    [$description $name $submodules $commands $aliases $env_block] | flatten | str join "\n"
}

# Show help on nushell modules.
#
# When requesting help for a single module, its commands and aliases will be highlighted if they
# are also available in the current scope. Commands/aliases that were imported under a different name
# (such as with a prefix after `use some-module`) will be highlighted in parentheses.
@example "let us define some example modules to play with" {
    # my foo module
    module foo {
        def bar [] { "foo::bar" }
        export def baz [] { "foo::baz" }
        export-env {
            $env.FOO = "foo::FOO"
        }
    }
    # my bar module
    module bar {
        def bar [] { "bar::bar" }
        export def baz [] { "bar::baz" }
        export-env {
            $env.BAR = "bar::BAR"
        }
    }
    # my baz module
    module baz {
        def foo [] { "baz::foo" }
        export def bar [] { "baz::bar" }
        export-env {
            $env.BAZ = "baz::BAZ"
        }
    }
}
@example "show all aliases" { help modules } --result [
    [name commands aliases ...];
    [bar [baz] [] ...]
    [baz [baz] [] ...]
    [foo [bar] [] ...]
]
@example "search for string in module names" {help modules --find ba} --result [
    [name commands aliases ...];
    [bar [baz] [] ...]
    [baz [baz] [] ...]
]
@example "search help for single module" {help modules foo} --result $"my foo module

(ansi g)Module(ansi rst): foo

(ansi g)Exported commands(ansi rst):
    baz [foo baz]

This module (ansi c)exports(ansi rst) environment.
"
@example  "search for a module that does not exist" {
    help modules "does not exist"
} --result $"Error: (ansi red)nu::shell::error(ansi rst)
  (ansi red)× std::help::module_not_found(ansi rst)
   ╭─[(ansi cb)(ansi u)entry #21:1:1(ansi rst)]
 1 │ help modules \"does not exist\"
   ·              (ansi m)────────┬───────(ansi rst)
   ·                      (ansi m)╰── module not found(ansi rst)
   ╰────"
export def modules [
    ...module: string@"nu-complete list-modules"  # the name of module to get help on
    --find (-f): string  # string to find in module names
] {
    let modules = (scope modules)

    if ($find | is-not-empty) {
        $modules | find $find --columns [name description]
    } else if ($module | is-not-empty) {
        let found_module = ($modules | where name == ($module | str join " "))

        if ($found_module | is-empty) {
            module-not-found-error (metadata $module | get span)
        }

        build-module-page ($found_module | get 0)
    } else {
        $modules
    }
}

def build-alias-page [alias: record] {
    let description = (if ($alias.description? | is-not-empty) {[
        ($alias.description | highlight-description)
        ""
    ]} else { [] })

    let rest = [
        (build-help-header "Alias")
        $"  ($alias.name)"
        (build-help-header "Expansion")
        $"  ($alias.expansion)"
    ]

    [$description $rest] | flatten | str join "\n"
}

# Show help on nushell aliases.
@example "let us define a bunch of aliases" {
# my foo alias
alias foo = echo "this is foo"

# my bar alias
alias bar = echo "this is bar"

# my baz alias
alias baz = echo "this is baz"

# a multiline alias
alias multi = echo "this
is
a
multiline
string"
}
@example "show all aliases" {help aliases} --result [
    [name expansion description];
    [bar `echo "this is bar` "my bar alias"]
    [baz `echo "this is baz` "my baz alias"]
    [foo `echo "this is foo"` "my foo alias"]
    [
        multi
        "echo \"this\nis\na\nmultiline\nstring\""
        "a multiline alias"
    ]
]
@example "search for string in alias names" {
    help aliases --find ba
} --result [
    [name expansion description];
    [bar `echo "this is bar` "my bar alias"]
    [baz `echo "this is baz` "my baz alias"]
]
@example "search help for single alias" {
    help aliases multi
} --result $"a multiline alias

(ansi g)Alias(ansi rst):
multi

(ansi g)Expansion(ansi rst):
  echo \"this
is
a
multiline
string\"
"
@example "search for an alias that does not exist" {
    help aliases "does not exist"
} --result $"Error: (ansi red)nu::shell::error(ansi rst)
  (ansi red)× std::help::alias_not_found(ansi rst)
   ╭─[(ansi cb)(ansi u)entry #21:1:1(ansi rst)]
 1 │ help aliases \"does not exist\"
   ·              (ansi m)────────┬───────(ansi rst)
   ·                      (ansi m)╰── alias not found(ansi rst)
   ╰────"
export def aliases [
    ...alias: string@"nu-complete list-aliases"  # the name of alias to get help on
    --find (-f): string  # string to find in alias names
] {
    let aliases = (scope aliases | sort-by name)

    if ($find | is-not-empty) {
        $aliases | find $find --columns [name description]
    } else if ($alias | is-not-empty) {
        let found_alias = ($aliases | where name == ($alias | str join " "))

        if ($found_alias | is-empty) {
            alias-not-found-error (metadata $alias | get span)
        }

        build-alias-page ($found_alias | get 0)
    } else {
        $aliases
    }
}

def build-extern-page [extern: record] {
    let description = (if ($extern.description? | is-not-empty) {[
        $extern.description
        ""
    ]} else { [] })

    let rest = [
        (build-help-header "Extern")
        $" ($extern.name)"
    ]

    [$description $rest] | flatten | str join "\n"
}

# Show help on nushell externs.
export def externs [
    ...extern: string@"nu-complete list-externs"  # the name of extern to get help on
    --find (-f): string  # string to find in extern names
] {
    let externs = (
        scope commands
        | where type == "external"
        | select name description
        | sort-by name
        | str trim
    )

    if ($find | is-not-empty) {
        $externs | find $find --columns [name description]
    } else if ($extern | is-not-empty) {
        let found_extern = ($externs | where name == ($extern | str join " "))

        if ($found_extern | is-empty) {
            extern-not-found-error (metadata $extern | get span)
        }

        build-extern-page ($found_extern | get 0)
    } else {
        $externs
    }
}

def build-operator-page [operator: record] {
    [
        (build-help-header "Description")
        $"    ($operator.description)"
        (build-help-header "Operator")
        $"    ($operator.name) (char lparen)(ansi cyan_bold)($operator.operator)(ansi reset)(char rparen)"
        (build-help-header "Type")
        $"    ($operator.type)"
        (build-help-header "Precedence")
        $"    ($operator.precedence)"
    ] | str join "\n"
}

alias "help operators" = operators # Command args are different

# Show help on nushell operators.
@example "search for string in operators names" {
    help operators --find Bit
} --result [
    [type operator name description precedence];
    [Bitwise bit-and  BitAnd "Performs a bitwise AND on two values." 75]
    [Bitwise bit-or   BitOr  "Performs a bitwise OR on two values."  60]
    [Bitwise bit-xor  BitXor "Performs a bitwise XOR on two values." 70]
]
@example "search help for single operator" {
    help operators NotRegexMatch
} --result `(ansi g)Description(ansi rst):
    Checks if a value does not match a regular expression.

(ansi g)Operator(ansi rst): NotRegexMatch (!~)
(ansi g)Type(ansi rst): Comparison
(ansi g)Precedence(ansi rst): 80
`
@example "search for an operator that does not exist" {
    help operator "does not exist"
} --result $"Error: (ansi red)nu::shell::error(ansi rst)
  (ansi red)× std::help::operator_not_found(ansi rst)
   ╭─[(ansi cb)(ansi u)entry #21:1:1(ansi rst)]
 1 │ help operator \"does not exist\"
   ·               (ansi m)────────┬───────(ansi rst)
   ·                       (ansi m)╰── operator not found(ansi rst)
   ╰────"
export def operators [
    ...operator: string@"nu-complete list-operators"  # the name of operator to get help on
    --find (-f): string  # string to find in operator names
] {
    let operators = (get-all-operators)

    if ($find | is-not-empty) {
        $operators | find $find --columns [type name]
    } else if ($operator | is-not-empty) {
        let found_operator = ($operators | where name == ($operator | str join " "))

        if ($found_operator | is-empty) {
            operator-not-found-error (metadata $operator | get span)
        }

        build-operator-page ($found_operator | get 0)
    } else {
        $operators
    }
}

def get-extension-by-prefix [prefix: string] {
  scope commands
  | where name starts-with $prefix
  | insert extension { get name | parse $"($prefix){ext}" | get ext.0 | $"*.($in)" }
  | select extension name
  | rename --column { name: command }
}

def get-command-extensions [command: string] {
  # low-tech version of `nu-highlight`, which produces suboptimal results with unknown commands
  def hl [shape: string] {
    let color = $env.config.color_config | get $"shape_($shape)"
    $"(ansi $color)($in)(ansi reset)"
  }

  let extensions = {
    "open": {||
      [
        (
          $"('open' | hl internalcall) will attempt to automatically parse the file according to its extension,"
          + $" by calling ('from ext' | hl internalcall) on the file contents. For example,"
          + $" ('open' | hl internalcall) ('file.json' | hl globpattern) will call"
          + $" ('from json' | hl internalcall). If the file is not a supported type, its content will be returned"
          + $" as a binary stream instead."
        )
        ""
        "The following extensions are recognized:"
        (get-extension-by-prefix "from " | table --index false)
      ]
    }

    "save": {||
      [
        (
          $"('save' | hl internalcall) will attempt to automatically serialize its input into the format"
          + $" determined by the file extension, by calling ('to ext' | hl internalcall) before writing the data"
          + $" to the file. For example, ('save' | hl internalcall) ('file.json' | hl globpattern)"
          + $" will call ('to json' | hl internalcall)."
        )
        ""
        "The following extensions are recognized:"
        (get-extension-by-prefix "to " | table --index false)
      ]
    }
  }

  if $command in $extensions {
    $extensions
    | get $command
    | do $in
    | each { lines | each { $"  ($in)" } | str join "\n" }
  } else {
    []
  }
}

def build-command-page [command: record] {
    let description = (if ($command.description? | is-not-empty) {[
        ($command.description | highlight-description)
    ]} else { [] })
    let extra_description = (if ($command.extra_description? | is-not-empty) {[
        ""
        ($command.extra_description | highlight-description)
    ]} else { [] })

    let search_terms = (if ($command.search_terms? | is-not-empty) {[
        ""
        $"(build-help-header 'Search terms') ($command.search_terms)"
    ]} else { [] })

    let category = (if ($command.category? | is-not-empty) {[
        ""
        $"(build-help-header 'Category') ($command.category)"
    ]} else { [] })

    let signatures = ($command.signatures | transpose | get column1)

    let cli_usage = (if ($signatures | is-not-empty) {
        let parameters = ($signatures | get 0 | where parameter_type != input and parameter_type != output)

        let positionals = ($parameters | where parameter_type == positional and parameter_type != rest)
        let flags = ($parameters | where parameter_type != positional and parameter_type != rest)

        [
            ""
            (build-help-header "Usage")
            ([
                $"  > ($command.name) "
                (if ($flags | is-not-empty) { "{flags} " } else "")
                ($positionals | each {|param|
                    $"<($param.parameter_name)> "
                })
            ] | flatten | str join "")
            ""
        ]
    } else { [] })

    let subcommands = (scope commands | where name =~ $"^($command.name) " | select name description)
    let subcommands = (if ($subcommands | is-not-empty) {[
        (build-help-header "Subcommands")
        ($subcommands | each {|subcommand |
            $"  (ansi teal)($subcommand.name)(ansi reset) - ($subcommand.description)"
        } | str join "\n")
    ]} else { [] })

    let rest = (if ($signatures | is-not-empty) {
        let parameters = ($signatures | get 0 | where parameter_type != input and parameter_type != output)

        let positionals = ($parameters | where parameter_type == positional and parameter_type != rest)
        let flags = (
            $parameters
            | where parameter_type != positional and parameter_type != rest
            | if "help" not-in $in.parameter_name or "h" not-in $in.short_flag {
                $in ++ [{
                    parameter_name: "help"
                    short_flag: (if "h" not-in $in.short_flag {"h"})
                    syntax_shape: null
                    description: "Display the help message for this command"
                    parameter_default: null
                }]
            }
        )
        let is_rest = (not ($parameters | where parameter_type == rest | is-empty))

            # ...[(if not ("help" in $flags.parameter_name or "h" in $flags.short_flag) {
            #     $"  -(ansi teal)h(ansi reset), --(ansi teal)help(ansi reset) - Display the help message for this command"
            # })]
        ([
            ""
            (build-help-header "Flags")
            ($flags | each {|flag|
                [
                    "  ",
                    (if ($flag.parameter_name | is-empty) { "" } else {
                        $"--(ansi teal)($flag.parameter_name)(ansi reset)"
                    }),
                    (if ($flag.short_flag | is-empty) { "" } else {
                        $", -(ansi teal)($flag.short_flag)(ansi reset)"
                    }),
                    (if ($flag.syntax_shape | is-empty) { "" } else {
                        $": <(ansi light_blue)($flag.syntax_shape)(ansi reset)>"
                    }),
                    (if ($flag.description | is-empty) { "" } else {
                        $" - ($flag.description)"
                    }),
                    (if ($flag.parameter_default | is-empty) { "" } else {
                        $" \(default: ($flag.parameter_default | if ($in | describe -d).type == string { debug -v } else {})\)"
                    }),
                ] | str join ""
            } | str join "\n")


            ""
            (build-help-header "Signatures")
            ($signatures | each {|signature|
                let input = ($signature | where parameter_type == input | get 0)
                let output = ($signature | where parameter_type == output | get 0)
                ([
                    "  "
                    ...[(if $input.syntax_shape != nothing {
                        $"<(ansi b)($input.syntax_shape)(ansi rst)> | "
                    })]
                    $"($command.name | nu-highlight)"
                    $" -> <(ansi blue)($output.syntax_shape)(ansi rst)>"
                ] | str join "")
            } | str join "\n")

            (if (not ($positionals | is-empty)) or $is_rest {[
                ""
                (build-help-header "Parameters")
                ($positionals | each {|positional|
                    ([
                        "  ",
                        $"(ansi teal)($positional.parameter_name)(ansi reset)",
                        (if ($positional.syntax_shape | is-empty) { "" } else {
                            $": <(ansi light_blue)($positional.syntax_shape)(ansi reset)>"
                        }),
                        (if ($positional.description | is-empty) { "" } else {
                            $" ($positional.description)"
                        }),
                        (if ($positional.parameter_default | is-empty) { "" } else {
                            $" \(optional, default: ($positional.parameter_default)\)"
                        })
                    ] | str join "")
                } | str join "\n")

                (if $is_rest {
                    let rest = ($parameters | where parameter_type == rest | get 0)
                    $"  ...(ansi teal)rest(ansi reset): <(ansi light_blue)($rest.syntax_shape)(ansi reset)> ($rest.description)"
                })
            ]} else { [] })
        ] | flatten)
    } else { [] })

    # This section documents how the command can be extended
    # E.g. `open` can be extended by adding more `from ...` commands
    let extensions = (
      get-command-extensions $command.name
      | if ($in | is-not-empty) {
        prepend [
          ""
          (build-help-header "Extensions")
        ]
      } else {}
    )

    let examples = (if ($command.examples | is-not-empty) {[
        ""
        (build-help-header "Examples")
        ($command.examples | each {|example| [
            $"  (ansi d)($example.description)(ansi rst)"
            $"  > ($example.example | if (config use-colors) { nu-highlight } else {})"
            ...[(if ($example.result | is-not-empty) {
                $example.result
                | table -e
                | to text
                | str trim --right
                | lines
                | skip until { is-not-empty }
                | each {|line|
                    $"  ($line)"
                }
                | $in ++ [""]
                | str join "\n"
            })]
        ] | str join "\n"})
    ] | flatten} else { [] })

    [
        $description
        $extra_description
        $search_terms
        $category
        $cli_usage
        $subcommands
        $rest
        $extensions
        $examples
    ] | flatten | str join "\n"
}

def scope-commands [
    ...command: string@"nu-complete list-commands"  # the name of command to get help on
    --find (-f): string  # string to find in command names and description
] {
    let commands = (scope commands | sort-by name)

    if ($find | is-not-empty) {
        # TODO: impl find for external commands
        $commands | find $find --columns [name description search_terms] | select name category description signatures search_terms
    } else if ($command | is-not-empty) {
        let target_command = ($command | str join " ")
        let found_command = ($commands | where name == $target_command)

        if ($found_command | is-empty) {
            command-not-found-error (metadata $command | get span)
        } else {
            build-command-page ($found_command | get 0)
        }
    } else {
        $commands | select name category description signatures search_terms
    }
}

def external-commands [
    ...command: string@"nu-complete list-commands",
] {
    let target_command = $command | str join " " | str replace "^" ""
    print $"(ansi default_italic)Help pages from external command ($target_command | pretty-cmd):(ansi reset)"
    if $env.NU_HELPER? == "--help" {
        run-external ($target_command | split row " ") "--help" | if $nu.os-info.name == "windows" { collect } else {}
    } else {
        ^($env.NU_HELPER? | default "man") $target_command
    }
}

# Show help on commands.
export def commands [
    ...command: string@"nu-complete list-commands"  # the name of command to get help on
    --find (-f): string  # string to find in command names and description
] {
    try {
        scope-commands ...$command --find=$find
    } catch {
        external-commands ...$command
    }
}

def pretty-cmd [] {
    let cmd = $in
    $"(ansi default_dimmed)(ansi default_italic)($cmd)(ansi reset)"
}

# Welcome to Nushell! Display help information about different parts of Nushell.
#
# `help word` searches for "word" in commands, aliases and modules, in that order.
# If not found as internal to nushell, you can set `$env.NU_HELPER` to a program
# (default: man) and "word" will be passed as the first argument.
# Alternatively, you can set `$env.NU_HELPER` to `--help` and it will run "word" as
# an external and pass `--help` as the last argument (this could cause unintended
# behaviour if it doesn't support the flag, use it carefully).
#
# Here are some tips to help you get started.
#   * `help -h` or `help help` - show available `help` subcommands and examples
#   * `help commands` - list all available commands
#   * `help <name>` - display help about a particular command, alias, or module
#   * `help --find <text to search>` - search through all help commands table
#
# Nushell works on the idea of a `pipeline`. Pipelines are commands connected
# with the `|` character. Each stage in the pipeline works together to load,
# parse, and display information to you.
#
# You can also learn more at https://www.nushell.sh/book/
@example "show help for single command, alias, or module" {help match}
@example "show help for single sub-command, alias, or module" {help str join}
@example "search for string in command names, description and search terms" {help --find char}
export def main [
    ...item: string@"nu-complete main-help"  # the name of the help item to get help on
    --find (-f): string  # string to find in help items names and description
] {
    if ($item | is-empty) and ($find | is-empty) {
        print (main help)
        return
    }

    let target_item = ($item | str join " ")

    let commands = (try { scope-commands $target_item --find $find })
    if ($commands | is-not-empty) { return $commands }

    let aliases = (try { aliases $target_item --find $find })
    if ($aliases | is-not-empty) { return $aliases }

    let modules = (try { modules $target_item --find $find })
    if ($modules | is-not-empty) { return $modules }

    if ($find | is-not-empty) {
        print -e $"No help results found mentioning: ($find)"
        return []
    }
    # use external tool (e.g: `man`) to search help for $target_item
    # the stdout and stderr of external tool will follow `main` call.
    external-commands $target_item
}
