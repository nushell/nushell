use std::sync::Arc;

use nu_engine::eval_block;
use nu_parser::parse;
use nu_plugin::{get_signature, PersistentPlugin, PluginCommand, PluginDeclaration};
use nu_plugin_polars::dataframe::eager::ToDataFrame;
use nu_plugin_polars::PolarsDataFramePlugin;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    PipelineData, PluginExample, PluginGcConfig, PluginIdentity, RegisteredPlugin, Span,
};

use nu_protocol::debugger::WithoutDebug;

pub fn test_dataframe(cmds: Vec<Box<dyn PluginCommand<Plugin = PolarsDataFramePlugin> + 'static>>) {
    if cmds.is_empty() {
        panic!("Empty commands vector")
    }

    // The first element in the cmds vector must be the one tested
    let examples = cmds[0].signature().examples;
    let (plugin_identity, persistent_plugin, registered_plugin) = build_plugin();
    let mut engine_state = build_test_engine_state(
        plugin_identity,
        Arc::clone(&persistent_plugin),
        Arc::clone(&registered_plugin),
    );

    for example in examples {
        test_dataframe_example(&mut engine_state, &example);
    }
}

pub fn build_plugin() -> (
    PluginIdentity,
    Arc<PersistentPlugin>,
    Arc<dyn RegisteredPlugin>,
) {
    let identity = PluginIdentity::new("../../target/debug/nu_plugin_polars", None)
        .expect("Error creating PluginIdentity");
    let gc_config = PluginGcConfig {
        enabled: false,
        stop_after: 0,
    };
    let persistent_plugin = Arc::new(PersistentPlugin::new(identity.clone(), gc_config.clone()));
    let registered_plugin: Arc<dyn RegisteredPlugin> =
        Arc::new(PersistentPlugin::new(identity.clone(), gc_config));

    (identity, persistent_plugin, registered_plugin)
}

pub fn build_test_engine_state(
    identity: PluginIdentity,
    persistent_plugin: Arc<PersistentPlugin>,
    registered_plugin: Arc<dyn RegisteredPlugin>,
) -> Box<EngineState> {
    let mut engine_state = Box::new(EngineState::new());

    let get_envs = || {
        let stack = Stack::new().capture();
        nu_engine::env::env_to_strings(&engine_state, &stack)
    };

    let signatures = get_signature(Arc::clone(&persistent_plugin), get_envs)
        .expect("should be able to get plugin signature");

    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.find_or_create_plugin(&identity, || Arc::clone(&registered_plugin));

        for signature in signatures {
            let plugin_decl = PluginDeclaration::new(&persistent_plugin, signature);
            working_set.add_decl(Box::new(plugin_decl));
        }
        working_set.render()
    };

    engine_state
        .merge_delta(delta)
        .expect("Error merging delta");

    engine_state
}

pub fn test_dataframe_example(engine_state: &mut Box<EngineState>, example: &PluginExample) {
    // Skip tests that don't have results to compare to
    if example.result.is_none() {
        return;
    }

    let start = std::time::Instant::now();

    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let output = parse(&mut working_set, None, example.example.as_bytes(), false);

        if let Some(err) = working_set.parse_errors.first() {
            panic!("test parse error in `{}`: {:?}", example.example, err)
        }

        (output, working_set.render())
    };

    engine_state
        .merge_delta(delta)
        .expect("Error merging delta");

    let mut stack = Stack::new();

    let result =
        eval_block::<WithoutDebug>(engine_state, &mut stack, &block, PipelineData::empty())
            .unwrap_or_else(|err| panic!("test eval error in `{}`: {:?}", example.example, err))
            .into_value(Span::test_data());

    println!("input: {}", example.example);
    println!("result: {result:?}");
    println!("done: {:?}", start.elapsed());

    // Note. Value implements PartialEq for Bool, Int, Float, String and Block
    // If the command you are testing requires to compare another case, then
    // you need to define its equality in the Value struct
    if let Some(expected) = example.result.clone() {
        if result != expected {
            panic!("the example result is different to expected value: {result:?} != {expected:?}")
        }
    }
}

#[test]
//#[ignore = "not yet working"]
fn test_into_df() {
    test_dataframe(vec![Box::new(ToDataFrame {})])
}
