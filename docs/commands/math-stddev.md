# math stddev
Finds the stddev of a list of numbers or tables

## Usage
```shell
> math stddev {flags} 
 ```

## Flags
* -h, --help: Display this help message
* -s, --sample: calculate sample standard deviation

## Examples
  Get the stddev of a list of numbers
```shell
> echo [1 2 3 4 5] | math stddev
 ```

  Get the sample stddev of a list of numbers
```shell
> echo [1 2 3 4 5] | math stddev -s
 ```

