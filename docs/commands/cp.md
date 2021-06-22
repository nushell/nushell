# cp
Copy files.

## Usage
```shell
> cp <src> <dst> {flags} 
 ```

## Parameters
* `<src>` the place to copy from
* `<dst>` the place to copy to

## Flags
* -h, --help: Display this help message
* -r, --recursive: copy recursively through subdirectories

## Examples
  Copy myfile to dir_b
```shell
> cp myfile dir_b
 ```

  Recursively copy dir_a to dir_b
```shell
> cp -r dir_a dir_b
 ```

