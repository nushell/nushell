# str ends_with
checks if string ends with pattern

## Usage
```shell
> str ends_with <pattern> ...args {flags} 
 ```

## Parameters
* `<pattern>` the pattern to match
* ...args: optionally matches suffix of text by column paths

## Flags
* -h, --help: Display this help message

## Examples
  Checks if string ends with '.rb' pattern
```shell
> echo 'my_library.rb' | str ends_with '.rb'
 ```

