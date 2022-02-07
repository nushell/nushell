# str ltrim
trims whitespace or character from the beginning of text

## Usage
```shell
> str ltrim ...args {flags} 
 ```

## Parameters
* ...args: optionally trim text starting from the beginning by column paths

## Flags
* -h, --help: Display this help message
* -c, --char <string>: character to trim (default: whitespace)

## Examples
  Trim whitespace from the beginning of string
```shell
> echo ' Nu shell ' | str ltrim
 ```

  Trim a specific character
```shell
> echo '=== Nu shell ===' | str ltrim -c '='
 ```

