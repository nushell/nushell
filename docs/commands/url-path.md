# url path
gets the path of a url

## Usage
```shell
> url path ...args {flags} 
 ```

## Parameters
* ...args: optionally operate by column path

## Flags
* -h, --help: Display this help message

## Examples
  Get path of a url
```shell
> echo 'http://www.example.com/foo/bar' | url path
 ```

  A trailing slash will be reflected in the path
```shell
> echo 'http://www.example.com' | url path
 ```

