# flatten
Flatten the table.

## Usage
```shell
> flatten ...args {flags} 
 ```

## Parameters
* ...args: optionally flatten data by column

## Flags
* -h, --help: Display this help message

## Examples
* flatten a table
```shell
> echo [[N, u, s, h, e, l, l]] | flatten | first
 ```

* flatten a column having a nested table
```shell
> echo [[origin, people]; [Ecuador, (echo [[name, meal]; ['Andres', 'arepa']])]] | flatten | get meal
 ```

  restrict the flattening by passing column names
```shell
> echo [[origin, crate, versions]; [World, (echo [[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten versions | last | get versions
 ```

