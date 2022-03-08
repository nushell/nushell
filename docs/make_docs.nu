let vers = (version).version

for command in ($nu.scope.commands | where is_custom == false && is_extern == false) {
    let top = $"---
title: ($command.command)
layout: command
version: ($vers)
---

($command.usage)

"
    let sig = ($command.signature | each { |param|
        if $param.parameter_type == "positional" {
            $"('(')($param.parameter_name)(')')"
        } else if $param.parameter_type == "switch" {
            $"--($param.parameter_name)"
        } else if $param.parameter_type == "named" {
            $"--($param.parameter_name)"
        } else if $param.parameter_type == "rest" {
            $"...($param.parameter_name)"
        }
    } | str collect " ")

    let signature = $"## Signature(char nl)(char nl)```> ($command.command) ($sig)```(char nl)(char nl)"

    let params = ($command.signature | each { |param|
        if $param.parameter_type == "positional" {
            $" -  `($param.parameter_name)`: ($param.description)"
        } else if $param.parameter_type == "switch" {
            $" -  `--($param.parameter_name)`: ($param.description)"
        } else if $param.parameter_type == "named" {
            $" -  `--($param.parameter_name) {($param.syntax_shape)}`: ($param.description)"
        } else if $param.parameter_type == "rest" {
            $" -  `...($param.parameter_name)`: ($param.description)"
        }
    } | str collect (char nl))

    let parameters = if ($command.signature | length) > 0 {
        $"## Parameters(char nl)(char nl)($params)(char nl)(char nl)"
    } else {
        ""
    }

    let examples = if ($command.examples | length) > 0 {
        let example_top = $"## Examples(char nl)(char nl)"

        let $examples = ($command.examples | each { |example|
$"($example.description)
```shell
> ($example.example)
```

"
        } | str collect)

        $example_top + $examples
    } else { "" }

    let doc = (
            ($top + $signature + $parameters + $examples) |
            lines |
            each {|it| ($it | str trim -r) } |
            str collect (char nl)
        )

    let safe_name = ($command.command | str find-replace '\?' '' | str find-replace ' ' '_')
    $doc | save --raw $"./docs/commands/($safe_name).md"
    $"./docs/commands/($safe_name).md"
} | length | $"($in) commands written"

