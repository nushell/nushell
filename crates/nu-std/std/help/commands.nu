use error.nu command-not-found-error

def "nu-complete list-commands" [] {
    $nu.scope.commands | select name usage | rename value description
}

def build-command-page [command: record] {
    let usage = (if not ($command.usage? | is-empty) {[
        $command.usage
    ]} else { [] })
    let extra_usage = (if not ($command.extra_usage? | is-empty) {[
        ""
        $command.extra_usage
    ]} else { [] })

    let search_terms = (if not ($command.search_terms? | is-empty) {[
        ""
        $"(build-help-header -n 'Search terms') ($command.search_terms)"
    ]} else { [] })

    let module = (if not ($command.module_name? | is-empty) {[
        ""
        $"(build-help-header -n 'Module') ($command.module_name)"
    ]} else { [] })

    let category = (if not ($command.category? | is-empty) {[
        ""
        $"(build-help-header -n 'Category') ($command.category)"
    ]} else { [] })

    let this = ([
        ""
        "This command:"
    ] | append (
        if ($command.creates_scope) {
            $"- (ansi cyan)does create(ansi reset) a scope."
        } else {
            $"- (ansi cyan)does not create(ansi reset) a scope."
        }
    ) | append (
        if ($command.is_builtin) {
            $"- (ansi cyan)is(ansi reset) a built-in command."
        } else {
            $"- (ansi cyan)is not(ansi reset) a built-in command."
        }
    ) | append (
        if ($command.is_sub) {
            $"- (ansi cyan)is(ansi reset) a subcommand."
        } else {
            $"- (ansi cyan)is not(ansi reset) a subcommand."
        }
    ) | append (
        if ($command.is_plugin) {
            $"- (ansi cyan)is part(ansi reset) of a plugin."
        } else {
            $"- (ansi cyan)is not part(ansi reset) of a plugin."
        }
    ) | append (
        if ($command.is_custom) {
            $"- (ansi cyan)is(ansi reset) a custom command."
        } else {
            $"- (ansi cyan)is not(ansi reset) a custom command."
        }
    ) | append (
        if ($command.is_keyword) {
            $"- (ansi cyan)is(ansi reset) a keyword."
        } else {
            $"- (ansi cyan)is not(ansi reset) a keyword."
        }
    ))

    let signatures = ($command.signatures | transpose | get column1)

    let cli_usage = (if not ($signatures | is-empty) {
        let parameters = ($signatures | get 0 | where parameter_type != input and parameter_type != output)

        let positionals = ($parameters | where parameter_type == positional and parameter_type != rest)
        let flags = ($parameters | where parameter_type != positional and parameter_type != rest)

        [
            ""
            (build-help-header -n "Usage")
            ([
                $"  > ($command.name) "
                (if not ($flags | is-empty) { "{flags} " } else "")
                ($positionals | each {|param|
                    $"<($param.parameter_name)> "
                })
            ] | flatten | str join "")
            ""
        ]
    } else { [] })

    let subcommands = ($nu.scope.commands | where name =~ $"^($command.name) " | select name usage)
    let subcommands = (if not ($subcommands | is-empty) {[
        (build-help-header "Subcommands")
        ($subcommands | each {|subcommand |
            $"  (ansi teal)($subcommand.name)(ansi reset) - ($subcommand.usage)"
        } | str join "\n")
    ]} else { [] })

    let rest = (if not ($signatures | is-empty) {
        let parameters = ($signatures | get 0 | where parameter_type != input and parameter_type != output)

        let positionals = ($parameters | where parameter_type == positional and parameter_type != rest)
        let flags = ($parameters | where parameter_type != positional and parameter_type != rest)
        let is_rest = (not ($parameters | where parameter_type == rest | is-empty))

        ([
            ""
            (build-help-header "Flags")
            ($flags | each {|flag|
                [
                    "  ",
                    (if ($flag.short_flag | is-empty) { "" } else {
                        $"-(ansi teal)($flag.short_flag)(ansi reset), "
                    }),
                    (if ($flag.parameter_name | is-empty) { "" } else {
                        $"--(ansi teal)($flag.parameter_name)(ansi reset)"
                    }),
                    (if ($flag.syntax_shape | is-empty) { "" } else {
                        $": <(ansi light_blue)($flag.syntax_shape)(ansi reset)>"
                    }),
                    (if ($flag.description | is-empty) { "" } else {
                        $" - ($flag.description)"
                    }),
                    (if ($flag.parameter_default | is-empty) { "" } else {
                        $" \(default: ($flag.parameter_default)\)"
                    }),
                ] | str join ""
            } | str join "\n")
            $"  (ansi teal)-h(ansi reset), --(ansi teal)help(ansi reset) - Display the help message for this command"

            ""
            (build-help-header "Signatures")
            ($signatures | each {|signature|
                let input = ($signature | where parameter_type == input | get 0)
                let output = ($signature | where parameter_type == output | get 0)

                ([
                    $"  <($input.syntax_shape)> | ($command.name)"
                    ($positionals | each {|positional|
                        $" <($positional.syntax_shape)>"
                    })
                    $" -> <($output.syntax_shape)>"
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

    let examples = (if not ($command.examples | is-empty) {[
        ""
        (build-help-header -n "Examples")
        ($command.examples | each {|example| [
            $"  ($example.description)"
            $"  > ($example.example | nu-highlight)"
            (if not ($example.result | is-empty) {
                $example.result
                | table
                | if ($example.result | describe) == "binary" { str join } else { lines }
                | each {|line|
                    $"  ($line)"
                }
                | str join "\n"
            })
            ""
        ] | str join "\n"})
    ] | flatten} else { [] })

    [
        $usage
        $extra_usage
        $search_terms
        $module
        $category
        $this
        $cli_usage
        $subcommands
        $rest
        $examples
    ] | flatten | str join "\n"
}

# Show help on commands.
export def main [
    ...command: string@"nu-complete list-commands"  # the name of command to get help on
    --find (-f): string  # string to find in command names and usage
] {
    let commands = ($nu.scope.commands | where not is_extern | reject is_extern | sort-by name)

    if not ($find | is-empty) {
        # TODO: impl find for external commands
        $commands | find $find --columns [name usage search_terms] | select name category usage signatures search_terms
    } else if not ($command | is-empty) {
        let target_command = ($command | str join " ")
        let found_command = ($commands | where name == $target_command)

        if ($found_command | is-empty) {
            try {
                print $"(ansi default_italic)Help pages from external command ($target_command | pretty-cmd):(ansi reset)"
                ^($env.NU_HELPER? | default "man") $target_command
            } catch {
                command-not-found-error (metadata $command | get span)
            }
        }

        build-command-page ($found_command | get 0)
    } else {
        $commands | select name category usage signatures search_terms
    }
}
