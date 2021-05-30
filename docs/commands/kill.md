# kill
Kill a process using the process id.

## Usage
```shell
> kill <pid> ...args {flags} 
 ```

## Parameters
* `<pid>` process id of process that is to be killed
* ...args: rest of processes to kill

## Flags
* -h, --help: Display this help message
* -f, --force: forcefully kill the process
* -q, --quiet: won't print anything to the console
* -s, --signal <integer>: signal decimal number to be sent instead of the default 15 (unsupported on Windows)

## Examples
  Kill the pid using the most memory
```shell
> ps | sort-by mem | last | kill $it.pid
 ```

  Force kill a given pid
```shell
> kill --force 12345
 ```

  Send INT signal
```shell
> kill -s 2 12345
 ```

