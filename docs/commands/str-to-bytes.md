# str to-int

converts text into byte sizes

## Usage

```shell
> str to-bytes ...args {flags}
```

## Parameters

-   ...args: optionally convert text into bytes by column paths

## Flags

-   -h, --help: Display this help message
-   -r, --radix <number>: radix of integer

## Examples

Convert to bytes

```shell
> echo '255' | str to-byes
```

Convert str column to bytes

```shell
> echo [['count']; ['255']] | str to-bytes count | get count
```

Convert to bytes from binary

```shell
> echo '1101' | str to-bytes -r 2
```

Convert to bytes from hex

```shell
> echo 'FF' | str to-bytes -r 16
```
