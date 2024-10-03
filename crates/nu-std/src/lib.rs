#![doc = include_str!("../README.md")]
use log::trace;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{FileStack, Stack, StateWorkingSet, VirtualPath},
    report_parse_error, PipelineData,
};
use std::path::PathBuf;

pub fn load_standard_library(
    engine_state: &mut nu_protocol::engine::EngineState,
) -> Result<(), miette::ErrReport> {
    trace!("load_standard_library");
    let (block, delta) = {
        let std_dir = PathBuf::from("std");

        let mut std_files = vec![
            // Loaded at startup
            ("core", include_str!("../std/core.nu")),
            // std module - Loads all commands and submodules
            ("mod.nu", include_str!("../std/mod.nu")),
            // std submodules
            ("assert", include_str!("../std/assert.nu")),
            ("bench", include_str!("../std/bench.nu")),
            ("dirs", include_str!("../std/dirs.nu")),
            ("dt", include_str!("../std/dt.nu")),
            ("formats", include_str!("../std/formats.nu")),
            ("help", include_str!("../std/help.nu")),
            ("input", include_str!("../std/input.nu")),
            ("iter", include_str!("../std/iter.nu")),
            ("log", include_str!("../std/log.nu")),
            ("math", include_str!("../std/math.nu")),
            ("lib", include_str!("../std/lib.nu")),
            ("xml", include_str!("../std/xml.nu")),
            // Remove in following release
            ("deprecated_dirs", include_str!("../std/deprecated_dirs.nu")),
        ];

        let mut working_set = StateWorkingSet::new(engine_state);
        let mut std_virt_paths = vec![];

        for (name, content) in std_files.drain(..) {
            let name = std_dir.join(name);

            let file_id =
                working_set.add_file(name.to_string_lossy().to_string(), content.as_bytes());
            let virtual_file_id = working_set.add_virtual_path(
                name.to_string_lossy().to_string(),
                VirtualPath::File(file_id),
            );
            std_virt_paths.push(virtual_file_id);
        }

        let std_dir = std_dir.to_string_lossy().to_string();
        let source = r#"
# Prelude
use std/core *
use std/deprecated_dirs [
    enter
    shells
    g
    n
    p
    dexit
]
"#;

        let _ = working_set.add_virtual_path(std_dir, VirtualPath::Dir(std_virt_paths));

        // Add a placeholder file to the stack of files being evaluated.
        // The name of this file doesn't matter; it's only there to set the current working directory to NU_STDLIB_VIRTUAL_DIR.
        let placeholder = PathBuf::from("load std/core");
        working_set.files = FileStack::with_file(placeholder);

        let block = parse(
            &mut working_set,
            Some("loading stdlib"),
            source.as_bytes(),
            false,
        );

        // Remove the placeholder file from the stack of files being evaluated.
        working_set.files.pop();

        if let Some(err) = working_set.parse_errors.first() {
            report_parse_error(&working_set, err);
        }

        (block, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    // We need to evaluate the module in order to run the `export-env` blocks.
    let mut stack = Stack::new();
    let pipeline_data = PipelineData::Empty;

    eval_block::<WithoutDebug>(engine_state, &mut stack, &block, pipeline_data)?;

    engine_state.merge_env(&mut stack)?;

    Ok(())
}
