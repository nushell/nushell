# url query
gets the query of a url

## Usage
```shell
> url query ...args {flags} 
 ```

## Parameters
* ...args: optionally operate by column path

## Flags
* -h, --help: Display this help message

## Examples
  Get query of a url
```shell
> echo 'http://www.example.com/?foo=bar&baz=quux' | url query
 ```

  No query gives the empty string
```shell
> echo 'http://www.example.com/' | url query
 ```

