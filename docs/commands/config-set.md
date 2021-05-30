# config set
Sets a value in the config

## Usage
```shell
> config set <key> <value> {flags} 
 ```

## Parameters
* `<key>` variable name to set
* `<value>` value to use

## Flags
* -h, --help: Display this help message

## Examples
  Set auto pivoting
```shell
> config set pivot_mode always
 ```

  Set line editor options
```shell
> config set line_editor [[edit_mode, completion_type]; [emacs circular]]
 ```

  Set coloring options
```shell
> config set color_config [[header_align header_bold]; [left $true]]
 ```

  Set nested options
```shell
> config set color_config.header_color white
 ```

