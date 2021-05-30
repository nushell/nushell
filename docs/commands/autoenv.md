# autoenv
Manage directory specific environment variables and scripts.

Create a file called .nu-env in any directory and run 'autoenv trust' to let nushell load it when entering the directory.
The .nu-env file has the same format as your $HOME/nu/config.toml file. By loading a .nu-env file the following applies:
  * - environment variables (section \"[env]\") are loaded from the .nu-env file. Those env variables only exist in this directory (and children directories)
  * - the \"startup\" commands are run when entering the directory
  * - the \"on_exit\" commands are run when leaving the directory


## Usage
```shell
> autoenv <subcommand> {flags} 
 ```

## Subcommands
* autoenv trust - Trust a .nu-env file in the current or given directory
* autoenv untrust - Untrust a .nu-env file in the current or given directory

## Flags
* -h, --help: Display this help message

## Examples
  Example .nu-env file
```shell
> cat .nu-env
 ```
        startup = ["echo ...entering the directory", "echo 1 2 3"]
        on_exit = ["echo ...leaving the directory"]

        [env]
        mykey = "myvalue"
            

