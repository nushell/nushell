# path basename
Get the final component of a path

## Usage
```shell
> path basename ...args {flags} 
 ```

## Parameters
* ...args: Optionally operate by column path

## Flags
* -h, --help: Display this help message
* -r, --replace <string>: Return original path with basename replaced by this string

## Examples
  Get basename of a path
```shell
> echo '/home/joe/test.txt' | path basename
 ```

  Replace basename of a path
```shell
> echo '/home/joe/test.txt' | path basename -r 'spam.png'
 ```

