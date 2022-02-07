# range
Return only the selected rows.

## Usage
```shell
> range <rows> {flags} 
 ```

## Parameters
* `<rows>` range of rows to return: Eg) 4..7 (=> from 4 to 7)

## Flags
* -h, --help: Display this help message

## Examples
  Return rows 1 through 3
```shell
> echo [1 2 3 4 5] | range 1..3
 ```

  Return the third row from the end, through to the end
```shell
> echo [1 2 3 4 5] | range (-3..)
 ```

