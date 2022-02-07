# str starts-with
checks if string starts with pattern

## Usage
```shell
> str starts-with <pattern> ...args {flags} 
 ```

## Parameters
* `<pattern>` the pattern to match
* ...args: optionally matches prefix of text by column paths

## Flags
* -h, --help: Display this help message

## Examples
  Checks if string starts with 'my' pattern
```shell
> echo 'my_library.rb' | str starts-with 'my'
 ```

