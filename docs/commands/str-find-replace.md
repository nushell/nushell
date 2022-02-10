# str find_replace
finds and replaces text

## Usage
```shell
> str find_replace <find> <replace> ...args {flags} 
 ```

## Parameters
* `<find>` the pattern to find
* `<replace>` the replacement pattern
* ...args: optionally find and replace text by column paths

## Flags
* -h, --help: Display this help message
* -a, --all: replace all occurrences of find string

## Examples
  Find and replace contents with capture group
```shell
> echo 'my_library.rb' | str find_replace '(.+).rb' '$1.nu'
 ```

  Find and replace all occurrences of find string
```shell
> echo 'abc abc abc' | str find_replace -a 'b' 'z'
 ```

