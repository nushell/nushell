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
        if let Some(config) = Arc::get_mut(&mut engine_state.config) {
            config.use_ansi_coloring = UseAnsiColoring::False;
            config.color_config.clear();
        }
        let engine_state = Arc::new(engine_state);
        NushellMcpServer {
            tool_router: Self::tool_router(),
            evaluator: Evaluator::new(engine_state),
        }
    }

    #[tool(description = "List available Nushell native commands.")]
    async fn list_commands(
        &self,
        Parameters(ListCommandsRequest { cursor }): Parameters<ListCommandsRequest>,
    ) -> Result<Json<EvalResult>, McpError> {
        self.evaluator.eval("help commands", cursor).map(Json)
    }

    #[tool(
        description = "Find a nushell command by searching command names, descriptions, and search terms"
    )]
    async fn find_command(
        &self,
        Parameters(CommandNameRequest {
            name: query,
            cursor,
        }): Parameters<CommandNameRequest>,
    ) -> Result<Json<EvalResult>, McpError> {
        let cmd = format!("help commands --find {query}");
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

Prefer nushell native commands where possible as they work within a nushell pipeline as oppposed to converting the pipeline to text.

To find a nushell command, use the find_command tool, or list all commands with the list_commands tool.

To learn more about how to use a command, use the command_help tool.

Avoid commands that produce a large amount of output, and consider piping those outputs to files.
If you need to run a long lived command, background it - e.g. `uvicorn main:app &` so that
this tool does not run indefinitely.

Nushell specific commands will return a nushell table. Piping these commands to `to text` will return text and `to json` will return JSON. 
In order to find out what columns are available use the `columns` command. For example `ps | columns` will return the columns available from the `ps` command.

If the polars command is available, prefer it for working with parquet, jsonl, ndjson, csv files, and avro files. It is much more efficient than the default Nushell commands or other non-nushell commands. It exposes much of the functionality of the polars dataframe library. The polars command has sub commands for opening and saving these file types via `polars open` and `polars save`, do not use the `open` and `save` command for these file types. When working with polars run all commands within a single pipeline if possible (e.g `polars open file.parquet | polars select name status | polars save file2.parquet`). The command `polars collect` must be run in order to collect the data into a table, otherwise it will return a lazy frame which is not useful for display purposes. When saving output to a file the entire pipeline must be run in one command. `polars collect` is not needed when saving to a file, as the file will be written directly from the lazy frame.

**Important**: Use ripgrep - `rg` - exclusively when you need to locate a file or a code reference,
other solutions may produce too large output because of hidden files! For example *do not* use `find` or `ls -r`
- List files by name: `rg --files | rg <filename>`
- List files that contain a regex: `rg '<regex>' -l`

**Important**: Each shell command runs in its own process. Things like directory changes or
sourcing files do not persist between tool calls. So you may need to repeat them each time by
stringing together commands, e.g. `cd example; ls` or `source env/bin/activate && pip install numpy`
- Multiple commands: Use ; to chain commands, avoid newlines
- Pathnames: Use absolute paths and avoid cd unless explicitly requested
"#)]
    async fn evaluate(
        &self,
        Parameters(NuSourceRequest { input, cursor }): Parameters<NuSourceRequest>,
    ) -> Result<Json<EvalResult>, McpError> {
        self.evaluator.eval(&input, cursor).map(Json)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct QueryRequest {
    #[schemars(description = "string to find in command names, descriptions, and search terms")]
    query: String,
    #[schemars(description = "The cursor for the result of the page.")]
    cursor: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct ListCommandsRequest {
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

            You can use the eval tool to run any command that would work on the relevant operating system.
            Use the eval tool as needed to locate files or interact with the project.
            "#
            }),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
