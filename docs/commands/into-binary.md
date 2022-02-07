# into binary
Convert value to a binary primitive

## Usage
```shell
> into binary ...args {flags} 
 ```

## Parameters
* ...args: column paths to convert to binary (for table input)

## Flags
* -h, --help: Display this help message
* -s, --skip <integer>: skip x number of bytes
* -b, --bytes <integer>: show y number of bytes

## Examples
  convert string to a nushell binary primitive
```shell
> echo 'This is a string that is exactly 52 characters long.' | into binary
 ```

  convert string to a nushell binary primitive
```shell
> echo 'This is a string that is exactly 52 characters long.' | into binary --skip 10
 ```

  convert string to a nushell binary primitive
```shell
> echo 'This is a string that is exactly 52 characters long.' | into binary --skip 10 --bytes 10
 ```

  convert a number to a nushell binary primitive
```shell
> echo 1 | into binary
 ```

  convert a boolean to a nushell binary primitive
```shell
> echo $true | into binary
 ```

  convert a filesize to a nushell binary primitive
```shell
> ls | where name == LICENSE | get size | into binary
 ```

  convert a filepath to a nushell binary primitive
```shell
> ls | where name == LICENSE | get name | path expand | into binary
 ```

  convert a decimal to a nushell binary primitive
```shell
> echo 1.234 | into binary
 ```

