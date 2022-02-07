# let
Create a variable and give it a value.

## Usage
```shell
> let <name> <equals> <expr> {flags} 
 ```

## Parameters
* `<name>` the name of the variable
* `<equals>` the equals sign
* `<expr>` the value for the variable

## Flags
* -h, --help: Display this help message

## Examples
  Assign a simple value to a variable
```shell
> let x = 3
 ```

  Assign the result of an expression to a variable
```shell
> let result = (3 + 7); echo $result
 ```

  Create a variable using the full name
```shell
> let $three = 3
 ```

