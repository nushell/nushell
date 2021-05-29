# keep
Keep the number of rows only.

## Usage
```shell
> keep (rows) <subcommand> {flags} 
 ```

## Subcommands
* keep until - Keeps rows until the condition matches.
* keep while - Keeps rows while the condition matches.

## Parameters
* `(rows)` Starting from the front, the number of rows to keep

## Flags
* -h, --help: Display this help message

## Examples
  Keep the first row
```shell
> echo [1 2 3] | keep
 ```

  Keep the first four rows
```shell
> echo [1 2 3 4 5] | keep 4
 ```

