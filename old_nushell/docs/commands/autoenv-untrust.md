# autoenv untrust
Untrust a .nu-env file in the current or given directory

## Usage
```shell
> autoenv untrust (dir) {flags} 
 ```

## Parameters
* `(dir)` Directory to disallow

## Flags
* -h, --help: Display this help message

## Examples
  Disallow .nu-env file in current directory
```shell
> autoenv untrust
 ```

  Disallow .nu-env file in directory foo
```shell
> autoenv untrust foo
 ```

