# date format
Format a given date using the given format string.

## Usage
```shell
> date format <format> {flags} 
 ```

## Parameters
* `<format>` strftime format

## Flags
* -h, --help: Display this help message
* -t, --table: print date in a table

## Examples
  Format the current date
```shell
> date now | date format '%Y.%m.%d_%H %M %S,%z'
 ```

  Format the current date and print in a table
```shell
> date now | date format -t '%Y-%m-%d_%H:%M:%S %z'
 ```

