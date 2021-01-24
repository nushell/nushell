# def

Use `def` to create a custom command.

## Examples

```shell
> def my_command [] { echo hi nu }
> my_command
hi nu
```

```shell
> def my_command [adjective: string, num: int] { echo $adjective $num meet nu }
> my_command nice 2
nice 2 meet nu
```

```shell
def my_cookie_daemon [
    in: path             # Specify where the cookie daemon shall look for cookies :p
    ...rest: path        # Other places to consider for cookie supplies
    --output (-o): path  # Where to store leftovers
    --verbose
] {
    echo $in $rest | each { eat $it }
    ...
}
my_cookie_daemon /home/bob /home/alice --output /home/mallory
```

Further (and non trivial) examples can be found in our [nushell scripts repo](https://github.com/nushell/nu_scripts)

## Syntax

The syntax of the def command is as follows.
`def <name> <signature> <block>`

The signature is a list of parameters flags and at maximum one rest argument. You can specify the type of each of them by appending `: <type>`.
Example:
```shell
def cmd [
parameter: string
--flag: int
...rest: path
] { ... }
```

It is possible to comment them by appending `# Comment text`!
Example
```shell
def cmd [
parameter # Paramter Comment
--flag: int # Flag comment
...rest: path # Rest comment
] { ... }
```

Flags can have a single character shorthand form. For example `--output` is often abbreviated by `-o`. You can declare a shorthand by writing `(-<shorthand>)` after the flag name.
Example
```shell
def cmd [
--flag(-f): int # Flag comment
] { ... }
```

You can make a parameter optional by adding `?` to its name. Optional parameters do not need to be passed.
(TODO Handling optional parameters in scripts is WIP. Please don't expect it to work seamlessly)
```shell
def cmd [
parameter?: path # Optional parameter
] { ... }
```
