use crate::{
    ast::{
        Argument, Block, Call, Expr, Expression, ExternalArgument, Keyword, ListItem, PathMember,
        Pipeline, PipelineElement, RecordItem, Table,
    },
    debugger::{Debugger, NoopDebugger},
    engine::{
        description::{build_desc, Doccomments},
        CachedFile, Command, CommandType, EnvVars, OverlayFrame, ScopeFrame, Stack, StateDelta,
        Variable, Visibility, DEFAULT_OVERLAY_NAME,
    },
    eval_const::create_nu_constant,
    shell_error::io::IoError,
    BlockId, Category, Config, DeclId, FileId, GetSpan, Handlers, HistoryConfig, Module, ModuleId,
    OverlayId, ParsedKeybinding, ParsedMenu, ShellError, SignalAction, Signals, Signature, Span,
    SpanId, Type, Value, VarId, VirtualPathId,
};
use fancy_regex::Regex;
use lru::LruCache;
use nu_path::AbsolutePathBuf;
use nu_utils::IgnoreCaseExt;
use std::{
    collections::HashMap,
    num::NonZeroUsize,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex, MutexGuard, PoisonError,
    },
};

type PoisonDebuggerError<'a> = PoisonError<MutexGuard<'a, Box<dyn Debugger>>>;

#[cfg(feature = "plugin")]
use crate::{PluginRegistryFile, PluginRegistryItem, RegisteredPlugin};

use super::{Jobs, ThreadJob};

#[derive(Clone, Debug)]
pub enum VirtualPath {
    File(FileId),
    Dir(Vec<VirtualPathId>),
}

pub struct ReplState {
    pub buffer: String,
    // A byte position, as `EditCommand::MoveToPosition` is also a byte position
    pub cursor_pos: usize,
}

pub struct IsDebugging(AtomicBool);

impl IsDebugging {
    pub fn new(val: bool) -> Self {
        IsDebugging(AtomicBool::new(val))
    }
}

impl Clone for IsDebugging {
    fn clone(&self) -> Self {
        IsDebugging(AtomicBool::new(self.0.load(Ordering::Relaxed)))
    }
}

/// The core global engine state. This includes all global definitions as well as any global state that
/// will persist for the whole session.
///
/// Declarations, variables, blocks, and other forms of data are held in the global state and referenced
/// elsewhere using their IDs. These IDs are simply their index into the global state. This allows us to
/// more easily handle creating blocks, binding variables and callsites, and more, because each of these
/// will refer to the corresponding IDs rather than their definitions directly. At runtime, this means
/// less copying and smaller structures.
///
/// Many of the larger objects in this structure are stored within `Arc` to decrease the cost of
/// cloning `EngineState`. While `Arc`s are generally immutable, they can be modified using
/// `Arc::make_mut`, which automatically clones to a new allocation if there are other copies of
/// the `Arc` already in use, but will let us modify the `Arc` directly if we have the only
/// reference to it.
///
/// Note that the runtime stack is not part of this global state. Runtime stacks are handled differently,
/// but they also rely on using IDs rather than full definitions.
#[derive(Clone)]
pub struct EngineState {
    files: Vec<CachedFile>,
    pub(super) virtual_paths: Vec<(String, VirtualPath)>,
    vars: Vec<Variable>,
    decls: Arc<Vec<Box<dyn Command + 'static>>>,
    // The Vec is wrapped in Arc so that if we don't need to modify the list, we can just clone
    // the reference and not have to clone each individual Arc inside. These lists can be
    // especially long, so it helps
    pub(super) blocks: Arc<Vec<Arc<Block>>>,
    pub(super) modules: Arc<Vec<Arc<Module>>>,
    pub spans: Vec<Span>,
    doccomments: Doccomments,
    pub scope: ScopeFrame,
    signals: Signals,
    pub signal_handlers: Option<Handlers>,
    pub env_vars: Arc<EnvVars>,
    pub previous_env_vars: Arc<HashMap<String, Value>>,
    pub config: Arc<Config>,
    pub pipeline_externals_state: Arc<(AtomicU32, AtomicU32)>,
    pub repl_state: Arc<Mutex<ReplState>>,
    pub table_decl_id: Option<DeclId>,
    #[cfg(feature = "plugin")]
    pub plugin_path: Option<PathBuf>,
    #[cfg(feature = "plugin")]
    plugins: Vec<Arc<dyn RegisteredPlugin>>,
    config_path: HashMap<String, PathBuf>,
    pub history_enabled: bool,
    pub history_session_id: i64,
    // Path to the file Nushell is currently evaluating, or None if we're in an interactive session.
    pub file: Option<PathBuf>,
    pub regex_cache: Arc<Mutex<LruCache<String, Regex>>>,
    pub is_interactive: bool,
    pub is_login: bool,
    startup_time: i64,
    is_debugging: IsDebugging,
    pub debugger: Arc<Mutex<Box<dyn Debugger>>>,

    pub jobs: Arc<Mutex<Jobs>>,

    // The job being executed with this engine state, or None if main thread
    pub current_thread_job: Option<ThreadJob>,

    // When there are background jobs running, the interactive behavior of `exit` changes depending on
    // the value of this flag:
    // - if this is false, then a warning about running jobs is shown and `exit` enables this flag
    // - if this is true, then `exit` will `std::process::exit`
    //
    // This ensures that running exit twice will terminate the program correctly
    pub exit_warning_given: Arc<AtomicBool>,
}

// The max number of compiled regexes to keep around in a LRU cache, arbitrarily chosen
const REGEX_CACHE_SIZE: usize = 100; // must be nonzero, otherwise will panic

pub const NU_VARIABLE_ID: VarId = VarId::new(0);
pub const IN_VARIABLE_ID: VarId = VarId::new(1);
pub const ENV_VARIABLE_ID: VarId = VarId::new(2);
// NOTE: If you add more to this list, make sure to update the > checks based on the last in the list

// The first span is unknown span
pub const UNKNOWN_SPAN_ID: SpanId = SpanId::new(0);

/// Helper function to calculate the memory size of a Value recursively with exact measurements
pub fn calculate_value_size(value: &Value) -> usize {
    let base_size = std::mem::size_of::<Value>();

    // Add additional heap allocations based on the variant
    match value {
        Value::String { val, .. } => {
            // String has a pointer, length, and capacity on the stack
            // The actual string data is on the heap
            base_size + val.capacity()
        }
        Value::Glob { val, .. } => {
            // Similar to String
            base_size + val.capacity()
        }
        Value::List { vals, .. } => {
            // Vec has a pointer, length, and capacity on the stack
            // The elements are on the heap
            let vec_overhead = std::mem::size_of::<Vec<Value>>();

            // Space for the Vec's allocation
            let vec_allocation = vals.capacity() * std::mem::size_of::<Value>();

            // Recursively calculate the size of each element
            let elements_size = vals.iter().map(calculate_value_size).sum::<usize>();

            base_size + vec_overhead + vec_allocation + elements_size
        }
        Value::Record { val, .. } => {
            // Record is a wrapper around a HashMap
            // Get the size of the Record struct itself
            let record_overhead = std::mem::size_of_val(val);

            // Calculate the size of all entries (keys and values)
            let entries_size = val
                .iter()
                .map(|(k, v)| {
                    // Each key is a String with its own heap allocation
                    let key_size = std::mem::size_of::<String>() + k.capacity();

                    // Recursively calculate the size of each value
                    let value_size = calculate_value_size(v);

                    key_size + value_size
                })
                .sum::<usize>();

            // Add HashMap overhead (buckets, etc.) - estimate based on typical HashMap implementation
            let hashmap_overhead = 64 + val.len() * 16; // Base overhead + per-entry overhead

            base_size + record_overhead + entries_size + hashmap_overhead
        }
        Value::Binary { val, .. } => {
            // Binary is a Vec<u8>
            base_size + std::mem::size_of::<Vec<u8>>() + val.capacity()
        }
        Value::Closure { val, .. } => {
            // Closure has a Box<Closure> which points to heap data
            let closure_overhead = std::mem::size_of_val(&**val);

            // Calculate the size of captures
            let captures_overhead = std::mem::size_of::<Vec<Value>>();
            let captures_allocation = val.captures.capacity() * std::mem::size_of::<Value>();

            // Calculate size of captures - need to handle the tuple structure
            let mut captures_size = 0;
            for (_, value) in &val.captures {
                captures_size += calculate_value_size(value);
            }

            // Calculate the size of the block_id
            let block_id_size = std::mem::size_of::<BlockId>();

            base_size
                + closure_overhead
                + captures_overhead
                + captures_allocation
                + captures_size
                + block_id_size
        }
        Value::Error { error, .. } => {
            // Error has a Box<ShellError> which points to heap data
            let error_overhead = std::mem::size_of_val(&**error);

            // ShellError can contain Values, so we need to account for those
            let error_data_size = match &**error {
                ShellError::GenericError {
                    error,
                    msg,
                    span: _,
                    help,
                    inner,
                } => {
                    error.len()
                        + msg.len()
                        + help.as_ref().map_or(0, |h| h.len())
                        + inner.iter().map(std::mem::size_of_val).sum::<usize>()
                }
                // Handle other ShellError variants similarly
                _ => std::mem::size_of::<ShellError>(), // Minimum size for other variants
            };

            base_size + error_overhead + error_data_size
        }
        Value::CellPath { val, .. } => {
            // CellPath has a Vec<PathMember>
            let cellpath_overhead = std::mem::size_of_val(val);
            let members_overhead = std::mem::size_of::<Vec<PathMember>>();
            let members_allocation = val.members.capacity() * std::mem::size_of::<PathMember>();

            // Calculate the size of each PathMember
            let members_size = val
                .members
                .iter()
                .map(|m| match m {
                    PathMember::String { val, .. } => {
                        std::mem::size_of::<PathMember>() + val.capacity()
                    }
                    _ => std::mem::size_of::<PathMember>(),
                })
                .sum::<usize>();

            base_size + cellpath_overhead + members_overhead + members_allocation + members_size
        }
        Value::Range { val, .. } => {
            // Range has a Box<Range> which points to heap data
            let range_overhead = std::mem::size_of_val(&**val);

            // Range contains start, end, and inclusive fields
            // We can't access them directly, so estimate based on typical Range implementation
            let range_fields_size = 32; // Reasonable estimate for Range fields

            base_size + range_overhead + range_fields_size
        }
        Value::Custom { val, .. } => {
            // Custom values are opaque, but we can measure the Box overhead
            let custom_overhead = std::mem::size_of_val(&**val);

            // We can't know the exact size of the custom data, so we'll use a reasonable estimate
            // based on the type's memory footprint
            let custom_data_size = 256; // Reasonable estimate for most custom types

            base_size + custom_overhead + custom_data_size
        }
        Value::Bool { .. } => base_size,
        Value::Int { .. } => base_size,
        Value::Float { .. } => base_size,
        Value::Filesize { .. } => base_size,
        Value::Duration { .. } => base_size,
        Value::Date { .. } => base_size,
        Value::Nothing { .. } => base_size,
    }
}

impl EngineState {
    /// Get the memory size of the files collection, including both stack and heap allocations
    pub fn files_memory_size(&self) -> (usize, usize) {
        let stack_size = std::mem::size_of::<Vec<CachedFile>>();
        let mut heap_size = 0;

        // Size of the Vec itself on the heap (capacity * element size)
        heap_size += self.files.capacity() * std::mem::size_of::<CachedFile>();

        // Calculate heap size of all files
        for file in &self.files {
            // Size of Arc<str> for name (pointer + length + capacity)
            heap_size += std::mem::size_of::<Arc<str>>() + file.name.len();

            // Size of Arc<[u8]> for content (pointer + length + capacity)
            heap_size += std::mem::size_of::<Arc<[u8]>>() + file.content.len();

            // Span is already counted in the CachedFile struct size
        }

        (stack_size, heap_size)
    }

    /// Get the memory size of the vars collection, including both stack and heap allocations
    pub fn vars_memory_size(&self) -> (usize, usize) {
        let stack_size = std::mem::size_of::<Vec<Variable>>();
        let mut heap_size = 0;

        // Size of the Vec itself on the heap
        // This includes the pointer, length, and capacity fields
        let vec_overhead = std::mem::size_of::<Vec<Variable>>();

        // Size of the Vec's allocation (capacity * element size)
        let vec_allocation = self.vars.capacity() * std::mem::size_of::<Variable>();
        heap_size += vec_overhead + vec_allocation;

        // Calculate heap size of all variables
        for var in &self.vars {
            // Variable struct contains a Span and potentially a Value
            // Span is already counted in the Variable struct size

            // If there's a const_val, add its size recursively
            if let Some(val) = &var.const_val {
                // For Value, we need to account for both the enum size and any heap allocations
                heap_size += calculate_value_size(val);
            }
        }

        (stack_size, heap_size)
    }

    /// Get the memory size of the decls collection, including both stack and heap allocations
    pub fn decls_memory_size(&self) -> (usize, usize) {
        let stack_size = std::mem::size_of::<Arc<Vec<Box<dyn Command + 'static>>>>();
        let mut heap_size = 0;

        // Size of the Arc itself
        heap_size += std::mem::size_of::<Vec<Box<dyn Command + 'static>>>();

        // Size of the Vec's allocation (capacity * element size)
        heap_size += self.decls.capacity() * std::mem::size_of::<Box<dyn Command>>();

        // Calculate heap size of all declarations
        for decl in self.decls.iter() {
            // For each Command trait object, we need to account for:
            // 1. The vtable pointer (already in Box<dyn Command>)
            // 2. The actual implementation size

            // Get the signature to estimate the size
            let sig = decl.signature();

            // Estimate the size based on the signature components
            let sig_size = std::mem::size_of_val(&sig)
                + sig.name.capacity()
                + sig.description.capacity()
                + sig.extra_description.capacity()
                + sig.category.to_string().len()
                + sig.required_positional.capacity()
                    * std::mem::size_of_val(&sig.required_positional)
                + sig.optional_positional.capacity()
                    * std::mem::size_of_val(&sig.optional_positional)
                + sig
                    .rest_positional
                    .map_or(0, |_| std::mem::size_of::<Type>())
                + sig.named.capacity() * std::mem::size_of_val(&sig.named);

            // Add description size
            let desc_size = decl.description().len();

            // Add extra description size
            let extra_desc_size = decl.extra_description().len();

            // Add examples size if any
            let examples_size = decl
                .examples()
                .iter()
                .map(|ex| {
                    // Calculate size of example components
                    ex.description.len() + ex.example.len() +
                    // Calculate exact size of the result
                    ex.result.as_ref().map_or(0, |result| {
                        // The result is a Value, so we need to calculate its exact size
                        calculate_value_size(result)
                    })
                })
                .sum::<usize>();

            // Sum up the total size
            heap_size += sig_size + desc_size + extra_desc_size + examples_size;
        }

        (stack_size, heap_size)
    }

    /// Get the memory size of the blocks collection, including both stack and heap allocations
    pub fn blocks_memory_size(&self) -> (usize, usize) {
        let stack_size = std::mem::size_of::<Arc<Vec<Arc<Block>>>>();
        let mut heap_size = 0;

        // Size of the Arc control block for the outer Arc
        let arc_control_block = 24; // Typical size on 64-bit systems
        heap_size += arc_control_block;

        // Size of the Vec itself on the heap
        let vec_overhead = std::mem::size_of::<Vec<Arc<Block>>>();
        heap_size += vec_overhead;

        // Size of the Vec's allocation (capacity * element size)
        heap_size += self.blocks.capacity() * std::mem::size_of::<Arc<Block>>();

        // Calculate heap size of all blocks
        for block_arc in self.blocks.iter() {
            // Add size of the inner Arc control block
            let inner_arc_control_block = 24; // Typical size on 64-bit systems
            heap_size += inner_arc_control_block;

            // Get reference to the actual Block
            let block = &**block_arc;

            // Calculate size of pipelines Vec
            let pipelines_overhead = std::mem::size_of::<Vec<Pipeline>>();
            let pipelines_allocation = block.pipelines.capacity() * std::mem::size_of::<Pipeline>();
            heap_size += pipelines_overhead + pipelines_allocation;

            // Calculate size of each pipeline's elements
            for pipeline in &block.pipelines {
                let elements_overhead = std::mem::size_of::<Vec<PipelineElement>>();
                let elements_allocation =
                    pipeline.elements.capacity() * std::mem::size_of::<PipelineElement>();
                heap_size += elements_overhead + elements_allocation;

                // Calculate size of each element's expression
                for element in &pipeline.elements {
                    // Get the expression from the element
                    let expr = &element.expr;

                    // Add size of expression struct
                    heap_size += std::mem::size_of::<Expression>();

                    // Add size of expression-specific data based on the Expr enum variant
                    match &expr.expr {
                        Expr::Call(call) => {
                            heap_size += std::mem::size_of::<Call>();
                            heap_size +=
                                call.arguments.capacity() * std::mem::size_of::<Argument>();
                            // Add size of head and decl_id
                            heap_size += std::mem::size_of::<Span>();
                            heap_size += std::mem::size_of::<DeclId>();
                        }
                        Expr::ExternalCall(_head, args) => {
                            // ExternalCall has a Box<Expression> for head and Box<[ExternalArgument]> for args
                            heap_size += std::mem::size_of::<Expression>(); // For the head
                            heap_size += std::mem::size_of::<Box<[ExternalArgument]>>(); // For the args box

                            // Add size of args array
                            heap_size += args.len() * std::mem::size_of::<ExternalArgument>();

                            // Add size of head span
                            heap_size += std::mem::size_of::<Span>();
                        }
                        Expr::Subexpression(_block_id) => {
                            heap_size += std::mem::size_of::<BlockId>();
                        }
                        Expr::Block(_block_id) => {
                            heap_size += std::mem::size_of::<BlockId>();
                        }
                        Expr::List(items) => {
                            heap_size += std::mem::size_of::<Vec<ListItem>>();
                            heap_size += items.capacity() * std::mem::size_of::<ListItem>();
                        }
                        Expr::Record(items) => {
                            heap_size += std::mem::size_of::<Vec<RecordItem>>();
                            heap_size += items.capacity() * std::mem::size_of::<RecordItem>();
                        }
                        Expr::Table(_table) => {
                            // Table has headers and rows
                            heap_size += std::mem::size_of::<Table>();
                        }
                        Expr::Keyword(_keyword) => {
                            heap_size += std::mem::size_of::<Keyword>();
                        }
                        Expr::String(s) => {
                            heap_size += s.capacity();
                        }
                        Expr::RawString(s) => {
                            heap_size += s.capacity();
                        }
                        Expr::GlobPattern(g, _) => {
                            heap_size += g.capacity();
                        }
                        // For other variants that don't have heap allocations
                        _ => {}
                    }
                }
            }

            // Calculate size of signature
            let signature = &block.signature;
            heap_size += std::mem::size_of::<Signature>();
            // Add size of signature components
            heap_size += signature.name.capacity();
            heap_size += signature.description.capacity();
            heap_size += signature.extra_description.capacity();
            // We don't have direct access to PositionalArg and NamedArg types here
            // So we'll estimate their size based on typical struct sizes
            let positional_arg_size = 24; // Reasonable estimate for a struct with a name and type
            let named_arg_size = 32; // Reasonable estimate for a struct with a name, type, and flags

            heap_size += signature.required_positional.capacity() * positional_arg_size;
            heap_size += signature.optional_positional.capacity() * positional_arg_size;
            heap_size += signature.named.capacity() * named_arg_size;
        }

        (stack_size, heap_size)
    }

    /// Get the memory size of the spans collection, including both stack and heap allocations
    pub fn spans_memory_size(&self) -> (usize, usize) {
        let stack_size = std::mem::size_of::<Vec<Span>>();

        // Size of the Vec's allocation (capacity * element size)
        let heap_size = self.spans.capacity() * std::mem::size_of::<Span>();

        (stack_size, heap_size)
    }

    /// Get the memory size of the env_vars collection, including both stack and heap allocations
    pub fn env_vars_memory_size(&self) -> (usize, usize) {
        let stack_size = std::mem::size_of::<Arc<EnvVars>>();
        let mut heap_size = 0;

        // Size of the Arc control block (strong count, weak count, data pointer)
        let arc_control_block = 24; // Typical size on 64-bit systems
        heap_size += arc_control_block;

        // Size of the EnvVars HashMap itself
        // EnvVars is a type alias for HashMap<String, HashMap<String, Value>>
        let envvars_size = std::mem::size_of::<EnvVars>();
        heap_size += envvars_size;

        // Calculate heap size of all environment variables
        for (overlay_name, env_map) in self.env_vars.iter() {
            // Size of the overlay name string (String has ptr, len, capacity on stack)
            // We add the actual string data size on heap
            heap_size += overlay_name.capacity();

            // Size of the HashMap for this overlay
            // HashMap has a complex structure with buckets, entries, etc.

            // HashMap overhead (control structure)
            let hashmap_overhead = std::mem::size_of::<HashMap<String, Value>>();
            heap_size += hashmap_overhead;

            // HashMap bucket array size (capacity * pointer size)
            // HashMap typically uses a power-of-two capacity
            let bucket_array_size = env_map.capacity() * std::mem::size_of::<*const u8>();
            heap_size += bucket_array_size;

            // Size of each key-value pair
            for (key, value) in env_map.iter() {
                // String data on heap (key)
                let key_overhead = std::mem::size_of::<String>();
                let key_data = key.capacity();
                heap_size += key_overhead + key_data;

                // Value size (recursive calculation for exact measurement)
                heap_size += calculate_value_size(value);
            }
        }

        (stack_size, heap_size)
    }

    /// Get the memory size of the config, including both stack and heap allocations
    pub fn config_memory_size(&self) -> (usize, usize) {
        let stack_size = std::mem::size_of::<Arc<Config>>();
        let mut heap_size = 0;

        // Size of the Arc control block (strong count, weak count, data pointer)
        let arc_control_block = 24; // Typical size on 64-bit systems
        heap_size += arc_control_block;

        // Size of the Config struct itself
        let config_size = std::mem::size_of::<Config>();
        heap_size += config_size;

        // Add size of color_config HashMap
        let color_config_overhead = std::mem::size_of::<HashMap<String, Value>>();
        let color_config_buckets =
            self.config.color_config.capacity() * std::mem::size_of::<*const u8>();
        heap_size += color_config_overhead + color_config_buckets;

        // Add size of each color_config entry
        for (key, value) in self.config.color_config.iter() {
            heap_size += std::mem::size_of::<String>() + key.capacity(); // Key overhead + data
            heap_size += calculate_value_size(value); // Exact value size
        }

        // Add size of explore HashMap
        let explore_overhead = std::mem::size_of::<HashMap<String, Value>>();
        let explore_buckets = self.config.explore.capacity() * std::mem::size_of::<*const u8>();
        heap_size += explore_overhead + explore_buckets;

        // Add size of each explore entry
        for (key, value) in self.config.explore.iter() {
            heap_size += std::mem::size_of::<String>() + key.capacity(); // Key overhead + data
            heap_size += calculate_value_size(value); // Exact value size
        }

        // Add size of plugins HashMap
        let plugins_overhead = std::mem::size_of::<HashMap<String, Value>>();
        let plugins_buckets = self.config.plugins.capacity() * std::mem::size_of::<*const u8>();
        heap_size += plugins_overhead + plugins_buckets;

        // Add size of each plugins entry
        for (key, value) in self.config.plugins.iter() {
            heap_size += std::mem::size_of::<String>() + key.capacity(); // Key overhead + data
            heap_size += calculate_value_size(value); // Exact value size
        }

        // Add size of keybindings Vec
        let keybindings_overhead = std::mem::size_of::<Vec<ParsedKeybinding>>();
        let keybindings_data =
            self.config.keybindings.capacity() * std::mem::size_of::<ParsedKeybinding>();
        heap_size += keybindings_overhead + keybindings_data;

        // Add estimated size for keybindings' internal strings (30 bytes per keybinding)
        heap_size += self.config.keybindings.len() * 30;

        // Add size of menus Vec
        let menus_overhead = std::mem::size_of::<Vec<ParsedMenu>>();
        let menus_data = self.config.menus.capacity() * std::mem::size_of::<ParsedMenu>();
        heap_size += menus_overhead + menus_data;

        // Add estimated size for menus' internal strings (40 bytes per menu)
        heap_size += self.config.menus.len() * 40;

        // Add size of buffer_editor and show_banner Values (exact calculation)
        heap_size += calculate_value_size(&self.config.buffer_editor);
        heap_size += calculate_value_size(&self.config.show_banner);

        (stack_size, heap_size)
    }

    #[cfg(feature = "plugin")]
    /// Get the memory size of the plugins collection, including both stack and heap allocations
    pub fn plugins_memory_size(&self) -> (usize, usize) {
        let stack_size = std::mem::size_of::<Vec<Arc<dyn RegisteredPlugin>>>();
        let mut heap_size = 0;

        // Size of the Vec itself on the heap
        let vec_overhead = std::mem::size_of::<Vec<Arc<dyn RegisteredPlugin>>>();
        let vec_allocation =
            self.plugins.capacity() * std::mem::size_of::<Arc<dyn RegisteredPlugin>>();
        heap_size += vec_overhead + vec_allocation;

        // Calculate heap size of all plugins
        for plugin in self.plugins.iter() {
            // Size of the Arc control block for each plugin
            let arc_control_block = 24; // Typical size on 64-bit systems
            heap_size += arc_control_block;

            // Size of the vtable pointer for the trait object
            let vtable_ptr_size = std::mem::size_of::<usize>();
            heap_size += vtable_ptr_size;

            // Get plugin identity to calculate exact name size
            let identity = plugin.identity();
            heap_size += std::mem::size_of::<String>() + identity.name().len();

            // Add version string size if available
            // Note: The PluginIdentity trait might not have a version() method in all implementations
            // so we'll just estimate a reasonable size for version information
            heap_size += 20; // Reasonable estimate for version string

            // Calculate size of signatures
            // Note: The RegisteredPlugin trait might not have a signatures() method in all implementations
            // so we'll just estimate a reasonable size for signatures
            heap_size += 256; // Reasonable estimate for signatures
        }

        (stack_size, heap_size)
    }

    pub fn new() -> Self {
        Self {
            files: vec![],
            virtual_paths: vec![],
            vars: vec![
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
                Variable::new(Span::new(0, 0), Type::Any, false),
            ],
            decls: Arc::new(vec![]),
            blocks: Arc::new(vec![]),
            modules: Arc::new(vec![Arc::new(Module::new(
                DEFAULT_OVERLAY_NAME.as_bytes().to_vec(),
            ))]),
            spans: vec![Span::unknown()],
            doccomments: Doccomments::new(),
            // make sure we have some default overlay:
            scope: ScopeFrame::with_empty_overlay(
                DEFAULT_OVERLAY_NAME.as_bytes().to_vec(),
                ModuleId::new(0),
                false,
            ),
            signal_handlers: None,
            signals: Signals::empty(),
            env_vars: Arc::new(
                [(DEFAULT_OVERLAY_NAME.to_string(), HashMap::new())]
                    .into_iter()
                    .collect(),
            ),
            previous_env_vars: Arc::new(HashMap::new()),
            config: Arc::new(Config::default()),
            pipeline_externals_state: Arc::new((AtomicU32::new(0), AtomicU32::new(0))),
            repl_state: Arc::new(Mutex::new(ReplState {
                buffer: "".to_string(),
                cursor_pos: 0,
            })),
            table_decl_id: None,
            #[cfg(feature = "plugin")]
            plugin_path: None,
            #[cfg(feature = "plugin")]
            plugins: vec![],
            config_path: HashMap::new(),
            history_enabled: true,
            history_session_id: 0,
            file: None,
            regex_cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(REGEX_CACHE_SIZE).expect("tried to create cache of size zero"),
            ))),
            is_interactive: false,
            is_login: false,
            startup_time: -1,
            is_debugging: IsDebugging::new(false),
            debugger: Arc::new(Mutex::new(Box::new(NoopDebugger))),
            jobs: Arc::new(Mutex::new(Jobs::default())),
            current_thread_job: None,
            exit_warning_given: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn signals(&self) -> &Signals {
        &self.signals
    }

    pub fn reset_signals(&mut self) {
        self.signals.reset();
        if let Some(ref handlers) = self.signal_handlers {
            handlers.run(SignalAction::Reset);
        }
    }

    pub fn set_signals(&mut self, signals: Signals) {
        self.signals = signals;
    }

    /// Merges a `StateDelta` onto the current state. These deltas come from a system, like the parser, that
    /// creates a new set of definitions and visible symbols in the current scope. We make this transactional
    /// as there are times when we want to run the parser and immediately throw away the results (namely:
    /// syntax highlighting and completions).
    ///
    /// When we want to preserve what the parser has created, we can take its output (the `StateDelta`) and
    /// use this function to merge it into the global state.
    pub fn merge_delta(&mut self, mut delta: StateDelta) -> Result<(), ShellError> {
        // Take the mutable reference and extend the permanent state from the working set
        self.files.extend(delta.files);
        self.virtual_paths.extend(delta.virtual_paths);
        self.vars.extend(delta.vars);
        self.spans.extend(delta.spans);
        self.doccomments.merge_with(delta.doccomments);

        // Avoid potentially cloning the Arcs if we aren't adding anything
        if !delta.decls.is_empty() {
            Arc::make_mut(&mut self.decls).extend(delta.decls);
        }
        if !delta.blocks.is_empty() {
            Arc::make_mut(&mut self.blocks).extend(delta.blocks);
        }
        if !delta.modules.is_empty() {
            Arc::make_mut(&mut self.modules).extend(delta.modules);
        }

        let first = delta.scope.remove(0);

        for (delta_name, delta_overlay) in first.clone().overlays {
            if let Some((_, existing_overlay)) = self
                .scope
                .overlays
                .iter_mut()
                .find(|(name, _)| name == &delta_name)
            {
                // Updating existing overlay
                for item in delta_overlay.decls.into_iter() {
                    existing_overlay.decls.insert(item.0, item.1);
                }
                for item in delta_overlay.vars.into_iter() {
                    existing_overlay.vars.insert(item.0, item.1);
                }
                for item in delta_overlay.modules.into_iter() {
                    existing_overlay.modules.insert(item.0, item.1);
                }

                existing_overlay
                    .visibility
                    .merge_with(delta_overlay.visibility);
            } else {
                // New overlay was added to the delta
                self.scope.overlays.push((delta_name, delta_overlay));
            }
        }

        let mut activated_ids = self.translate_overlay_ids(&first);

        let mut removed_ids = vec![];

        for name in &first.removed_overlays {
            if let Some(overlay_id) = self.find_overlay(name) {
                removed_ids.push(overlay_id);
            }
        }

        // Remove overlays removed in delta
        self.scope
            .active_overlays
            .retain(|id| !removed_ids.contains(id));

        // Move overlays activated in the delta to be first
        self.scope
            .active_overlays
            .retain(|id| !activated_ids.contains(id));
        self.scope.active_overlays.append(&mut activated_ids);

        #[cfg(feature = "plugin")]
        if !delta.plugins.is_empty() {
            for plugin in std::mem::take(&mut delta.plugins) {
                // Connect plugins to the signal handlers
                if let Some(handlers) = &self.signal_handlers {
                    plugin.clone().configure_signal_handler(handlers)?;
                }

                // Replace plugins that overlap in identity.
                if let Some(existing) = self
                    .plugins
                    .iter_mut()
                    .find(|p| p.identity().name() == plugin.identity().name())
                {
                    // Stop the existing plugin, so that the new plugin definitely takes over
                    existing.stop()?;
                    *existing = plugin;
                } else {
                    self.plugins.push(plugin);
                }
            }
        }

        #[cfg(feature = "plugin")]
        if !delta.plugin_registry_items.is_empty() {
            // Update the plugin file with the new signatures.
            if self.plugin_path.is_some() {
                self.update_plugin_file(std::mem::take(&mut delta.plugin_registry_items))?;
            }
        }

        Ok(())
    }

    /// Merge the environment from the runtime Stack into the engine state
    pub fn merge_env(&mut self, stack: &mut Stack) -> Result<(), ShellError> {
        for mut scope in stack.env_vars.drain(..) {
            for (overlay_name, mut env) in Arc::make_mut(&mut scope).drain() {
                if let Some(env_vars) = Arc::make_mut(&mut self.env_vars).get_mut(&overlay_name) {
                    // Updating existing overlay
                    env_vars.extend(env.drain());
                } else {
                    // Pushing a new overlay
                    Arc::make_mut(&mut self.env_vars).insert(overlay_name, env);
                }
            }
        }

        let cwd = self.cwd(Some(stack))?;
        std::env::set_current_dir(cwd).map_err(|err| {
            IoError::new_internal(err.kind(), "Could not set current dir", crate::location!())
        })?;

        if let Some(config) = stack.config.take() {
            // If config was updated in the stack, replace it.
            self.config = config;

            // Make plugin GC config changes take effect immediately.
            #[cfg(feature = "plugin")]
            self.update_plugin_gc_configs(&self.config.plugin_gc);
        }

        Ok(())
    }

    pub fn has_overlay(&self, name: &[u8]) -> bool {
        self.scope
            .overlays
            .iter()
            .any(|(overlay_name, _)| name == overlay_name)
    }

    pub fn active_overlay_ids<'a, 'b>(
        &'b self,
        removed_overlays: &'a [Vec<u8>],
    ) -> impl DoubleEndedIterator<Item = &'b OverlayId> + 'a
    where
        'b: 'a,
    {
        self.scope.active_overlays.iter().filter(|id| {
            !removed_overlays
                .iter()
                .any(|name| name == self.get_overlay_name(**id))
        })
    }

    pub fn active_overlays<'a, 'b>(
        &'b self,
        removed_overlays: &'a [Vec<u8>],
    ) -> impl DoubleEndedIterator<Item = &'b OverlayFrame> + 'a
    where
        'b: 'a,
    {
        self.active_overlay_ids(removed_overlays)
            .map(|id| self.get_overlay(*id))
    }

    pub fn active_overlay_names<'a, 'b>(
        &'b self,
        removed_overlays: &'a [Vec<u8>],
    ) -> impl DoubleEndedIterator<Item = &'b [u8]> + 'a
    where
        'b: 'a,
    {
        self.active_overlay_ids(removed_overlays)
            .map(|id| self.get_overlay_name(*id))
    }

    /// Translate overlay IDs from other to IDs in self
    pub fn translate_overlay_ids(&self, other: &ScopeFrame) -> Vec<OverlayId> {
        let other_names = other.active_overlays.iter().map(|other_id| {
            &other
                .overlays
                .get(other_id.get())
                .expect("internal error: missing overlay")
                .0
        });

        other_names
            .map(|other_name| {
                self.find_overlay(other_name)
                    .expect("internal error: missing overlay")
            })
            .collect()
    }

    pub fn last_overlay_name(&self, removed_overlays: &[Vec<u8>]) -> &[u8] {
        self.active_overlay_names(removed_overlays)
            .last()
            .expect("internal error: no active overlays")
    }

    pub fn last_overlay(&self, removed_overlays: &[Vec<u8>]) -> &OverlayFrame {
        self.active_overlay_ids(removed_overlays)
            .last()
            .map(|id| self.get_overlay(*id))
            .expect("internal error: no active overlays")
    }

    pub fn get_overlay_name(&self, overlay_id: OverlayId) -> &[u8] {
        &self
            .scope
            .overlays
            .get(overlay_id.get())
            .expect("internal error: missing overlay")
            .0
    }

    pub fn get_overlay(&self, overlay_id: OverlayId) -> &OverlayFrame {
        &self
            .scope
            .overlays
            .get(overlay_id.get())
            .expect("internal error: missing overlay")
            .1
    }

    pub fn render_env_vars(&self) -> HashMap<&str, &Value> {
        let mut result: HashMap<&str, &Value> = HashMap::new();

        for overlay_name in self.active_overlay_names(&[]) {
            let name = String::from_utf8_lossy(overlay_name);
            if let Some(env_vars) = self.env_vars.get(name.as_ref()) {
                result.extend(env_vars.iter().map(|(k, v)| (k.as_str(), v)));
            }
        }

        result
    }

    pub fn add_env_var(&mut self, name: String, val: Value) {
        let overlay_name = String::from_utf8_lossy(self.last_overlay_name(&[])).to_string();

        if let Some(env_vars) = Arc::make_mut(&mut self.env_vars).get_mut(&overlay_name) {
            env_vars.insert(name, val);
        } else {
            Arc::make_mut(&mut self.env_vars)
                .insert(overlay_name, [(name, val)].into_iter().collect());
        }
    }

    pub fn get_env_var(&self, name: &str) -> Option<&Value> {
        for overlay_id in self.scope.active_overlays.iter().rev() {
            let overlay_name = String::from_utf8_lossy(self.get_overlay_name(*overlay_id));
            if let Some(env_vars) = self.env_vars.get(overlay_name.as_ref()) {
                if let Some(val) = env_vars.get(name) {
                    return Some(val);
                }
            }
        }

        None
    }

    // Returns Some((name, value)) if found, None otherwise.
    // When updating environment variables, make sure to use
    // the same case (the returned "name") as the original
    // environment variable name.
    pub fn get_env_var_insensitive(&self, name: &str) -> Option<(&String, &Value)> {
        for overlay_id in self.scope.active_overlays.iter().rev() {
            let overlay_name = String::from_utf8_lossy(self.get_overlay_name(*overlay_id));
            if let Some(env_vars) = self.env_vars.get(overlay_name.as_ref()) {
                if let Some(v) = env_vars.iter().find(|(k, _)| k.eq_ignore_case(name)) {
                    return Some((v.0, v.1));
                }
            }
        }

        None
    }

    #[cfg(feature = "plugin")]
    pub fn plugins(&self) -> &[Arc<dyn RegisteredPlugin>] {
        &self.plugins
    }

    #[cfg(feature = "plugin")]
    pub fn update_plugin_file(
        &self,
        updated_items: Vec<PluginRegistryItem>,
    ) -> Result<(), ShellError> {
        // Updating the signatures plugin file with the added signatures
        use std::fs::File;

        let plugin_path = self
            .plugin_path
            .as_ref()
            .ok_or_else(|| ShellError::GenericError {
                error: "Plugin file path not set".into(),
                msg: "".into(),
                span: None,
                help: Some("you may be running nu with --no-config-file".into()),
                inner: vec![],
            })?;

        // Read the current contents of the plugin file if it exists
        let mut contents = match File::open(plugin_path.as_path()) {
            Ok(mut plugin_file) => PluginRegistryFile::read_from(&mut plugin_file, None),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Ok(PluginRegistryFile::default())
                } else {
                    Err(ShellError::Io(IoError::new_internal_with_path(
                        err.kind(),
                        "Failed to open plugin file",
                        crate::location!(),
                        PathBuf::from(plugin_path),
                    )))
                }
            }
        }?;

        // Update the given signatures
        for item in updated_items {
            contents.upsert_plugin(item);
        }

        // Write it to the same path
        let plugin_file = File::create(plugin_path.as_path()).map_err(|err| {
            IoError::new_internal_with_path(
                err.kind(),
                "Failed to write plugin file",
                crate::location!(),
                PathBuf::from(plugin_path),
            )
        })?;

        contents.write_to(plugin_file, None)
    }

    /// Update plugins with new garbage collection config
    #[cfg(feature = "plugin")]
    fn update_plugin_gc_configs(&self, plugin_gc: &crate::PluginGcConfigs) {
        for plugin in &self.plugins {
            plugin.set_gc_config(plugin_gc.get(plugin.identity().name()));
        }
    }

    pub fn num_files(&self) -> usize {
        self.files.len()
    }

    pub fn num_virtual_paths(&self) -> usize {
        self.virtual_paths.len()
    }

    pub fn num_vars(&self) -> usize {
        self.vars.len()
    }

    pub fn num_decls(&self) -> usize {
        self.decls.len()
    }

    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    pub fn num_modules(&self) -> usize {
        self.modules.len()
    }

    pub fn num_spans(&self) -> usize {
        self.spans.len()
    }

    pub fn num_env_vars(&self) -> usize {
        self.env_vars.values().map(|overlay| overlay.len()).sum()
    }

    pub fn num_config_vars(&self) -> usize {
        self.config.entry_count()
    }

    #[cfg(feature = "plugin")]
    pub fn num_plugins(&self) -> usize {
        self.plugins.len()
    }

    #[cfg(not(feature = "plugin"))]
    pub fn num_plugins(&self) -> usize {
        0
    }

    pub fn print_vars(&self) {
        for var in self.vars.iter().enumerate() {
            println!("var{}: {:?}", var.0, var.1);
        }
    }

    pub fn print_decls(&self) {
        for decl in self.decls.iter().enumerate() {
            println!("decl{}: {:?}", decl.0, decl.1.signature());
        }
    }

    pub fn print_blocks(&self) {
        for block in self.blocks.iter().enumerate() {
            println!("block{}: {:?}", block.0, block.1);
        }
    }

    pub fn print_contents(&self) {
        for cached_file in self.files.iter() {
            let string = String::from_utf8_lossy(&cached_file.content);
            println!("{string}");
        }
    }

    /// Find the [`DeclId`](crate::DeclId) corresponding to a declaration with `name`.
    ///
    /// Searches within active overlays, and filtering out overlays in `removed_overlays`.
    pub fn find_decl(&self, name: &[u8], removed_overlays: &[Vec<u8>]) -> Option<DeclId> {
        let mut visibility: Visibility = Visibility::new();

        for overlay_frame in self.active_overlays(removed_overlays).rev() {
            visibility.append(&overlay_frame.visibility);

            if let Some(decl_id) = overlay_frame.get_decl(name) {
                if visibility.is_decl_id_visible(&decl_id) {
                    return Some(decl_id);
                }
            }
        }

        None
    }

    /// Find the name of the declaration corresponding to `decl_id`.
    ///
    /// Searches within active overlays, and filtering out overlays in `removed_overlays`.
    pub fn find_decl_name(&self, decl_id: DeclId, removed_overlays: &[Vec<u8>]) -> Option<&[u8]> {
        let mut visibility: Visibility = Visibility::new();

        for overlay_frame in self.active_overlays(removed_overlays).rev() {
            visibility.append(&overlay_frame.visibility);

            if visibility.is_decl_id_visible(&decl_id) {
                for (name, id) in overlay_frame.decls.iter() {
                    if id == &decl_id {
                        return Some(name);
                    }
                }
            }
        }

        None
    }

    /// Find the [`OverlayId`](crate::OverlayId) corresponding to `name`.
    ///
    /// Searches all overlays, not just active overlays. To search only in active overlays, use [`find_active_overlay`](EngineState::find_active_overlay)
    pub fn find_overlay(&self, name: &[u8]) -> Option<OverlayId> {
        self.scope.find_overlay(name)
    }

    /// Find the [`OverlayId`](crate::OverlayId) of the active overlay corresponding to `name`.
    ///
    /// Searches only active overlays. To search in all overlays, use [`find_overlay`](EngineState::find_active_overlay)
    pub fn find_active_overlay(&self, name: &[u8]) -> Option<OverlayId> {
        self.scope.find_active_overlay(name)
    }

    /// Find the [`ModuleId`](crate::ModuleId) corresponding to `name`.
    ///
    /// Searches within active overlays, and filtering out overlays in `removed_overlays`.
    pub fn find_module(&self, name: &[u8], removed_overlays: &[Vec<u8>]) -> Option<ModuleId> {
        for overlay_frame in self.active_overlays(removed_overlays).rev() {
            if let Some(module_id) = overlay_frame.modules.get(name) {
                return Some(*module_id);
            }
        }

        None
    }

    pub fn get_module_comments(&self, module_id: ModuleId) -> Option<&[Span]> {
        self.doccomments.get_module_comments(module_id)
    }

    #[cfg(feature = "plugin")]
    pub fn plugin_decls(&self) -> impl Iterator<Item = &Box<dyn Command + 'static>> {
        let mut unique_plugin_decls = HashMap::new();

        // Make sure there are no duplicate decls: Newer one overwrites the older one
        for decl in self.decls.iter().filter(|d| d.is_plugin()) {
            unique_plugin_decls.insert(decl.name(), decl);
        }

        let mut plugin_decls: Vec<(&str, &Box<dyn Command>)> =
            unique_plugin_decls.into_iter().collect();

        // Sort the plugins by name so we don't end up with a random plugin file each time
        plugin_decls.sort_by(|a, b| a.0.cmp(b.0));
        plugin_decls.into_iter().map(|(_, decl)| decl)
    }

    pub fn which_module_has_decl(
        &self,
        decl_name: &[u8],
        removed_overlays: &[Vec<u8>],
    ) -> Option<&[u8]> {
        for overlay_frame in self.active_overlays(removed_overlays).rev() {
            for (module_name, module_id) in overlay_frame.modules.iter() {
                let module = self.get_module(*module_id);
                if module.has_decl(decl_name) {
                    return Some(module_name);
                }
            }
        }

        None
    }

    pub fn find_commands_by_predicate(
        &self,
        mut predicate: impl FnMut(&[u8]) -> bool,
        ignore_deprecated: bool,
    ) -> Vec<(DeclId, Vec<u8>, Option<String>, CommandType)> {
        let mut output = vec![];

        for overlay_frame in self.active_overlays(&[]).rev() {
            for (name, decl_id) in &overlay_frame.decls {
                if overlay_frame.visibility.is_decl_id_visible(decl_id) && predicate(name) {
                    let command = self.get_decl(*decl_id);
                    if ignore_deprecated && command.signature().category == Category::Removed {
                        continue;
                    }
                    output.push((
                        *decl_id,
                        name.clone(),
                        Some(command.description().to_string()),
                        command.command_type(),
                    ));
                }
            }
        }

        output
    }

    pub fn get_span_contents(&self, span: Span) -> &[u8] {
        for file in &self.files {
            if file.covered_span.contains_span(span) {
                return &file.content
                    [(span.start - file.covered_span.start)..(span.end - file.covered_span.start)];
            }
        }
        &[0u8; 0]
    }

    /// Get the global config from the engine state.
    ///
    /// Use [`Stack::get_config()`] instead whenever the `Stack` is available, as it takes into
    /// account local changes to `$env.config`.
    pub fn get_config(&self) -> &Arc<Config> {
        &self.config
    }

    pub fn set_config(&mut self, conf: impl Into<Arc<Config>>) {
        let conf = conf.into();

        #[cfg(feature = "plugin")]
        if conf.plugin_gc != self.config.plugin_gc {
            // Make plugin GC config changes take effect immediately.
            self.update_plugin_gc_configs(&conf.plugin_gc);
        }

        self.config = conf;
    }

    /// Fetch the configuration for a plugin
    ///
    /// The `plugin` must match the registered name of a plugin.  For `plugin add
    /// nu_plugin_example` the plugin name to use will be `"example"`
    pub fn get_plugin_config(&self, plugin: &str) -> Option<&Value> {
        self.config.plugins.get(plugin)
    }

    /// Returns the configuration settings for command history or `None` if history is disabled
    pub fn history_config(&self) -> Option<HistoryConfig> {
        self.history_enabled.then(|| self.config.history)
    }

    pub fn get_var(&self, var_id: VarId) -> &Variable {
        self.vars
            .get(var_id.get())
            .expect("internal error: missing variable")
    }

    pub fn get_constant(&self, var_id: VarId) -> Option<&Value> {
        let var = self.get_var(var_id);
        var.const_val.as_ref()
    }

    pub fn generate_nu_constant(&mut self) {
        self.vars[NU_VARIABLE_ID.get()].const_val = Some(create_nu_constant(self, Span::unknown()));
    }

    pub fn get_decl(&self, decl_id: DeclId) -> &dyn Command {
        self.decls
            .get(decl_id.get())
            .expect("internal error: missing declaration")
            .as_ref()
    }

    /// Get all commands within scope, sorted by the commands' names
    pub fn get_decls_sorted(&self, include_hidden: bool) -> Vec<(Vec<u8>, DeclId)> {
        let mut decls_map = HashMap::new();

        for overlay_frame in self.active_overlays(&[]) {
            let new_decls = if include_hidden {
                overlay_frame.decls.clone()
            } else {
                overlay_frame
                    .decls
                    .clone()
                    .into_iter()
                    .filter(|(_, id)| overlay_frame.visibility.is_decl_id_visible(id))
                    .collect()
            };

            decls_map.extend(new_decls);
        }

        let mut decls: Vec<(Vec<u8>, DeclId)> = decls_map.into_iter().collect();

        decls.sort_by(|a, b| a.0.cmp(&b.0));
        decls
    }

    pub fn get_signature(&self, decl: &dyn Command) -> Signature {
        if let Some(block_id) = decl.block_id() {
            *self.blocks[block_id.get()].signature.clone()
        } else {
            decl.signature()
        }
    }

    /// Get signatures of all commands within scope with their decl ids.
    pub fn get_signatures_and_declids(&self, include_hidden: bool) -> Vec<(Signature, DeclId)> {
        self.get_decls_sorted(include_hidden)
            .into_iter()
            .map(|(_, id)| {
                let decl = self.get_decl(id);

                (self.get_signature(decl).update_from_command(decl), id)
            })
            .collect()
    }

    pub fn get_block(&self, block_id: BlockId) -> &Arc<Block> {
        self.blocks
            .get(block_id.get())
            .expect("internal error: missing block")
    }

    /// Optionally get a block by id, if it exists
    ///
    /// Prefer to use [`.get_block()`](Self::get_block) in most cases - `BlockId`s that don't exist
    /// are normally a compiler error. This only exists to stop plugins from crashing the engine if
    /// they send us something invalid.
    pub fn try_get_block(&self, block_id: BlockId) -> Option<&Arc<Block>> {
        self.blocks.get(block_id.get())
    }

    pub fn get_module(&self, module_id: ModuleId) -> &Module {
        self.modules
            .get(module_id.get())
            .expect("internal error: missing module")
    }

    pub fn get_virtual_path(&self, virtual_path_id: VirtualPathId) -> &(String, VirtualPath) {
        self.virtual_paths
            .get(virtual_path_id.get())
            .expect("internal error: missing virtual path")
    }

    pub fn next_span_start(&self) -> usize {
        if let Some(cached_file) = self.files.last() {
            cached_file.covered_span.end
        } else {
            0
        }
    }

    pub fn files(
        &self,
    ) -> impl DoubleEndedIterator<Item = &CachedFile> + ExactSizeIterator<Item = &CachedFile> {
        self.files.iter()
    }

    pub fn add_file(&mut self, filename: Arc<str>, content: Arc<[u8]>) -> FileId {
        let next_span_start = self.next_span_start();
        let next_span_end = next_span_start + content.len();

        let covered_span = Span::new(next_span_start, next_span_end);

        self.files.push(CachedFile {
            name: filename,
            content,
            covered_span,
        });

        FileId::new(self.num_files() - 1)
    }

    pub fn set_config_path(&mut self, key: &str, val: PathBuf) {
        self.config_path.insert(key.to_string(), val);
    }

    pub fn get_config_path(&self, key: &str) -> Option<&PathBuf> {
        self.config_path.get(key)
    }

    pub fn build_desc(&self, spans: &[Span]) -> (String, String) {
        let comment_lines: Vec<&[u8]> = spans
            .iter()
            .map(|span| self.get_span_contents(*span))
            .collect();
        build_desc(&comment_lines)
    }

    pub fn build_module_desc(&self, module_id: ModuleId) -> Option<(String, String)> {
        self.get_module_comments(module_id)
            .map(|comment_spans| self.build_desc(comment_spans))
    }

    /// Returns the current working directory, which is guaranteed to be canonicalized.
    ///
    /// Returns an empty String if $env.PWD doesn't exist.
    #[deprecated(since = "0.92.3", note = "please use `EngineState::cwd()` instead")]
    pub fn current_work_dir(&self) -> String {
        self.cwd(None)
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    /// Returns the current working directory, which is guaranteed to be an
    /// absolute path without trailing slashes (unless it's the root path), but
    /// might contain symlink components.
    ///
    /// If `stack` is supplied, also considers modifications to the working
    /// directory on the stack that have yet to be merged into the engine state.
    pub fn cwd(&self, stack: Option<&Stack>) -> Result<AbsolutePathBuf, ShellError> {
        // Helper function to create a simple generic error.
        fn error(msg: &str, cwd: impl AsRef<nu_path::Path>) -> ShellError {
            ShellError::GenericError {
                error: msg.into(),
                msg: format!("$env.PWD = {}", cwd.as_ref().display()),
                span: None,
                help: Some("Use `cd` to reset $env.PWD into a good state".into()),
                inner: vec![],
            }
        }

        // Retrieve $env.PWD from the stack or the engine state.
        let pwd = if let Some(stack) = stack {
            stack.get_env_var(self, "PWD")
        } else {
            self.get_env_var("PWD")
        };

        let pwd = pwd.ok_or_else(|| error("$env.PWD not found", ""))?;

        if let Ok(pwd) = pwd.as_str() {
            let path = AbsolutePathBuf::try_from(pwd)
                .map_err(|_| error("$env.PWD is not an absolute path", pwd))?;

            // Technically, a root path counts as "having trailing slashes", but
            // for the purpose of PWD, a root path is acceptable.
            if path.parent().is_some() && nu_path::has_trailing_slash(path.as_ref()) {
                Err(error("$env.PWD contains trailing slashes", &path))
            } else if !path.exists() {
                Err(error("$env.PWD points to a non-existent directory", &path))
            } else if !path.is_dir() {
                Err(error("$env.PWD points to a non-directory", &path))
            } else {
                Ok(path)
            }
        } else {
            Err(error("$env.PWD is not a string", format!("{pwd:?}")))
        }
    }

    /// Like `EngineState::cwd()`, but returns a String instead of a PathBuf for convenience.
    pub fn cwd_as_string(&self, stack: Option<&Stack>) -> Result<String, ShellError> {
        let cwd = self.cwd(stack)?;
        cwd.into_os_string()
            .into_string()
            .map_err(|err| ShellError::NonUtf8Custom {
                msg: format!(
                    "The current working directory is not a valid utf-8 string: {:?}",
                    err
                ),
                span: Span::unknown(),
            })
    }

    // TODO: see if we can completely get rid of this
    pub fn get_file_contents(&self) -> &[CachedFile] {
        &self.files
    }

    pub fn get_startup_time(&self) -> i64 {
        self.startup_time
    }

    pub fn set_startup_time(&mut self, startup_time: i64) {
        self.startup_time = startup_time;
    }

    pub fn activate_debugger(
        &self,
        debugger: Box<dyn Debugger>,
    ) -> Result<(), PoisonDebuggerError> {
        let mut locked_debugger = self.debugger.lock()?;
        *locked_debugger = debugger;
        locked_debugger.activate();
        self.is_debugging.0.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn deactivate_debugger(&self) -> Result<Box<dyn Debugger>, PoisonDebuggerError> {
        let mut locked_debugger = self.debugger.lock()?;
        locked_debugger.deactivate();
        let ret = std::mem::replace(&mut *locked_debugger, Box::new(NoopDebugger));
        self.is_debugging.0.store(false, Ordering::Relaxed);
        Ok(ret)
    }

    pub fn is_debugging(&self) -> bool {
        self.is_debugging.0.load(Ordering::Relaxed)
    }

    pub fn recover_from_panic(&mut self) {
        if Mutex::is_poisoned(&self.repl_state) {
            self.repl_state = Arc::new(Mutex::new(ReplState {
                buffer: "".to_string(),
                cursor_pos: 0,
            }));
        }
        if Mutex::is_poisoned(&self.jobs) {
            self.jobs = Arc::new(Mutex::new(Jobs::default()));
        }
        if Mutex::is_poisoned(&self.regex_cache) {
            self.regex_cache = Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(REGEX_CACHE_SIZE).expect("tried to create cache of size zero"),
            )));
        }
    }

    /// Add new span and return its ID
    pub fn add_span(&mut self, span: Span) -> SpanId {
        self.spans.push(span);
        SpanId::new(self.num_spans() - 1)
    }

    /// Find ID of a span (should be avoided if possible)
    pub fn find_span_id(&self, span: Span) -> Option<SpanId> {
        self.spans
            .iter()
            .position(|sp| sp == &span)
            .map(SpanId::new)
    }

    // Determines whether the current state is being held by a background job
    pub fn is_background_job(&self) -> bool {
        self.current_thread_job.is_some()
    }
}

impl GetSpan for &EngineState {
    /// Get existing span
    fn get_span(&self, span_id: SpanId) -> Span {
        *self
            .spans
            .get(span_id.get())
            .expect("internal error: missing span")
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod engine_state_tests {
    use crate::{engine::StateWorkingSet, record};
    use std::str::{from_utf8, Utf8Error};

    use super::*;

    #[test]
    fn add_file_gives_id() {
        let engine_state = EngineState::new();
        let mut engine_state = StateWorkingSet::new(&engine_state);
        let id = engine_state.add_file("test.nu".into(), &[]);

        assert_eq!(id, FileId::new(0));
    }

    #[test]
    fn add_file_gives_id_including_parent() {
        let mut engine_state = EngineState::new();
        let parent_id = engine_state.add_file("test.nu".into(), Arc::new([]));

        let mut working_set = StateWorkingSet::new(&engine_state);
        let working_set_id = working_set.add_file("child.nu".into(), &[]);

        assert_eq!(parent_id, FileId::new(0));
        assert_eq!(working_set_id, FileId::new(1));
    }

    #[test]
    fn merge_states() -> Result<(), ShellError> {
        let mut engine_state = EngineState::new();
        engine_state.add_file("test.nu".into(), Arc::new([]));

        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let _ = working_set.add_file("child.nu".into(), &[]);
            working_set.render()
        };

        engine_state.merge_delta(delta)?;

        assert_eq!(engine_state.num_files(), 2);
        assert_eq!(&*engine_state.files[0].name, "test.nu");
        assert_eq!(&*engine_state.files[1].name, "child.nu");

        Ok(())
    }

    #[test]
    fn list_variables() -> Result<(), Utf8Error> {
        let varname = "something";
        let varname_with_sigil = "$".to_owned() + varname;
        let engine_state = EngineState::new();
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_variable(
            varname.as_bytes().into(),
            Span { start: 0, end: 1 },
            Type::Int,
            false,
        );
        let variables = working_set
            .list_variables()
            .into_iter()
            .map(from_utf8)
            .collect::<Result<Vec<&str>, Utf8Error>>()?;
        assert_eq!(variables, vec![varname_with_sigil]);
        Ok(())
    }

    #[test]
    fn get_plugin_config() {
        let mut engine_state = EngineState::new();

        assert!(
            engine_state.get_plugin_config("example").is_none(),
            "Unexpected plugin configuration"
        );

        let mut plugins = HashMap::new();
        plugins.insert("example".into(), Value::string("value", Span::test_data()));

        let mut config = Config::clone(engine_state.get_config());
        config.plugins = plugins;

        engine_state.set_config(config);

        assert!(
            engine_state.get_plugin_config("example").is_some(),
            "Plugin configuration not found"
        );
    }

    #[test]
    fn test_files_memory_size() {
        let mut engine_state = EngineState::new();

        // Initial state should have empty files
        let (stack_size, heap_size) = engine_state.files_memory_size();
        assert!(stack_size > 0, "Stack size should be non-zero");
        assert_eq!(heap_size, 0, "Heap size should be zero for empty files");

        // Add a file with known content
        let content = b"test content";
        let file_name = "test.nu";
        engine_state.add_file(file_name.into(), content.to_vec().into());

        // Calculate expected size manually
        let (_, new_heap_size) = engine_state.files_memory_size();

        // Verify that the size includes at least the content and filename
        let expected_min_size = content.len() + file_name.len();
        assert!(
            new_heap_size >= expected_min_size,
            "Heap size ({}) should be at least the sum of content and filename sizes ({})",
            new_heap_size,
            expected_min_size
        );

        // Add another file and verify the size increases proportionally
        let content2 = b"another test content with more bytes";
        let file_name2 = "test2.nu";
        engine_state.add_file(file_name2.into(), content2.to_vec().into());

        let (_, newer_heap_size) = engine_state.files_memory_size();
        let size_increase = newer_heap_size - new_heap_size;
        let expected_min_increase = content2.len() + file_name2.len();

        assert!(
            size_increase >= expected_min_increase,
            "Size increase ({}) should be at least the sum of new content and filename sizes ({})",
            size_increase,
            expected_min_increase
        );
    }

    #[test]
    fn test_vars_memory_size() {
        let mut engine_state = EngineState::new();

        // Get initial size with predefined variables
        let (_, initial_heap_size) = engine_state.vars_memory_size();

        // Add variables with different types and verify size increases appropriately
        let var_count_before = engine_state.vars.len();

        // Add an integer variable with known size
        let int_val = Value::int(42, Span::test_data());
        let int_val_size = calculate_value_size(&int_val);

        engine_state.vars.push(Variable {
            declaration_span: Span::test_data(),
            ty: Type::Int,
            mutable: false,
            const_val: Some(int_val),
        });

        let (_, int_var_heap_size) = engine_state.vars_memory_size();
        let int_var_added_size = int_var_heap_size - initial_heap_size;

        // Verify that the size increase includes the Value size
        assert!(
            int_var_added_size >= int_val_size,
            "Size increase ({}) should include at least the Value size ({})",
            int_var_added_size,
            int_val_size
        );

        // Add a string variable with known length
        let test_string = "test string value";
        let string_val = Value::string(test_string, Span::test_data());
        let string_val_size = calculate_value_size(&string_val);

        engine_state.vars.push(Variable {
            declaration_span: Span::test_data(),
            ty: Type::String,
            mutable: false,
            const_val: Some(string_val),
        });

        let (_, string_var_heap_size) = engine_state.vars_memory_size();
        let string_var_added_size = string_var_heap_size - int_var_heap_size;

        // Verify that the size increase includes the string Value size
        assert!(
            string_var_added_size >= string_val_size,
            "Size increase ({}) should include at least the string Value size ({})",
            string_var_added_size,
            string_val_size
        );

        // Verify that the string variable size includes the string content
        assert!(
            string_var_added_size >= test_string.len(),
            "String variable size ({}) should include at least the string content size ({})",
            string_var_added_size,
            test_string.len()
        );

        // Verify total count of variables
        assert_eq!(
            engine_state.vars.len(),
            var_count_before + 2,
            "Should have added exactly 2 variables"
        );
    }

    #[test]
    fn test_spans_memory_size() {
        let mut engine_state = EngineState::new();

        // Initial state has one span (UNKNOWN_SPAN)
        let initial_span_count = engine_state.spans.len();
        let (_, initial_heap_size) = engine_state.spans_memory_size();

        // Add multiple spans and check that memory size increases proportionally
        let spans_to_add = 10;
        for i in 0..spans_to_add {
            engine_state.add_span(Span::new(i, i + 10));
        }

        let (_, new_heap_size) = engine_state.spans_memory_size();
        let size_increase = new_heap_size - initial_heap_size;

        // Calculate expected size increase (spans_to_add * size of one Span)
        let expected_increase = spans_to_add * std::mem::size_of::<Span>();

        assert_eq!(
            engine_state.spans.len(),
            initial_span_count + spans_to_add,
            "Should have added exactly {} spans",
            spans_to_add
        );

        assert!(
            size_increase >= expected_increase,
            "Size increase ({}) should be at least the number of spans added times the size of Span ({})",
            size_increase, expected_increase
        );
    }

    #[test]
    fn test_env_vars_memory_size() {
        let mut engine_state = EngineState::new();

        // Get initial size with default env_vars
        let (_, initial_heap_size) = engine_state.env_vars_memory_size();

        // Add environment variables with different sizes
        let small_var_name = "TEST_VAR1";
        let small_var_value = "small value";
        let small_val = Value::string(small_var_value, Span::test_data());

        engine_state.add_env_var(small_var_name.to_string(), small_val);

        let (_, small_var_heap_size) = engine_state.env_vars_memory_size();
        let small_var_added_size = small_var_heap_size - initial_heap_size;

        // Verify that the size increase is reasonable
        assert!(
            small_var_added_size > 0,
            "Size increase should be positive after adding an env var"
        );

        // Verify that the size increase includes at least the string content
        assert!(
            small_var_added_size >= small_var_value.len(),
            "Size increase ({}) should include at least the value content size ({})",
            small_var_added_size,
            small_var_value.len()
        );

        // Add a larger environment variable
        let large_var_name = "TEST_VAR2";
        let large_var_value = "this is a much larger value with more bytes";
        let large_val = Value::string(large_var_value, Span::test_data());

        engine_state.add_env_var(large_var_name.to_string(), large_val);

        let (_, large_var_heap_size) = engine_state.env_vars_memory_size();
        let large_var_added_size = large_var_heap_size - small_var_heap_size;

        // Verify that the size increase is reasonable
        assert!(
            large_var_added_size > 0,
            "Size increase should be positive after adding an env var"
        );

        // Verify that the size increase includes at least the string content
        assert!(
            large_var_added_size >= large_var_value.len(),
            "Size increase ({}) should include at least the value content size ({})",
            large_var_added_size,
            large_var_value.len()
        );

        // Verify that the larger variable takes more space than the smaller one
        assert!(
            large_var_added_size > small_var_added_size,
            "Large var size ({}) should be greater than small var size ({})",
            large_var_added_size,
            small_var_added_size
        );
    }

    #[test]
    fn test_config_memory_size() {
        let mut engine_state = EngineState::new();

        // Get initial size with default config
        let (_, initial_heap_size) = engine_state.config_memory_size();

        // Modify config with known data sizes
        let mut config = Config::clone(engine_state.get_config());

        // Add entries to the plugins HashMap
        let plugin_key = "test_plugin";
        let plugin_value = "test plugin value";
        let plugin_val = Value::string(plugin_value, Span::test_data());
        let plugin_val_size = calculate_value_size(&plugin_val);

        config.plugins.insert(plugin_key.to_string(), plugin_val);

        engine_state.set_config(config);

        let (_, new_heap_size) = engine_state.config_memory_size();
        let size_increase = new_heap_size - initial_heap_size;

        // Calculate the expected minimum size increase
        let expected_min_size =
            // Key string size
            std::mem::size_of::<String>() + plugin_key.len() +
            // Value size
            plugin_val_size;

        // Verify that the size increase includes the key, value, and overhead
        assert!(
            size_increase >= expected_min_size,
            "Size increase ({}) should include at least the key overhead + content and value size ({})",
            size_increase,
            expected_min_size
        );

        // Verify that the size increase includes the plugin value content
        assert!(
            size_increase >= plugin_value.len(),
            "Size increase ({}) should include at least the plugin value content size ({})",
            size_increase,
            plugin_value.len()
        );
    }

    #[test]
    fn test_calculate_value_size() {
        // Test with different value types and verify relative sizes

        // Integer value (fixed size)
        let int_val = Value::int(42, Span::test_data());
        let int_size = calculate_value_size(&int_val);

        // String value with known length
        let string_content = "test string";
        let string_val = Value::string(string_content, Span::test_data());
        let string_size = calculate_value_size(&string_val);

        // String should be larger than int by at least the string length
        assert!(
            string_size > int_size,
            "String value size ({}) should be larger than int value size ({})",
            string_size,
            int_size
        );
        assert!(
            string_size >= int_size + string_content.len(),
            "String value size ({}) should be at least int size ({}) plus string length ({})",
            string_size,
            int_size,
            string_content.len()
        );

        // List with known elements
        let list_val = Value::list(vec![int_val.clone(), string_val.clone()], Span::test_data());
        let list_size = calculate_value_size(&list_val);

        // List should be larger than the sum of its elements
        let elements_size = int_size + string_size;
        assert!(
            list_size > elements_size,
            "List value size ({}) should be larger than sum of element sizes ({})",
            list_size,
            elements_size
        );

        // Record with known fields
        let record_val = Value::record(
            record! {
                "key1" => int_val.clone(),
                "key2" => string_val.clone(),
            },
            Span::test_data(),
        );
        let record_size = calculate_value_size(&record_val);

        // Record should be larger than the sum of its fields plus key names
        let keys_size = "key1".len() + "key2".len();
        assert!(
            record_size > elements_size + keys_size,
            "Record value size ({}) should be larger than sum of element sizes ({}) plus key sizes ({})",
            record_size, elements_size, keys_size
        );

        // Nested structure
        let nested_val = Value::list(
            vec![
                record_val.clone(),
                Value::list(vec![int_val.clone(), string_val.clone()], Span::test_data()),
            ],
            Span::test_data(),
        );
        let nested_size = calculate_value_size(&nested_val);

        // Nested structure should be larger than the sum of its components
        assert!(
            nested_size > record_size + list_size,
            "Nested value size ({}) should be larger than sum of component sizes ({} + {})",
            nested_size,
            record_size,
            list_size
        );
    }
}

#[cfg(test)]
mod test_cwd {
    //! Here're the test cases we need to cover:
    //!
    //! `EngineState::cwd()` computes the result from `self.env_vars["PWD"]` and
    //! optionally `stack.env_vars["PWD"]`.
    //!
    //! PWD may be unset in either `env_vars`.
    //! PWD should NOT be an empty string.
    //! PWD should NOT be a non-string value.
    //! PWD should NOT be a relative path.
    //! PWD should NOT contain trailing slashes.
    //! PWD may point to a directory or a symlink to directory.
    //! PWD should NOT point to a file or a symlink to file.
    //! PWD should NOT point to non-existent entities in the filesystem.

    use crate::{
        engine::{EngineState, Stack},
        Value,
    };
    use nu_path::{assert_path_eq, AbsolutePath, Path};
    use tempfile::{NamedTempFile, TempDir};

    /// Creates a symlink. Works on both Unix and Windows.
    #[cfg(any(unix, windows))]
    fn symlink(
        original: impl AsRef<AbsolutePath>,
        link: impl AsRef<AbsolutePath>,
    ) -> std::io::Result<()> {
        let original = original.as_ref();
        let link = link.as_ref();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(original, link)
        }
        #[cfg(windows)]
        {
            if original.is_dir() {
                std::os::windows::fs::symlink_dir(original, link)
            } else {
                std::os::windows::fs::symlink_file(original, link)
            }
        }
    }

    /// Create an engine state initialized with the given PWD.
    fn engine_state_with_pwd(path: impl AsRef<Path>) -> EngineState {
        let mut engine_state = EngineState::new();
        engine_state.add_env_var(
            "PWD".into(),
            Value::test_string(path.as_ref().to_str().unwrap()),
        );
        engine_state
    }

    /// Create a stack initialized with the given PWD.
    fn stack_with_pwd(path: impl AsRef<Path>) -> Stack {
        let mut stack = Stack::new();
        stack.add_env_var(
            "PWD".into(),
            Value::test_string(path.as_ref().to_str().unwrap()),
        );
        stack
    }

    #[test]
    fn pwd_not_set() {
        let engine_state = EngineState::new();
        engine_state.cwd(None).unwrap_err();
    }

    #[test]
    fn pwd_is_empty_string() {
        let engine_state = engine_state_with_pwd("");
        engine_state.cwd(None).unwrap_err();
    }

    #[test]
    fn pwd_is_non_string_value() {
        let mut engine_state = EngineState::new();
        engine_state.add_env_var("PWD".into(), Value::test_glob("*"));
        engine_state.cwd(None).unwrap_err();
    }

    #[test]
    fn pwd_is_relative_path() {
        let engine_state = engine_state_with_pwd("./foo");

        engine_state.cwd(None).unwrap_err();
    }

    #[test]
    fn pwd_has_trailing_slash() {
        let dir = TempDir::new().unwrap();
        let engine_state = engine_state_with_pwd(dir.path().join(""));

        engine_state.cwd(None).unwrap_err();
    }

    #[test]
    fn pwd_points_to_root() {
        #[cfg(windows)]
        let root = Path::new(r"C:\");
        #[cfg(not(windows))]
        let root = Path::new("/");

        let engine_state = engine_state_with_pwd(root);
        let cwd = engine_state.cwd(None).unwrap();
        assert_path_eq!(cwd, root);
    }

    #[test]
    fn pwd_points_to_normal_file() {
        let file = NamedTempFile::new().unwrap();
        let engine_state = engine_state_with_pwd(file.path());

        engine_state.cwd(None).unwrap_err();
    }

    #[test]
    fn pwd_points_to_normal_directory() {
        let dir = TempDir::new().unwrap();
        let engine_state = engine_state_with_pwd(dir.path());

        let cwd = engine_state.cwd(None).unwrap();
        assert_path_eq!(cwd, dir.path());
    }

    #[test]
    fn pwd_points_to_symlink_to_file() {
        let file = NamedTempFile::new().unwrap();
        let temp_file = AbsolutePath::try_new(file.path()).unwrap();
        let dir = TempDir::new().unwrap();
        let temp = AbsolutePath::try_new(dir.path()).unwrap();

        let link = temp.join("link");
        symlink(temp_file, &link).unwrap();
        let engine_state = engine_state_with_pwd(&link);

        engine_state.cwd(None).unwrap_err();
    }

    #[test]
    fn pwd_points_to_symlink_to_directory() {
        let dir = TempDir::new().unwrap();
        let temp = AbsolutePath::try_new(dir.path()).unwrap();

        let link = temp.join("link");
        symlink(temp, &link).unwrap();
        let engine_state = engine_state_with_pwd(&link);

        let cwd = engine_state.cwd(None).unwrap();
        assert_path_eq!(cwd, link);
    }

    #[test]
    fn pwd_points_to_broken_symlink() {
        let dir = TempDir::new().unwrap();
        let temp = AbsolutePath::try_new(dir.path()).unwrap();
        let other_dir = TempDir::new().unwrap();
        let other_temp = AbsolutePath::try_new(other_dir.path()).unwrap();

        let link = temp.join("link");
        symlink(other_temp, &link).unwrap();
        let engine_state = engine_state_with_pwd(&link);

        drop(other_dir);
        engine_state.cwd(None).unwrap_err();
    }

    #[test]
    fn pwd_points_to_nonexistent_entity() {
        let engine_state = engine_state_with_pwd(TempDir::new().unwrap().path());

        engine_state.cwd(None).unwrap_err();
    }

    #[test]
    fn stack_pwd_not_set() {
        let dir = TempDir::new().unwrap();
        let engine_state = engine_state_with_pwd(dir.path());
        let stack = Stack::new();

        let cwd = engine_state.cwd(Some(&stack)).unwrap();
        assert_eq!(cwd, dir.path());
    }

    #[test]
    fn stack_pwd_is_empty_string() {
        let dir = TempDir::new().unwrap();
        let engine_state = engine_state_with_pwd(dir.path());
        let stack = stack_with_pwd("");

        engine_state.cwd(Some(&stack)).unwrap_err();
    }

    #[test]
    fn stack_pwd_points_to_normal_directory() {
        let dir1 = TempDir::new().unwrap();
        let dir2 = TempDir::new().unwrap();
        let engine_state = engine_state_with_pwd(dir1.path());
        let stack = stack_with_pwd(dir2.path());

        let cwd = engine_state.cwd(Some(&stack)).unwrap();
        assert_path_eq!(cwd, dir2.path());
    }

    #[test]
    fn stack_pwd_points_to_normal_directory_with_symlink_components() {
        let dir = TempDir::new().unwrap();
        let temp = AbsolutePath::try_new(dir.path()).unwrap();

        // `/tmp/dir/link` points to `/tmp/dir`, then we set PWD to `/tmp/dir/link/foo`
        let link = temp.join("link");
        symlink(temp, &link).unwrap();
        let foo = link.join("foo");
        std::fs::create_dir(temp.join("foo")).unwrap();
        let engine_state = EngineState::new();
        let stack = stack_with_pwd(&foo);

        let cwd = engine_state.cwd(Some(&stack)).unwrap();
        assert_path_eq!(cwd, foo);
    }
}
