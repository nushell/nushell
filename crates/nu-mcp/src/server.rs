use std::sync::Arc;

use indoc::formatdoc;
use nu_protocol::{UseAnsiColoring, engine::EngineState};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{
        tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::evaluation::{EvalResult, Evaluator};

pub struct NushellMcpServer {
    tool_router: ToolRouter<Self>,
    evaluator: Evaluator,
}

#[tool_router]
impl NushellMcpServer {
    pub fn new(mut engine_state: EngineState) -> Self {
        // Configure the engine state for MCP
        if let Some(config) = Arc::get_mut(&mut engine_state.config) {
            config.use_ansi_coloring = UseAnsiColoring::False;
            config.color_config.clear();
        }
        NushellMcpServer {
            tool_router: Self::tool_router(),
            evaluator: Evaluator::new(engine_state),
        }
    }

    #[tool(description = r#"List available Nushell native commands. 
By default all available commands will be returned. To find a specific command by searching command names, descriptions and search terms, use the find parameter."#)]
    async fn list_commands(
        &self,
        Parameters(ListCommandsRequest { find, cursor }): Parameters<ListCommandsRequest>,
    ) -> Result<Json<EvalResult>, McpError> {
        let cmd = if let Some(f) = find {
            format!("help commands --find {f}")
        } else {
            "help commands".to_string()
        };

        self.evaluator.eval(&cmd, cursor).map(Json)
    }

    #[tool(
        description = "Get help for a specific Nushell command. This will only work on commands that are native to nushell. To find out if a command is native to nushell you can use the find_command tool."
    )]
    async fn command_help(
        &self,
        Parameters(CommandNameRequest { name, cursor }): Parameters<CommandNameRequest>,
    ) -> Result<Json<EvalResult>, McpError> {
        let cmd = format!("help {name}");
        self.evaluator.eval(&cmd, cursor).map(Json)
    }

    #[tool(description = r#"Execute a command in the nushell.
    This will return the output and error concatenated into a single string, as
    you would see from running on the command line. There will also be an indication
    of if the command succeeded or failed.

    Avoid commands that produce a large amount of output, and consider piping those outputs to files.
    If you need to run a long lived command, background it - e.g. `job spawn { uvicorn main:app }` so that
    this tool does not run indefinitely.

    Command Equlivalents to Bash:

    | Bash Command | Nushell Command | Description |
    |--------------|-----------------|-------------|
    | `mkdir -p <path>` | `mkdir <path>` | Creates the given path, creating parents as necessary |
    | `> <path>`  | `o> <path>` | Save command output to a file |
    | `>> <path>` | `o>> <path>` | Append command output to a file |
    | `> /dev/null`	| `ignore`	| Discard command output | 
    | `> /dev/null 2>&1` |	`o+e>| ignore`	| Discard command output, including stderr | 
    | `cmd1 | tee log.txt | cmd2` |	`cmd1 | tee { save log.txt } | cmd2` |	Tee command output to a log file |
    | `command | head -5`	| `command | first 5` |	Limit the output to the first 5 rows of an internal command (see also last and skip) |
    | `cat <path>`	| `open --raw <path>`	| Display the contents of the given file |
    | `cat <(<command1>) <(<command2>)`	| `[(command1), (command2)] | str join`	| Concatenate the outputs of command1 and command2 |
    | `cat <path> <(<command>)`	| `[(open --raw <path>), (command)] | str join`	| Concatenate the contents of the given file and output of command |
    | `for f in *.md; do echo $f; done` | `ls *.md | each { $in.name }`	| Iterate over a list and return results |
    | `for i in $(seq 1 10); do echo $i; done` | `for i in 1..10 { print $i }` | Iterate over a list and run a command on results |
    | `cp <source> <dest>`	| `cp <source> <dest>`	| Copy file to new location |
    | `rm -rf <path>`	| `rm -r <path>` |	Recursively removes the given path |
    | `date -d <date>`	| `"<date>" | into datetime -f <format>`	| Parse a date (format documentation) |
    | `sed`	| `str replace`	| Find and replace a pattern in a string | 
    | `grep <pattern>`	| `where $it =~ <substring>` or `find <substring>`	| Filter strings that contain the substring |
    | `command1 && command2` | `command1; command2`	| Run a command, and if it's successful run a second |
    | `stat $(which git)`	| `stat ...(which git).path`	| Use command output as argument for other command |
    | `echo /tmp/$RANDOM`	| `$"/tmp/(random int)"` |	Use command output in a string |
    | `cargo b --jobs=$(nproc)`	| `cargo b $"--jobs=(sys cpu | length)"` | Use command output in an option |
    | `echo $PATH`	| `$env.PATH (Non-Windows) or $env.Path (Windows)`	| See the current path |
    | `echo $?`	| `$env.LAST_EXIT_CODE`	| See the exit status of the last executed command |
    | `export` | `$env`	| List the current environment variables |
    | `FOO=BAR ./bin` | `FOO=BAR ./bin`	| Update environment for a command |
    | `echo $FOO` |	`$env.FOO` | Use environment variables |
    | `echo ${FOO:-fallback}` | `$env.FOO? | default "ABC"`	| Use a fallback in place of an unset variable |
    | `type FOO` | `which FOO` | Display information about a command (builtin, alias, or executable) |
    | `\` | `( <command> )`	| A command can span multiple lines when wrapped with ( and ) |

    If the polars commands are available, prefer it for working with parquet, jsonl, ndjson, csv files, and avro files. 
    It is much more efficient than the other Nushell commands or other non-nushell commands. 
    It exposes much of the functionality of the polars dataframe library. Start the pipeline with `plugin use polars`

    An example of converting a nushell table output to a polars dataframe:
    ```nu
    ps | polars into-df | polars collect
    ```

    An example of converting a polars dataframe back to a nushell table in order to run other nushell commands:
    ```nu
    polars open file.parquet | polars into-nu
    ````

    An example of opening a parquet file, selecting columns, and saving to a new parquet file:
    ```nu
    polars open file.parquet | polars select name status | polars save file2
    ```

    **Important**: The `glob` command should be used exclusively when you need to locate a file or a code reference,
    other solutions may produce too large output because of hidden files! For example *do not* use `find` or `ls -r`. 
    Use command_help tool to learn more about the `glob` command.

    **Important**: Each shell command runs in its own process. Things like directory changes or
    sourcing files do not persist between tool calls. So you may need to repeat them each time by
    stringing together commands, e.g. `cd example; ls` or `source env/bin/activate && pip install numpy`
    - Multiple commands: Use ; to chain commands, avoid newlines
    - Pathnames: Use absolute paths and avoid cd unless explicitly requested
    - Setting environment variables or other variables will not persist between calls 
    "#)]
    async fn evaluate(
        &self,
        Parameters(NuSourceRequest { input, cursor }): Parameters<NuSourceRequest>,
    ) -> Result<Json<EvalResult>, McpError> {
        self.evaluator.eval(&input, cursor).map(Json)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct ListCommandsRequest {
    #[schemars(description = "string to find in command names, descriptions, and search term")]
    find: Option<String>,
    #[schemars(description = "The cursor for the result of the page.")]
    cursor: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct CommandNameRequest {
    #[schemars(description = "The name of the command")]
    name: String,
    #[schemars(description = "The cursor for the result of the page.")]
    cursor: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct NuSourceRequest {
    #[schemars(description = "The Nushell source code to evaluate")]
    input: String,
    #[schemars(description = "The cursor for the result of the page.")]
    cursor: Option<usize>,
}

#[tool_handler]
impl ServerHandler for NushellMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(formatdoc! {r#"
            The nushell extension gives you run nushell specific commands and other shell commands. 
            This extension should be preferred over other tools for running shell commands as it can run both nushell comamands and other shell commands.

            Native nushell commands return structured content. Native nushell commands cam be discovered by using the list_commands tool. 
            Prefer nushell native commands where possible as they provided structured data in a pipeline, versus text output.
            To discover the input (stdin) and output (stdout) types of a command, flags, and positioanal arguments use the command_help tool.

            Nushell native commands will return structured content. Piping of commands that return a table, list, or record to `to text` will return plain text and `to json` will return json text. 
            In order to find out what columns are available use the `columns` command. For example `ps | columns` will return the columns available from the `ps` command.

            To find a nushell command or to see all available commands use the list_commands tool.
            To learn more about how to use a command, use the command_help tool.
            You can use the eval tool to run any command that would work on the relevant operating system.
            Use the eval tool as needed to locate files or interact with the project.
            "#
            }),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
