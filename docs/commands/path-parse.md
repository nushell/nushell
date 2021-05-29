# path parse
Convert a path into structured data.

Each path is split into a table with 'parent', 'stem' and 'extension' fields.
On Windows, an extra 'prefix' column is added.

## Usage
```shell
> path parse ...args {flags} 
 ```

## Parameters
* ...args: Optionally operate by column path

## Flags
* -h, --help: Display this help message
* -e, --extension <string>: Manually supply the extension (without the dot)

## Examples
  Parse a path
```shell
> echo '/home/viking/spam.txt' | path parse
 ```

  Replace a complex extension
```shell
> echo '/home/viking/spam.tar.gz' | path parse -e tar.gz | update extension { 'txt' }
 ```

  Ignore the extension
```shell
> echo '/etc/conf.d' | path parse -e ''
 ```

  Parse all paths under the 'name' column
```shell
> ls | path parse name
 ```

