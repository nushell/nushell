# str rtrim
trims whitespace or character from the end of text

## Usage
```shell
> str rtrim ...args {flags} 
 ```

## Parameters
* ...args: optionally trim text starting from the end by column paths

## Flags
* -h, --help: Display this help message
* -c, --char <string>: character to trim (default: whitespace)

## Examples
  Trim whitespace from the end of string
```shell
> echo ' Nu shell ' | str rtrim
 ```

  Trim a specific character
```shell
> echo '=== Nu shell ===' | str rtrim -c '='
 ```

