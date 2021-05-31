# url scheme
gets the scheme (eg http, file) of a url

## Usage
```shell
> url scheme ...args {flags} 
 ```

## Parameters
* ...args: optionally operate by path

## Flags
* -h, --help: Display this help message

## Examples
  Get scheme of a url
```shell
> echo 'http://www.example.com' | url scheme
 ```

  You get an empty string if there is no scheme
```shell
> echo 'test' | url scheme
 ```

