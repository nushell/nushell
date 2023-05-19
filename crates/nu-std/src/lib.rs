use nu_parser::parse;
use nu_protocol::report_error;
use nu_protocol::{engine::StateWorkingSet, engine::VirtualPath};

// Virtual std directory unlikely to appear in user's file system
const NU_STD_VIRTUAL_DIR: &str = "NU_STD_VIRTUAL_DIR";

pub fn load_standard_library(
    engine_state: &mut nu_protocol::engine::EngineState,
) -> Result<(), miette::ErrReport> {
    let delta = {
        let std_files = vec![
            (
                format!("{NU_STD_VIRTUAL_DIR}/std/mod.nu"),
                include_str!("../std/mod.nu"),
            ),
            (
                format!("{NU_STD_VIRTUAL_DIR}/std/dirs.nu"),
                include_str!("../std/dirs.nu"),
            ),
            (
                format!("{NU_STD_VIRTUAL_DIR}/std/dt.nu"),
                include_str!("../std/dt.nu"),
            ),
            (
                format!("{NU_STD_VIRTUAL_DIR}/std/help.nu"),
                include_str!("../std/help.nu"),
            ),
            (
                format!("{NU_STD_VIRTUAL_DIR}/std/iter.nu"),
                include_str!("../std/iter.nu"),
            ),
            (
                format!("{NU_STD_VIRTUAL_DIR}/std/log.nu"),
                include_str!("../std/log.nu"),
            ),
            (
                format!("{NU_STD_VIRTUAL_DIR}/std/testing.nu"),
                include_str!("../std/testing.nu"),
            ),
            (
                format!("{NU_STD_VIRTUAL_DIR}/std/xml.nu"),
                include_str!("../std/xml.nu"),
            ),
        ];

        let mut working_set = StateWorkingSet::new(engine_state);
        let mut std_virt_paths = vec![];

        for (name, content) in std_files {
            let file_id = working_set.add_file(name.clone(), content.as_bytes());
            // TODO: Error on redefinition:
            let _ = working_set.add_virtual_path(name.clone(), VirtualPath::File(file_id));
            std_virt_paths.push((name, VirtualPath::File(file_id)));
        }

        let std_dir = format!("{NU_STD_VIRTUAL_DIR}/std");

        let source = format!(
            r#"
# Define the `std` module
module {std_dir}

# Prelude
use std dirs [ enter, shells, g, n, p, dexit ]
"#
        );

        // TODO: Error on redefinition:
        let _ = working_set.add_virtual_path(std_dir, VirtualPath::Dir(std_virt_paths));

        parse(
            &mut working_set,
            Some("loading stdlib"),
            source.as_bytes(),
            false,
        );

        if let Some(err) = working_set.parse_errors.first() {
            report_error(&working_set, err);
        }

        working_set.render()
    };

    engine_state.merge_delta(delta)?;

    Ok(())
}
