# date list-timezone
List supported time zones.

## Usage
```shell
> date list-timezone {flags} 
 ```

## Flags
* -h, --help: Display this help message

## Examples
  List all supported time zones
```shell
> date list-timezone
 ```

  List all supported European time zones
```shell
> date list-timezone | where timezone =~ Europe
 ```

