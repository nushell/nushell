# alias

This command allows you to define shortcuts for other common commands. By default, they only apply to the current session. To persist them, add `--save`.

Syntax: `alias {flags} <name> = <body>`

The command expects two parameters:

* The name of the alias
* The body of the alias

## Flags

* `-s`, `--save`: Save the alias to your config (see `config path` to edit them later)

## Examples

Define a custom `myecho` command as an alias:

```shell
> alias myecho = echo
> myecho "hello world"
hello world
```

The suggested help command works!

```shell
> myecho -h

Usage:
  > myecho {flags}

flags:
  -h, --help: Display this help message
```

## Persistent aliases

Aliases are most useful when they are persistent. For that, use the `--save` flag:

```shell
> alias --save myecho = echo
```

This will store the alias in your config, under the `startup` key. To edit the saved alias, run it again with the same name, or edit your config file directly. You can find the location of the file using `config path`.

For example, to edit your config file in `vi`, run:
```shell
> vi $(config path)
```
