use crate::is_perf_true;
use crate::utils::{eval_source, report_error};
use log::info;
use nu_parser::ParseError;
use nu_path::canonicalize_with;
use nu_protocol::engine::{EngineState, Stack, StateDelta, StateWorkingSet};
use nu_protocol::{PipelineData, Span, Spanned};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

const NUSHELL_FOLDER: &str = "nushell";
const CONFIG_FILE: &str = "config.nu";
const HISTORY_FILE: &str = "history.txt";
#[cfg(feature = "plugin")]
const PLUGIN_FILE: &str = "plugin.nu";

#[cfg(feature = "plugin")]
pub(crate) fn read_plugin_file(engine_state: &mut EngineState, stack: &mut Stack) {
    // Reading signatures from signature file
    // The plugin.nu file stores the parsed signature collected from each registered plugin
    if let Some(mut plugin_path) = nu_path::config_dir() {
        // Path to store plugins signatures
        plugin_path.push(NUSHELL_FOLDER);
        plugin_path.push(PLUGIN_FILE);
        engine_state.plugin_signatures = Some(plugin_path.clone());

        let plugin_filename = plugin_path.to_string_lossy().to_owned();

        if let Ok(contents) = std::fs::read(&plugin_path) {
            eval_source(
                engine_state,
                stack,
                &contents,
                &plugin_filename,
                PipelineData::new(Span::new(0, 0)),
            );
        }
    }
    if is_perf_true() {
        info!("read_plugin_file {}:{}:{}", file!(), line!(), column!());
    }
}

pub(crate) fn read_config_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_file: Option<Spanned<String>>,
) {
    // Load config startup file
    if let Some(file) = config_file {
        let working_set = StateWorkingSet::new(engine_state);
        let cwd = working_set.get_cwd();

        match canonicalize_with(&file.item, cwd) {
            Ok(path) => {
                eval_config_contents(path, engine_state, stack);
            }
            Err(_) => {
                let e = ParseError::FileNotFound(file.item, file.span);
                report_error(&working_set, &e);
            }
        }
    } else if let Some(mut config_path) = nu_path::config_dir() {
        config_path.push(NUSHELL_FOLDER);

        // Create config directory if it does not exist
        if !config_path.exists() {
            if let Err(err) = std::fs::create_dir_all(&config_path) {
                eprintln!("Failed to create config directory: {}", err);
                return;
            }
        }

        config_path.push(CONFIG_FILE);

        if !config_path.exists() {
            println!("No config file found at {:?}", config_path);
            println!("Would you like to create one (Y/n): ");

            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .expect("Failed to read user input");

            match answer.to_lowercase().trim() {
                "y" => {
                    let mut output = File::create(&config_path).expect("Unable to create file");
                    write!(output, "{}", default_config_contents()).expect("Writing file contents");
                    println!("Config file created {:?}", config_path);
                }
                _ => {
                    println!("Continuing without config file");
                    return;
                }
            }
        }

        eval_config_contents(config_path, engine_state, stack);
    }

    if is_perf_true() {
        info!("read_config_file {}:{}:{}", file!(), line!(), column!());
    }
}

fn eval_config_contents(config_path: PathBuf, engine_state: &mut EngineState, stack: &mut Stack) {
    if config_path.exists() & config_path.is_file() {
        let config_filename = config_path.to_string_lossy().to_owned();

        if let Ok(contents) = std::fs::read(&config_path) {
            eval_source(
                engine_state,
                stack,
                &contents,
                &config_filename,
                PipelineData::new(Span::new(0, 0)),
            );

            // Merge the delta in case env vars changed in the config
            match nu_engine::env::current_dir(engine_state, stack) {
                Ok(cwd) => {
                    if let Err(e) = engine_state.merge_delta(StateDelta::new(), Some(stack), cwd) {
                        let working_set = StateWorkingSet::new(engine_state);
                        report_error(&working_set, &e);
                    }
                }
                Err(e) => {
                    let working_set = StateWorkingSet::new(engine_state);
                    report_error(&working_set, &e);
                }
            }
        }
    }
}

pub(crate) fn create_history_path() -> Option<PathBuf> {
    nu_path::config_dir().and_then(|mut history_path| {
        history_path.push(NUSHELL_FOLDER);
        history_path.push(HISTORY_FILE);

        if !history_path.exists() {
            // Creating an empty file to store the history
            match std::fs::File::create(&history_path) {
                Ok(_) => Some(history_path),
                Err(_) => None,
            }
        } else {
            Some(history_path)
        }
    })
}

fn default_config_contents() -> &'static str {
    r#"
# Nushell Config File

def create_left_prompt [] {
    let path_segment = ([
        ($nu.cwd)
        (char space)
    ] | str collect)

    $path_segment
}

def create_right_prompt [] {
    let time_segment = ([
        (date now | date format '%m/%d/%Y %I:%M:%S%.3f')
    ] | str collect)

    $time_segment
}

# Use nushell functions to define your right and left prompt
let-env PROMPT_COMMAND = { create_left_prompt }
let-env PROMPT_COMMAND_RIGHT = { create_right_prompt }

# The prompt indicators are environmental variables that represent
# the state of the prompt
let-env PROMPT_INDICATOR = "〉"
let-env PROMPT_INDICATOR_VI_INSERT = ": "
let-env PROMPT_INDICATOR_VI_NORMAL = "〉 "
let-env PROMPT_MULTILINE_INDICATOR = "::: "

let $config = {
  filesize_metric: $true
  table_mode: rounded # basic, compact, compact_double, light, thin, with_love, rounded, reinforced, heavy, none, other
  use_ls_colors: $true
  rm_always_trash: $false
  color_config: {
    separator: yd
    leading_trailing_space_bg: white
    header: cb
    date: pu
    filesize: ub
    row_index: yb
    hints: dark_gray
    bool: red
    int: green
    duration: red
    range: red
    float: red
    string: red
    nothing: red
    binary: red
    cellpath: red
  }
  use_grid_icons: $true
  footer_mode: always #always, never, number_of_rows, auto
  quick_completions: $false
  animate_prompt: $false
  float_precision: 2
  use_ansi_coloring: $true
  filesize_format: "b" # b, kb, kib, mb, mib, gb, gib, tb, tib, pb, pib, eb, eib, zb, zib, auto
  env_conversions: {
    "PATH": {
        from_string: { |s| $s | split row (char esep) }
        to_string: { |v| $v | str collect (char esep) }
    }
  }
  edit_mode: emacs # emacs, vi
  max_history_size: 10000
  log_level: error  # warn, error,  info, debug, trace
  menu_config: {
    columns: 4
    col_width: 20   # Optional value. If missing all the screen width is used to calculate column width
    col_padding: 2
    text_style: red
    selected_text_style: green_reverse
    marker: "| "
  }
  history_config: {
   page_size: 10
   selector: ":"                                                                                                                          
   text_style: green
   selected_text_style: green_reverse
   marker: "? "
  }
  keybindings: [
  {
    name: completion
    modifier: control
    keycode: char_t
    mode: vi_insert # emacs vi_normal vi_insert
    event: { send: menu name: context_menu }
  }
  ]
}
    "#
}
