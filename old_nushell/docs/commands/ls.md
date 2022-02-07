# ls
View the contents of the current or given path.

## Usage
```shell
> ls (path) {flags} 
 ```

## Parameters
* `(path)` a path to get the directory contents from

## Flags
* -h, --help: Display this help message
* -a, --all: Show hidden files
* -l, --long: List all available columns for each entry
* -s, --short-names: Only print the file names and not the path
* -d, --du: Display the apparent directory size in place of the directory metadata size

## Examples
  List all files in the current directory
```shell
> ls
 ```

  List all files in a subdirectory
```shell
> ls subdir
 ```

  List all rust files
```shell
> ls *.rs
 ```

