# random integer
Generate a random integer [min..max]

## Usage
```shell
> random integer (range) {flags} 
 ```

## Parameters
* `(range)` Range of values

## Flags
* -h, --help: Display this help message

## Examples
  Generate an unconstrained random integer
```shell
> random integer
 ```

  Generate a random integer less than or equal to 500
```shell
> random integer ..500
 ```

  Generate a random integer greater than or equal to 100000
```shell
> random integer 100000..
 ```

  Generate a random integer between 1 and 10
```shell
> random integer 1..10
 ```

