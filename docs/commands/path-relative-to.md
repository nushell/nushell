# path relative-to
Get a path as relative to another path.

Can be used only when the input and the argument paths are either both
absolute or both relative. The argument path needs to be a parent of the input
path.

## Usage
```shell
> path relative-to <path> ...args {flags} 
 ```

## Parameters
* `<path>` Parent shared with the input path
* ...args: Optionally operate by column path

## Flags
* -h, --help: Display this help message

## Examples
  Find a relative path from two absolute paths
```shell
> '/home/viking' | path relative-to '/home'
 ```

  Find a relative path from two relative paths
```shell
> 'eggs/bacon/sausage/spam' | path relative-to 'eggs/bacon/sausage'
 ```

