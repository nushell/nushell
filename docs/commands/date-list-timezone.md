# date list_timezone
List supported time zones.

## Usage
```shell
> date list_timezone {flags} 
 ```

## Flags
* -h, --help: Display this help message

## Examples
  List all supported time zones
```shell
> date list_timezone
 ```

  List all supported European time zones
```shell
> date list_timezone | where timezone =~ Europe
 ```

