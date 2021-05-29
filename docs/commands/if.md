# if
Run blocks if a condition is true or false.

## Usage
```shell
> if <condition> <then_case> <else_case> {flags} 
 ```

## Parameters
* `<condition>` the condition that must match
* `<then_case>` block to run if condition is true
* `<else_case>` block to run if condition is false

## Flags
* -h, --help: Display this help message

## Examples
  Run a block if a condition is true
```shell
> let x = 10; if $x > 5 { echo 'greater than 5' } { echo 'less than or equal to 5' }
 ```

  Run a block if a condition is false
```shell
> let x = 1; if $x > 5 { echo 'greater than 5' } { echo 'less than or equal to 5' }
 ```

