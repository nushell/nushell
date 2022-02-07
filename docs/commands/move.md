# move
Move columns.

## Usage
```shell
> move ...args {flags} 
 ```

## Parameters
* ...args: the columns to move

## Flags
* -h, --help: Display this help message
* --after <column path>: the column that will precede the columns moved
* --before <column path>: the column that will be next the columns moved

## Examples
  Move the column "type" before the column "name"
```shell
> ls | move type --before name | first
 ```

  or move the column "chickens" after "name"
```shell
> ls | move chickens --after name | first
 ```

  you can selectively move many columns in either direction
```shell
> ls | move name chickens --after type | first
 ```

