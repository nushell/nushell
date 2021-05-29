# empty?
Check for empty values.

## Usage
```shell
> empty? ...args {flags} 
 ```

## Parameters
* ...args: the names of the columns to check emptiness. Pass an optional block to replace if empty

## Flags
* -h, --help: Display this help message

## Examples
  Check if a value is empty
```shell
> echo '' | empty?
 ```

  more than one column
```shell
> echo [[meal size]; [arepa small] [taco '']] | empty? meal size
 ```

  use a block if setting the empty cell contents is wanted
```shell
> echo [[2020/04/16 2020/07/10 2020/11/16]; ['' [27] [37]]] | empty? 2020/04/16 { [33 37] }
 ```

