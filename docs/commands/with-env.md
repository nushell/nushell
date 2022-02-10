# with_env
Runs a block with an environment variable set.

## Usage
```shell
> with_env <variable> <block> {flags} 
 ```

## Parameters
* `<variable>` the environment variable to temporarily set
* `<block>` the block to run once the variable is set

## Flags
* -h, --help: Display this help message

## Examples
  Set the MYENV environment variable
```shell
> with_env [MYENV "my env value"] { echo $nu.env.MYENV }
 ```

  Set by primitive value list
```shell
> with_env [X Y W Z] { echo $nu.env.X $nu.env.W }
 ```

  Set by single row table
```shell
> with_env [[X W]; [Y Z]] { echo $nu.env.X $nu.env.W }
 ```

  Set by row(e.g. `open x.json` or `from json`)
```shell
> echo '{"X":"Y","W":"Z"}'|from json|with_env $it { echo $nu.env.X $nu.env.W }
 ```

