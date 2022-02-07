# str lpad
pad a string with a character a certain length

## Usage
```shell
> str lpad ...args {flags} 
 ```

## Parameters
* ...args: optionally check if string contains pattern by column paths

## Flags
* -h, --help: Display this help message
* -l, --length <integer> (required parameter): length to pad to
* -c, --character <string> (required parameter): character to pad with

## Examples
  Left pad a string with a character a number of places
```shell
> echo 'nushell' | str lpad -l 10 -c '*'
 ```

  Left pad a string with a character a number of places
```shell
> echo '123' | str lpad -l 10 -c '0'
 ```

  Use lpad to truncate a string
```shell
> echo '123456789' | str lpad -l 3 -c '0'
 ```

  Use lpad to pad Unicode
```shell
> echo '▉' | str lpad -l 10 -c '▉'
 ```

