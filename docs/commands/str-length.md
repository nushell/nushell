# str length
outputs the lengths of the strings in the pipeline

## Usage
```shell
> str length ...args {flags} 
 ```

## Parameters
* ...args: optionally find length of text by column paths

## Flags
* -h, --help: Display this help message

## Examples
  Return the lengths of multiple strings
```shell
> echo 'hello' | str length
 ```

  Return the lengths of multiple strings
```shell
> echo 'hi' 'there' | str length
 ```

