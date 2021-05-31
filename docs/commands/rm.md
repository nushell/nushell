# rm
Remove file(s).

## Usage
```shell
> rm ...args {flags} 
 ```

## Parameters
* ...args: the file path(s) to remove

## Flags
* -h, --help: Display this help message
* -t, --trash: use the platform's recycle bin instead of permanently deleting
* -p, --permanent: don't use recycle bin, delete permanently
* -r, --recursive: delete subdirectories recursively
* -f, --force: suppress error when no file

## Examples
  Delete or move a file to the system trash (depending on 'rm_always_trash' config option)
```shell
> rm file.txt
 ```

  Move a file to the system trash
```shell
> rm --trash file.txt
 ```

  Delete a file permanently
```shell
> rm --permanent file.txt
 ```

  Delete a file, and suppress errors if no file is found
```shell
> rm --force file.txt
 ```

