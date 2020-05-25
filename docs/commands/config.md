# config

Configuration management.

Syntax: `config {flags}`

### Flags

    --load <file path shape>
      load the config from the path give

    --set <any shape>
      set a value in the config, eg) --set [key value]

    --set_into <member shape>
      sets a variable from values in the pipeline

    --get <any shape>
      get a value from the config

    --remove <any shape>
      remove a value from the config

    --clear
      clear the config

    --path
      return the path to the config file

### Variables

| Variable           | Type                   | Description                                                         |
| ------------------ | ---------------------- | ------------------------------------------------------------------- |
| path               | table of strings       | PATH to use to find binaries                                        |
| env                | row                    | the environment variables to pass to external commands              |
| ctrlc_exit         | boolean                | whether or not to exit Nu after multiple ctrl-c presses             |
| table_mode         | "light" or other       | enable lightweight or normal tables                                 |
| edit_mode          | "vi" or "emacs"        | changes line editing to "vi" or "emacs" mode                        |
| key_timeout        | integer (milliseconds) | vi: the delay to wait for a longer key sequence after ESC           |
| history_size       | integer                | maximum entries that will be stored in history (100,000 default)    |
| completion_mode    | "circular" or "list"   | changes completion type to "circular" (default) or "list" mode      |
| pivot_mode      | "auto" or "always" or "never"                | "auto" will only pivot single row tables if the output is greater than the terminal width. "always" will always pivot single row tables. "never" will never pivot single row tables.            |
| complete_from_path | boolean                | whether or not to complete names of binaries on PATH (default true) |

## Examples

```shell
> config --set [table_mode "light"]
```

A more detailed description on how to use this command to configure Nu shell can be found in the configuration chapter of [Nu Book](https://www.nushell.sh/book/en/configuration.html).
