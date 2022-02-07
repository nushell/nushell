# alias

This command allows you to define shortcuts for other common commands. By default, they only apply to the current session. To persist them, add them to your config.

Syntax: `alias <name> = <body>`

The command expects two parameters:

* The name of the alias
* The body of the alias

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
