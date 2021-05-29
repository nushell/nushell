# math sum
Finds the sum of a list of numbers or tables

## Usage
```shell
> math sum {flags} 
 ```

## Flags
* -h, --help: Display this help message

## Examples
  Sum a list of numbers
```shell
> echo [1 2 3] | math sum
 ```

  Get the disk usage for the current directory
```shell
> ls --all --du | get size | math sum
 ```

