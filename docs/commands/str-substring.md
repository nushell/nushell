# str substring
substrings text

## Usage
```shell
> str substring <range> ...args {flags} 
 ```

## Parameters
* `<range>` the indexes to substring [start end]
* ...args: optionally substring text by column paths

## Flags
* -h, --help: Display this help message

## Examples
  Get a substring from the text
```shell
> echo 'good nushell' | str substring [5 12]
 ```

  Alternatively, you can use the form
```shell
> echo 'good nushell' | str substring '5,12'
 ```

  Drop the last `n` characters from the string
```shell
> echo 'good nushell' | str substring ',-5'
 ```

  Get the remaining characters from a starting index
```shell
> echo 'good nushell' | str substring '5,'
 ```

  Get the characters from the beginning until ending index
```shell
> echo 'good nushell' | str substring ',7'
 ```

