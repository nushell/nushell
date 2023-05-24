use std::path::PathBuf;

use nu_engine::{env::current_dir, eval_block};
use nu_parser::parse;
use nu_protocol::engine::{Stack, StateWorkingSet, VirtualPath};
use nu_protocol::{report_error, PipelineData};

// Virtual std directory unlikely to appear in user's file system
const NU_STDLIB_VIRTUAL_DIR: &str = "NU_STDLIB_VIRTUAL_DIR";

pub fn load_standard_library(
    engine_state: &mut nu_protocol::engine::EngineState,
) -> Result<(), miette::ErrReport> {
    let (block, delta) = {
        let mut std_files = vec![
            (
                PathBuf::from(NU_STDLIB_VIRTUAL_DIR)
                    .join("std")
                    .join("mod.nu"),
                include_str!("../std/mod.nu"),
            ),
            (
                PathBuf::from(NU_STDLIB_VIRTUAL_DIR)
                    .join("std")
                    .join("dirs.nu"),
                include_str!("../std/dirs.nu"),
            ),
            (
                PathBuf::from(NU_STDLIB_VIRTUAL_DIR)
                    .join("std")
                    .join("dt.nu"),
                include_str!("../std/dt.nu"),
            ),
            (
                PathBuf::from(NU_STDLIB_VIRTUAL_DIR)
                    .join("std")
                    .join("help.nu"),
                include_str!("../std/help.nu"),
            ),
            (
                PathBuf::from(NU_STDLIB_VIRTUAL_DIR)
                    .join("std")
                    .join("iter.nu"),
                include_str!("../std/iter.nu"),
            ),
            (
                PathBuf::from(NU_STDLIB_VIRTUAL_DIR)
                    .join("std")
                    .join("log.nu"),
                include_str!("../std/log.nu"),
            ),
            (
                PathBuf::from(NU_STDLIB_VIRTUAL_DIR)
                    .join("std")
                    .join("testing.nu"),
                include_str!("../std/testing.nu"),
            ),
            (
                PathBuf::from(NU_STDLIB_VIRTUAL_DIR)
                    .join("std")
                    .join("xml.nu"),
                include_str!("../std/xml.nu"),
            ),
        ];

        let mut working_set = StateWorkingSet::new(engine_state);
        let mut std_virt_paths = vec![];

        for (name, content) in std_files.drain(..) {
            let file_id =
                working_set.add_file(name.to_string_lossy().to_string(), content.as_bytes());
            let virtual_file_id = working_set.add_virtual_path(
                name.to_string_lossy().to_string(),
                VirtualPath::File(file_id),
            );
            std_virt_paths.push(virtual_file_id);
        }

        // Using full virtual path to avoid potential conflicts with user having 'std' directory
        // in their working directory.
        let std_dir = PathBuf::from(NU_STDLIB_VIRTUAL_DIR)
            .join("std")
            .to_string_lossy()
            .to_string();
        let source = format!(
            r#"
# Define the `std` module
module {std_dir}

# Prelude
use std dirs [ enter, shells, g, n, p, dexit ]
"#
        );

        let _ = working_set.add_virtual_path(std_dir, VirtualPath::Dir(std_virt_paths));

        // Change the currently parsed directory
        let prev_currently_parsed_cwd = working_set.currently_parsed_cwd.clone();
        working_set.currently_parsed_cwd = Some(PathBuf::from(NU_STDLIB_VIRTUAL_DIR));

        let block = parse(
            &mut working_set,
            Some("loading stdlib"),
            source.as_bytes(),
            false,
        );

        if let Some(err) = working_set.parse_errors.first() {
            report_error(&working_set, err);
        }

        // Restore the currently parsed directory back
        working_set.currently_parsed_cwd = prev_currently_parsed_cwd;

        (block, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    // We need to evaluate the module in order to run the `export-env` blocks.
    let mut stack = Stack::new();
    let pipeline_data = PipelineData::Empty;
    eval_block(
        engine_state,
        &mut stack,
        &block,
        pipeline_data,
        false,
        false,
    )?;

    let cwd = current_dir(engine_state, &stack)?;
    engine_state.merge_env(&mut stack, cwd)?;

    Ok(())
}
