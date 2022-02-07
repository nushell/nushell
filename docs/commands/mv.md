# mv
Move files or directories.

## Usage
```shell
> mv <source> <destination> {flags} 
 ```

## Parameters
* `<source>` the location to move files/directories from
* `<destination>` the location to move files/directories to

## Flags
* -h, --help: Display this help message

## Examples
  Rename a file
```shell
> mv before.txt after.txt
 ```

  Move a file into a directory
```shell
> mv test.txt my/subdirectory
 ```

  Move many files into a directory
```shell
> mv *.txt my/subdirectory
 ```

