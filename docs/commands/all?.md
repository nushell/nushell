# all?
Find if the table rows matches the condition.

## Usage
```shell
> all? <condition> {flags} 
 ```

## Parameters
* `<condition>` the condition that must match

## Flags
* -h, --help: Display this help message

## Examples
  Find if services are running
```shell
> echo [[status]; [UP] [UP]] | all? status == UP
 ```

  Check that all values are even
```shell
> echo [2 4 6 8] | all? ($it mod 2) == 0
 ```

