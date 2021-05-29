# seq
Print sequences of numbers.

## Usage
```shell
> seq ...args <subcommand> {flags} 
 ```

## Subcommands
* seq date - print sequences of dates

## Parameters
* ...args: sequence values

## Flags
* -h, --help: Display this help message
* -s, --separator <string>: separator character (defaults to \n)
* -t, --terminator <string>: terminator character (defaults to \n)
* -w, --widths: equalize widths of all numbers by padding with zeros

## Examples
* sequence 1 to 10 with newline separator
```shell
> seq 1 10
 ```

* sequence 1.0 to 2.0 by 0.1s with newline separator
```shell
> seq 1.0 0.1 2.0
 ```

* sequence 1 to 10 with pipe separator
```shell
> seq -s '|' 1 10
 ```

* sequence 1 to 10 with pipe separator padded with 0
```shell
> seq -s '|' -w 1 10
 ```

* sequence 1 to 10 with pipe separator padded by 2s
```shell
> seq -s ' | ' -w 1 2 10
 ```

