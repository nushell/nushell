# seq date
print sequences of dates

## Usage
```shell
> seq date {flags} 
 ```

## Flags
* -h, --help: Display this help message
* -s, --separator <string>: separator character (defaults to \n)
* -o, --output_format <string>: prints dates in this format (defaults to %Y-%m-%d)
* -i, --input_format <string>: give argument dates in this format (defaults to %Y-%m-%d)
* -b, --begin_date <string>: beginning date range
* -e, --end_date <string>: ending date
* -n, --increment <integer>: increment dates by this number
* -d, --days <integer>: number of days to print
* -r, --reverse: print dates in reverse

## Examples
  print the next 10 days in YYYY-MM-DD format with newline separator
```shell
> seq date --days 10
 ```

  print the previous 10 days in YYYY-MM-DD format with newline separator
```shell
> seq date --days 10 -r
 ```

  print the previous 10 days starting today in MM/DD/YYYY format with newline separator
```shell
> seq date --days 10 -o '%m/%d/%Y' -r
 ```

  print the first 10 days in January, 2020
```shell
> seq date -b '2020-01-01' -e '2020-01-10'
 ```

  print every fifth day between January 1st 2020 and January 31st 2020
```shell
> seq date -b '2020-01-01' -e '2020-01-31' -n 5
 ```

  starting on May 5th, 2020, print the next 10 days in your locale's date format, colon separated
```shell
> seq date -o %x -s ':' -d 10 -b '2020-05-01'
 ```

