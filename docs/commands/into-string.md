# into string
Convert value to string

## Usage
```shell
> into string ...args {flags} 
 ```

## Parameters
* ...args: column paths to convert to string (for table input)

## Flags
* -h, --help: Display this help message
* -d, --decimals <integer>: decimal digits to which to round

## Examples
  convert decimal to string and round to nearest integer
```shell
> echo 1.7 | into string -d 0
 ```

  convert decimal to string
```shell
> echo 4.3 | into string
 ```

  convert string to string
```shell
> echo '1234' | into string
 ```

  convert boolean to string
```shell
> echo $true | into string
 ```

  convert date to string
```shell
> date now | into string
 ```

  convert filepath to string
```shell
> ls Cargo.toml | get name | into string
 ```

  convert filesize to string
```shell
> ls Cargo.toml | get size | into string
 ```

