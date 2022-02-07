# hash base64
base64 encode or decode a value

## Usage
```shell
> hash base64 ...args {flags} 
 ```

## Parameters
* ...args: optionally base64 encode / decode data by column paths

## Flags
* -h, --help: Display this help message
* -c, --character_set <string>: specify the character rules for encoding the input.
Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt'
* -e, --encode: encode the input as base64. This is the default behavior if not specified.
* -d, --decode: decode the input from base64

## Examples
  Base64 encode a string with default settings
```shell
> echo 'username:password' | hash base64
 ```

  Base64 encode a string with the binhex character set
```shell
> echo 'username:password' | hash base64 --character_set binhex --encode
 ```

  Base64 decode a value
```shell
> echo 'dXNlcm5hbWU6cGFzc3dvcmQ=' | hash base64 --decode
 ```

