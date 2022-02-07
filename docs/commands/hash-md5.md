# hash md5
md5 encode a value

## Usage
```shell
> hash md5 ...args {flags} 
 ```

## Parameters
* ...args: optionally md5 encode data by column paths

## Flags
* -h, --help: Display this help message

## Examples
  md5 encode a string
```shell
> echo 'abcdefghijklmnopqrstuvwxyz' | hash md5
 ```

  md5 encode a file
```shell
> open ./nu_0_24_1_windows.zip | hash md5
 ```

