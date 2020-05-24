# alias
This command allows you to define shortcuts for other common commands. By default, they only apply to the current session. To persist them, add `--save`.

Syntax: `alias {flags} <name> [<parameters>] {<body>}`

The command expects three parameters:
* the name of alias
* the parameters as a space-separated list (`[a b ...]`), can be empty (`[]`)
* the body of the alias as a `{...}` block

## Flags

* `-s`, `--save`: Save the alias to your config (see `config --path` to edit them later)

## Examples

Define a custom `myecho` command as an alias:
```shell
> alias myecho [msg] { echo $msg }
> myecho "hello world"
hello world
```

Since the parameters are well defined, calling the command with the wrong number of parameters will fail properly:
```shell
> myecho hello world
error: myecho unexpected world
- shell:1:18
1 | myecho hello world
  |              ^^^^^ unexpected argument (try myecho -h)
```

The suggested help command works!
```shell
> myecho -h

Usage:
  > myecho ($msg) {flags}

parameters:
  ($msg)

flags:
  -h, --help: Display this help message
```

## Persistent aliases

Aliases are most useful when they are persistent. For that, add them to your startup config:
```
> config --set [startup ["alias myecho [msg] { echo $msg }"]]
```
This is fine for the first alias, but since it overwrites the startup config, you need a different approach for additional aliases.

To add a 2nd alias:
```
config --get startup | append "alias s [] { git status -sb }" | config --set_into startup
```
This first reads the `startup` config (a table of strings), then appends another alias, then sets the `startup` config with the output of the pipeline.

To make this process easier, you could define another alias:
```
> alias addalias [alias-string] { config --get startup | append $alias-string | config --set_into startup }
```
Then use that to add more aliases:
```
addalias "alias s [] { git status -sb }"
```
