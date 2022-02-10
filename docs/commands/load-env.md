# load_env
Set environment variables using a table stream

## Usage
```shell
> load_env (environ) {flags} 
 ```

## Parameters
* `(environ)` Optional environment table to load in. If not provided, will use the table provided on the input stream

## Flags
* -h, --help: Display this help message

## Examples
  Load variables from an input stream
```shell
> echo [[name, value]; ["NAME", "JT"] ["AGE", "UNKNOWN"]] | load_env; echo $nu.env.NAME
 ```

  Load variables from an argument
```shell
> load_env [[name, value]; ["NAME", "JT"] ["AGE", "UNKNOWN"]]; echo $nu.env.NAME
 ```

  Load variables from an argument and an input stream
```shell
> echo [[name, value]; ["NAME", "JT"]] | load_env [[name, value]; ["VALUE", "FOO"]]; echo $nu.env.NAME $nu.env.VALUE
 ```

