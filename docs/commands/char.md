# char
Output special characters (e.g., 'newline').

## Usage
```shell
> char (character) ...args {flags} 
 ```

## Parameters
* `(character)` the name of the character to output
* ...args: multiple Unicode bytes

## Flags
* -h, --help: Display this help message
* -l, --list: List all supported character names
* -u, --unicode: Unicode string i.e. 1f378

## Examples
  Output newline
```shell
> char newline
 ```

  Output prompt character, newline and a hamburger character
```shell
> echo (char prompt) (char newline) (char hamburger)
 ```

  Output Unicode character
```shell
> char -u 1f378
 ```

  Output multi-byte Unicode character
```shell
> char -u 1F468 200D 1F466 200D 1F466
 ```

