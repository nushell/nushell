# any?
Find if the table rows matches the condition.

## Usage
```shell
> any? <condition> {flags} 
 ```

## Parameters
* `<condition>` the condition that must match

## Flags
* -h, --help: Display this help message

## Examples
  Find if a service is not running
```shell
> echo [[status]; [UP] [DOWN] [UP]] | any? status == DOWN
 ```

  Check if any of the values is odd
```shell
> echo [2 4 1 6 8] | any? ($it mod 2) == 1
 ```

