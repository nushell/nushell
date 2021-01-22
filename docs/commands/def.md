# def

Use `def` to create a custom command.

## Examples

```
> def my_command [] { echo hi nu }
> my_command
hi nu
```

```
> def my_command [adjective: string, num: int] { echo $adjective $num meet nu }
> my_command nice 2
nice 2 meet nu
```
