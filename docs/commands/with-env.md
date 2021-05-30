# with-env
Runs a block with an environment variable set.

## Usage
```shell
> with-env <variable> <block> {flags} 
 ```

## Parameters
* `<variable>` the environment variable to temporarily set
* `<block>` the block to run once the variable is set

## Flags
* -h, --help: Display this help message

## Examples
  Set the MYENV environment variable
```shell
> with-env [MYENV "my env value"] { echo $nu.env.MYENV }
 ```

  Set by primitive value list
```shell
> with-env [X Y W Z] { echo $nu.env.X $nu.env.W }
 ```

  Set by single row table
```shell
> with-env [[X W]; [Y Z]] { echo $nu.env.X $nu.env.W }
 ```

  Set by row(e.g. `open x.json` or `from json`)
```shell
> echo '{"X":"Y","W":"Z"}'|from json|with-env $it { echo $nu.env.X $nu.env.W }
 ```

