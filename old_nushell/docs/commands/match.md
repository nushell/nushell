# match
Filter rows by Regex pattern.

## Usage
```shell
> match <member> <regex> {flags} 
 ```

## Parameters
* `<member>` the column name to match
* `<regex>` the regex to match with

## Flags
* -h, --help: Display this help message
* -i, --insensitive: case-insensitive search
* -m, --multiline: multi-line mode: ^ and $ match begin/end of line
* -s, --dotall: dotall mode: allow a dot . to match newline character \n
* -v, --invert: invert the match

