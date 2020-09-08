# alias

This command allows you to define shortcuts for other common commands. By default, they only apply to the current session. To persist them, add `--save`.

Syntax: `alias {flags} <name> [<parameters>] {<body>}`

The command expects three parameters:

* The name of the alias
* The parameters as a space-separated list (`[a b ...]`), can be empty (`[]`)
* The body of the alias as a `{...}` block

## Flags

* `-s`, `--save`: Save the alias to your config (see `config path` to edit them later)

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

Aliases are most useful when they are persistent. For that, use the `--save` flag:

```shell
> alias --save myecho [msg] { echo $msg }
```

This will store the alias in your config, under the `startup` key. To edit the saved alias, run it again with the same name, or edit your config file directly. You can find the location of the file using `config path`.

For example, to edit your config file in `vi`, run:
```shell
> vi $(config path)
```

## Var args

It is possible to pass a variable amount of arguments to an alias by specifying a var arg as the last parameter in the alias definition
Example:
```shell
alias myecho [msg...] { echo $msg }
```
The var-arg variable can be used to substitute for a variable amount of arguments.
Currently var args are only allowed as positional arguments.
