use error.nu module-not-found-error

def "nu-complete list-modules" [] {
    $nu.scope.modules | select name usage | rename value description
}

def build-module-page [module: record] {
    let usage = (if not ($module.usage? | is-empty) {[
        $module.usage
        ""
    ]} else { [] })

    let name = [
        $"(build-help-header -n "Module") ($module.name)"
        ""
    ]

    let commands = (if not ($module.commands? | is-empty) {[
        (build-help-header -n "Exported commands")
        $"    (
            $module.commands | each {|command|
                $'($command) (char lparen)($module.name) ($command)(char rparen)'
            }
            | str join ', '
        )"
        ""
    ]} else { [] })

    let aliases = (if not ($module.aliases? | is-empty) {[
        (build-help-header -n "Exported aliases")
        $"    ($module.aliases | str join ', ')"
        ""
    ]} else { [] })

    let env_block = (if ($module.env_block? | is-empty) {[
        $"This module (ansi cyan)does not export(ansi reset) environment."
    ]} else {[
        $"This module (ansi cyan)exports(ansi reset) environment."
        (view source $module.env_block)
    ]})

    [$usage $name $commands $aliases $env_block] | flatten | str join "\n"
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
export def main [
    ...module: string@"nu-complete list-modules"  # the name of module to get help on
    --find (-f): string  # string to find in module names
] {
    let modules = $nu.scope.modules

    if not ($find | is-empty) {
        $modules | find $find --columns [name usage]
    } else if not ($module | is-empty) {
        let found_module = ($modules | where name == ($module | str join " "))

        if ($found_module | is-empty) {
            module-not-found-error (metadata $module | get span)
        }

        build-module-page ($found_module | get 0)
    } else {
        $modules
    }
}
