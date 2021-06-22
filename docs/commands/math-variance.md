# math variance
Finds the variance of a list of numbers or tables

## Usage
```shell
> math variance {flags} 
 ```

## Flags
* -h, --help: Display this help message
* -s, --sample: calculate sample variance

## Examples
  Get the variance of a list of numbers
```shell
> echo [1 2 3 4 5] | math variance
 ```

  Get the sample variance of a list of numbers
```shell
> echo [1 2 3 4 5] | math variance -s
 ```

