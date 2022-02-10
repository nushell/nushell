# path relative_to
Get a path as relative to another path.

Can be used only when the input and the argument paths are either both
absolute or both relative. The argument path needs to be a parent of the input
path.

## Usage
```shell
> path relative_to <path> ...args {flags} 
 ```

## Parameters
* `<path>` Parent shared with the input path
* ...args: Optionally operate by column path

## Flags
* -h, --help: Display this help message

## Examples
  Find a relative path from two absolute paths
```shell
> '/home/viking' | path relative_to '/home'
 ```

  Find a relative path from two relative paths
```shell
> 'eggs/bacon/sausage/spam' | path relative_to 'eggs/bacon/sausage'
 ```

