# str trim
trims text

## Usage
```shell
> str trim ...args {flags} 
 ```

## Parameters
* ...args: optionally trim text by column paths

## Flags
* -h, --help: Display this help message
* -c, --char <string>: character to trim (default: whitespace)

## Examples
  Trim whitespace
```shell
> echo 'Nu shell ' | str trim
 ```

  Trim a specific character
```shell
> echo '=== Nu shell ===' | str trim -c '=' | str trim
 ```

