use std::{borrow::Cow, sync::Arc};

use futures::{future::BoxFuture, FutureExt};
use nu_protocol::{
    engine::{Command, EngineState, Stack}, PipelineData, ShellError, Value
};
use rmcp::{
    ServerHandler,
    handler::server::tool::{CallToolHandler, ToolCallContext, ToolRoute, ToolRouter},
    model::{CallToolResult, ServerCapabilities, ServerInfo, Tool, ToolAnnotations},
    tool_handler, tool_router,
};

use crate::schema::json_schema_signature;

pub struct NushellMcpServer {
    engine_state: Arc<EngineState>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl NushellMcpServer {
    pub fn new(engine_state: EngineState) -> Self {
        NushellMcpServer {
            engine_state: Arc::new(engine_state),
            tool_router: Self::tool_router(),
        }
    }

    fn build_tool_route(command: &dyn Command) -> ToolRoute<Self> {
        let tool = command_to_tool(command).expect("Failed to convert command to tool");
        todo!()
    }
}

#[tool_handler]
impl ServerHandler for NushellMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("generic data service".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

fn command_to_tool(command: &dyn Command) -> Result<Tool, ShellError> {
    let input_schema = json_schema_signature(&command.signature())?;
    Ok(Tool {
        name: Cow::Owned(command.name().to_owned()),
        description: Some(Cow::Owned(command.description().to_owned())),
        input_schema: Arc::new(rmcp::model::object(input_schema.into())),
        annotations: None,
    })
}

struct CommandCallTooHandler {
    engine_state: Arc<EngineState>,
    command: Arc<dyn Command>,
}

impl CallToolHandler<NushellMcpServer, ToolAnnotations> for CommandCallTooHandler {
    fn call(
        self,
        context: ToolCallContext<'_, NushellMcpServer>,
    ) -> BoxFuture<'_, Result<CallToolResult, rmcp::ErrorData>> {
        async {
            let stack = Stack::default(); // todo - make this more sophisticated
            // this isnt' going to work as it.. The Call object needs to take flags.
            // we need to retrofit the calls to flags
            // we also need to figure out how to handle not flagged args.
            let pipeline_input = context
                .arguments
                .map(|map_args| serde_json::Value::Object(map_args))
                .map(|args| {
                    serde_json::from_value::<Value>(args.clone())
                        .map_err(|e| rmcp::ErrorData::invalid_request(format!("{e}"), Some(args)))
                })
                .transpose()?
                .map(|v| PipelineData::Value(v, None))
                .unwrap_or_else(|| PipelineData::empty());


            self.command.run(
                &self.engine_state,
                &stack,
            ).map_err(|e| rmcp::ErrorData::internal_error(format!("{e}")))?;

            todo!()
        }.boxed()
    }
    // fn call_tool(
    //     &self,
    //     context: ToolCallContext<NushellMcpServer>,
    // ) -> BoxFuture<'_, Result<CallToolResult, rmcp::ErrorData>> {
    //     let command = self.command.clone();
    //     context.invoke(move |ctx| {
    //         let engine_state = ctx.server.engine_state.clone();
    //         command.run(engine_state, ctx.request).boxed()
    //     })
    // }
}
