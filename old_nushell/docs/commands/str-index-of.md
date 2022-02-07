# str index-of
Returns starting index of given pattern in string counting from 0. Returns -1 when there are no results.

## Usage
```shell
> str index-of <pattern> ...args {flags} 
 ```

## Parameters
* `<pattern>` the pattern to find index of
* ...args: optionally returns index of pattern in string by column paths

## Flags
* -h, --help: Display this help message
* -r, --range <any>: optional start and/or end index
* -e, --end: search from the end of the string

## Examples
  Returns index of pattern in string
```shell
> echo 'my_library.rb' | str index-of '.rb'
 ```

  Returns index of pattern in string with start index
```shell
> echo '.rb.rb' | str index-of '.rb' -r '1,'
 ```

  Returns index of pattern in string with end index
```shell
> echo '123456' | str index-of '6' -r ',4'
 ```

  Returns index of pattern in string with start and end index
```shell
> echo '123456' | str index-of '3' -r '1,4'
 ```

  Alternatively you can use this form
```shell
> echo '123456' | str index-of '3' -r [1 4]
 ```

  Returns index of pattern in string
```shell
> echo '/this/is/some/path/file.txt' | str index-of '/' -e
 ```

