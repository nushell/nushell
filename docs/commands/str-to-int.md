# str to-int
converts text into integer

## Usage
```shell
> str to-int ...args {flags} 
 ```

## Parameters
* ...args: optionally convert text into integer by column paths

## Flags
* -h, --help: Display this help message
* -r, --radix <number>: radix of integer

## Examples
  Convert to an integer
```shell
> echo '255' | str to-int
 ```

  Convert str column to an integer
```shell
> echo [['count']; ['255']] | str to-int count | get count
 ```

  Convert to integer from binary
```shell
> echo '1101' | str to-int -r 2
 ```

  Convert to integer from hex
```shell
> echo 'FF' | str to-int -r 16
 ```

