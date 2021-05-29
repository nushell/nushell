# path dirname
Get the parent directory of a path

## Usage
```shell
> path dirname ...args {flags} 
 ```

## Parameters
* ...args: Optionally operate by column path

## Flags
* -h, --help: Display this help message
* -r, --replace <string>: Return original path with dirname replaced by this string
* -n, --num-levels <integer>: Number of directories to walk up

## Examples
  Get dirname of a path
```shell
> echo '/home/joe/code/test.txt' | path dirname
 ```

  Walk up two levels
```shell
> echo '/home/joe/code/test.txt' | path dirname -n 2
 ```

  Replace the part that would be returned with a custom path
```shell
> echo '/home/joe/code/test.txt' | path dirname -n 2 -r /home/viking
 ```

