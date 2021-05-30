# str contains
Checks if string contains pattern

## Usage
```shell
> str contains <pattern> ...args {flags} 
 ```

## Parameters
* `<pattern>` the pattern to find
* ...args: optionally check if string contains pattern by column paths

## Flags
* -h, --help: Display this help message
* -i, --insensitive: search is case insensitive

## Examples
  Check if string contains pattern
```shell
> echo 'my_library.rb' | str contains '.rb'
 ```

  Check if string contains pattern case insensitive
```shell
> echo 'my_library.rb' | str contains -i '.RB'
 ```

