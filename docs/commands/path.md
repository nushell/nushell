# path
Explore and manipulate paths.

There are three ways to represent a path:

* As a path literal, e.g., '/home/viking/spam.txt'
* As a structured path: a table with 'parent', 'stem', and 'extension' (and
* 'prefix' on Windows) columns. This format is produced by the 'path parse'
  subcommand.
* As an inner list of path parts, e.g., '[[ / home viking spam.txt ]]'.
  Splitting into parts is done by the `path split` command.

All subcommands accept all three variants as an input. Furthermore, the 'path
join' subcommand can be used to join the structured path or path parts back into
the path literal.

## Usage
```shell
> path <subcommand> {flags} 
 ```

## Subcommands
* path basename - Get the final component of a path
* path dirname - Get the parent directory of a path
* path exists - Check whether a path exists
* path expand - Expand a path to its absolute form
* path join - Join a structured path or a list of path parts.
* path parse - Convert a path into structured data.
* path relative-to - Get a path as relative to another path.
* path split - Split a path into parts by a separator.
* path type - Get the type of the object a path refers to (e.g., file, dir, symlink)

## Flags
* -h, --help: Display this help message

