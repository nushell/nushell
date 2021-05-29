# path join
Join a structured path or a list of path parts.

Optionally, append an additional path to the result. It is designed to accept
the output of 'path parse' and 'path split' subcommands.

## Usage
```shell
> path join ...args {flags} 
 ```

## Parameters
* ...args: Optionally operate by column path

## Flags
* -h, --help: Display this help message
* -a, --append <file path>: Path to append to the input

## Examples
  Append a filename to a path
```shell
> echo '/home/viking' | path join -a spam.txt
 ```

  Join a list of parts into a path
```shell
> echo [ '/' 'home' 'viking' 'spam.txt' ] | path join
 ```

  Join a structured path into a path
```shell
> echo [[ parent stem extension ]; [ '/home/viking' 'spam' 'txt' ]] | path join
 ```

