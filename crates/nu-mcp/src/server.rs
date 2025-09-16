use std::{
    borrow::Cow,
    sync::{Mutex, MutexGuard},
};

use indoc::formatdoc;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    debugger::WithoutDebug, engine::{EngineState, Stack, StateWorkingSet}, write_all_and_flush, PipelineData, PipelineExecutionData, Value
};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::{Annotated, CallToolResult, Content, RawContent, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct NushellMcpServer {
    engine_state: Mutex<EngineState>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl NushellMcpServer {
    pub fn new(engine_state: EngineState) -> Self {
        NushellMcpServer {
            engine_state: Mutex::new(engine_state),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List available Nushell native commands.")]
    async fn list_commands(&self) -> Result<CallToolResult, McpError> {
        self.eval("help commands | select name description | where not ($it.name | str starts-with _) | to json", PipelineData::empty())
            .map(CallToolResult::success)
    }

    #[tool(
        description = "Find a nushell command by searching command names, descriptions, and search terms"
    )]
    async fn find_command(
        &self,
        Parameters(CommandNameRequest { name: query }): Parameters<CommandNameRequest>,
    ) -> Result<CallToolResult, McpError> {
        let cmd = format!(
            "help commands --find {query}| where not ($it.name | str starts-with _) | to json"
        );
        self.eval(&cmd, PipelineData::empty())
            .map(CallToolResult::success)
    }

    #[tool(
        description = "Get help for a specific Nushell command. This will only work on commands that are native to nushell. To find out if a command is native to nushell you can use the find_command tool."
    )]
    async fn command_help(
        &self,
        Parameters(CommandNameRequest { name }): Parameters<CommandNameRequest>,
    ) -> Result<CallToolResult, McpError> {
        let cmd = format!("help {name}");
        self.eval(&cmd, PipelineData::empty())
            .map(CallToolResult::success)
    }

    #[tool(description = r#"Execute a command in the nushell.
 
This will return the output and error concatenated into a single string, as
you would see from running on the command line. There will also be an indication
of if the command succeeded or failed.

Prefer nushell native commands where possible as they work within a nushell pipeline as oppposed to convering the pipeline to text.

To find a nushell command, use the find_command tool, or list all commands with the list_commands tool.

To learn more about how to use a command, use the command_help tool.

Avoid commands that produce a large amount of output, and consider piping those outputs to files.
If you need to run a long lived command, background it - e.g. `uvicorn main:app &` so that
this tool does not run indefinitely.

Nushell specific commands will return a nushell table. Piping these commands to `to text` will return text and `to json` will return JSON.

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
        Parameters(NuSourceRequest { input }): Parameters<NuSourceRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.eval(&input, PipelineData::empty())
            .map(CallToolResult::success)
    }

    fn eval(&self, nu_source: &str, input: PipelineData) -> Result<Vec<Content>, McpError> {
        let mut engine_state = self.engine_state_lock()?;
        let mut working_set = StateWorkingSet::new(&engine_state);

        // Parse the source code
        let block = parse(&mut working_set, None, nu_source.as_bytes(), false);

        // Check for parse errors
        if !working_set.parse_errors.is_empty() {
            // ShellError doesn't have ParseError, use LabeledError to contain it.
            return Err(McpError::invalid_request(
                "Failed to parse nushell pipeline",
                None,
            ));
        }

        let rendered = working_set.render();

        // Merge into state
        engine_state
            .merge_delta(rendered)
            .map_err(shell_error_to_mcp_error)?;

        // Eval the block with the input
        let mut stack = Stack::new().collect_value();
        let output = eval_block::<WithoutDebug>(&engine_state, &mut stack, &block, input)
            .map_err(shell_error_to_mcp_error)?;

        pipeline_to_content(output, &engine_state)
            .map(|content| vec![content])
            .map_err(|e| McpError::internal_error(format!("Failed to evaluate block: {e}"), None))
    }

    fn engine_state_lock(&self) -> Result<MutexGuard<EngineState>, McpError> {
        self.engine_state.lock().map_err(|e| {
            McpError::internal_error(format!("Failed to acquire engine state lock: {e}"), None)
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct QueryRequest {
    #[schemars(description = "string to find in command names, descriptions, and search terms")]
    query: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct CommandNameRequest {
    #[schemars(description = "The name of the command")]
    name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
struct NuSourceRequest {
    #[schemars(description = "The Nushell source code to evaluate")]
    input: String,
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

fn shell_error_to_mcp_error(error: nu_protocol::ShellError) -> McpError {
    //todo - make this more sophisticated   :w
    McpError::internal_error(format!("ShellError: {error}"), None)
}

fn pipeline_to_content(
    pipeline_execution_data: PipelineExecutionData,
    engine_state: &EngineState,
) -> Result<Content, McpError> {
    let span = pipeline_execution_data.span();
    // todo - this bystream use case won't work
    if let PipelineData::ByteStream(_stream, ..) = pipeline_execution_data.body {
        // Copy ByteStreams directly
        // stream.print(false)
        Err(McpError::internal_error(
            Cow::from("ByteStream output is not supported"),
            None,
        ))
    } else {
        let mut buffer: Vec<u8> = Vec::new();
        let config = engine_state.get_config();
        for item in pipeline_execution_data.body {
            let out = if let Value::Error { error, .. } = item {
                return Err(shell_error_to_mcp_error(*error));
            } else {
                item.to_expanded_string("\n", config)
            };

            write_all_and_flush(out, &mut buffer, "mcp_output", span, engine_state.signals())
                .map_err(shell_error_to_mcp_error)?;
        }
        let content =
            RawContent::text(String::from_utf8(buffer).map_err(|e| {
                McpError::internal_error(format!("Invalid UTF-8 output: {e}"), None)
            })?);
        Ok(Annotated::new(content, None))
    }
}
