# path split
Split a path into parts by a separator.

## Usage
```shell
> path split ...args {flags} 
 ```

## Parameters
* ...args: Optionally operate by column path

## Flags
* -h, --help: Display this help message

## Examples
  Split a path into parts
```shell
> echo '/home/viking/spam.txt' | path split
 ```

  Split all paths under the 'name' column
```shell
> ls | path split name
 ```

