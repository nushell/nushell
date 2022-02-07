# str to-datetime
converts text into datetime

## Usage
```shell
> str to-datetime ...args {flags} 
 ```

## Parameters
* ...args: optionally convert text into datetime by column paths

## Flags
* -h, --help: Display this help message
* -z, --timezone <string>: Specify timezone if the input is timestamp, like 'UTC/u' or 'LOCAL/l'
* -o, --offset <integer>: Specify timezone by offset if the input is timestamp, like '+8', '-4', prior than timezone
* -f, --format <string>: Specify date and time formatting

## Examples
  Convert to datetime
```shell
> echo '16.11.1984 8:00 am +0000' | str to-datetime
 ```

  Convert to datetime
```shell
> echo '2020-08-04T16:39:18+00:00' | str to-datetime
 ```

  Convert to datetime using a custom format
```shell
> echo '20200904_163918+0000' | str to-datetime -f '%Y%m%d_%H%M%S%z'
 ```

  Convert timestamp (no larger than 8e+12) to datetime using a specified timezone
```shell
> echo '1614434140' | str to-datetime -z 'UTC'
 ```

  Convert timestamp (no larger than 8e+12) to datetime using a specified timezone offset (between -12 and 12)
```shell
> echo '1614434140' | str to-datetime -o '+9'
 ```

