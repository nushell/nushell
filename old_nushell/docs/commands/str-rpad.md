# str rpad
pad a string with a character a certain length

## Usage
```shell
> str rpad ...args {flags} 
 ```

## Parameters
* ...args: optionally check if string contains pattern by column paths

## Flags
* -h, --help: Display this help message
* -l, --length <integer> (required parameter): length to pad to
* -c, --character <string> (required parameter): character to pad with

## Examples
  Right pad a string with a character a number of places
```shell
> echo 'nushell' | str rpad -l 10 -c '*'
 ```

  Right pad a string with a character a number of places
```shell
> echo '123' | str rpad -l 10 -c '0'
 ```

  Use rpad to truncate a string
```shell
> echo '123456789' | str rpad -l 3 -c '0'
 ```

  Use rpad to pad Unicode
```shell
> echo '▉' | str rpad -l 10 -c '▉'
 ```

