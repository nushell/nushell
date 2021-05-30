# parse
Parse columns from string data using a simple pattern.

## Usage
```shell
> parse <pattern> {flags} 
 ```

## Parameters
* `<pattern>` the pattern to match. Eg) "{foo}: {bar}"

## Flags
* -h, --help: Display this help message
* -r, --regex: use full regex syntax for patterns

## Examples
  Parse a string into two named columns
```shell
> echo "hi there" | parse "{foo} {bar}"
 ```

  Parse a string using regex pattern
```shell
> echo "hi there" | parse -r "(?P<foo>\w+) (?P<bar>\w+)"
 ```

