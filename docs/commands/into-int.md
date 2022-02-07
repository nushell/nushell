# into int
Convert value to integer

## Usage
```shell
> into int ...args {flags} 
 ```

## Parameters
* ...args: column paths to convert to int (for table input)

## Flags
* -h, --help: Display this help message

## Examples
  Convert string to integer in table
```shell
> echo [[num]; ['-5'] [4] [1.5]] | into int num
 ```

  Convert string to integer
```shell
> echo '2' | into int
 ```

  Convert decimal to integer
```shell
> echo 5.9 | into int
 ```

  Convert decimal string to integer
```shell
> echo '5.9' | into int
 ```

  Convert file size to integer
```shell
> echo 4KB | into int
 ```

  Convert bool to integer
```shell
> echo $false $true | into int
 ```

