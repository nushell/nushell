# date to-timezone
Convert a date to a given time zone.

Use 'date list-timezone' to list all supported time zones.

## Usage
```shell
> date to-timezone <time zone> {flags} 
 ```

## Parameters
  <time zone> time zone description

## Flags
* -h, --help: Display this help message

## Examples
  Get the current date in UTC+05:00
```shell
> date now | date to-timezone +0500
 ```

  Get the current local date
```shell
> date now | date to-timezone local
 ```

  Get the current date in Hawaii
```shell
> date now | date to-timezone US/Hawaii
 ```

