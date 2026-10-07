//! Persisted programs ("pickles"): the output of [`parse`](crate::parse), IR included, written
//! to bytes so a later process can run it without parsing and compiling the source again.
//!
//! An [`IrBlock`](nu_protocol::ir::IrBlock) alone is not executable: its instructions refer to
//! declarations, blocks, variables, modules and source spans by index into engine-state tables.
//! So a pickle holds everything a parse adds to the working set (files, variables, declarations,
//! blocks, modules and the scope frames that name them) plus the top-level block. The program is
//! parsed [`standalone`](StateWorkingSet::standalone), so that is everything it parsed, the
//! standard library modules it uses included.
//!
//! Loading works like linking an object file (see [`nu_protocol::relocation`]):
//!
//! - State the parse created is appended to the loading working set in the original order, and
//!   every reference to it is shifted by how much the tables grew or shrank in between.
//! - References to state that existed before the parse ("imports") are rebound. Declarations are
//!   looked up by the names the program called them by and must have the same [`DeclShape`] as
//!   when the program was compiled; constants are looked up by name. Anything that can't be
//!   rebound makes loading fail instead of running code against the wrong declaration.
//!
//! Pickles are portable. The program's own files are stored with paths relative to the script's
//! directory and resolved against the pickle's directory when it is loaded, and everything else
//! is found by name, so a pickle can be moved or copied to another machine.
//!
//! The format is conservative: pickles only load into the same nushell version and build (git
//! commit), and a checksum turns a damaged pickle into an error. See `devdocs/persistent_ir.md`
//! for the design.

use crate::{KnownExternal, type_check::type_compatible};
use nu_engine::HELP_DECL_ID_PARSER_INFO;
use nu_protocol::{
    Alias, BlockId, Category, CustomExample, DeclId, LAST_VARIABLE_ID, LabeledError, Module,
    ModuleId, ParseError, ShellError, Signature, Span, Type, Value, VarId,
    ast::{Block, Expr, Expression, Traverse},
    engine::{
        Command, CommandType, EngineState, ParseTimeValue, ScopeFrame, StateWorkingSet, Variable,
    },
    eval_const::eval_constant,
    ir::Instruction,
    parser_path::ParserPath,
    relocation::{self, IdBases, IdKind, IdRelocator, ImportCollector},
};
use postcard::de_flavors::{Flavor, Slice};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Seek},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

/// Leading bytes of every pickle. The NUL bytes keep it from ever being valid source code.
pub const MAGIC: &[u8] = b"\0nu-pickle\0";

/// Bump whenever the layout of [`PickleHeader`], [`Linkage`] or [`Body`] changes. It is written as
/// four little-endian bytes right after [`MAGIC`], so any later layout can still be told apart.
const FORMAT_VERSION: u32 = 8;

/// The git commit of this build (see `build.rs`), empty when the build couldn't tell.
const NU_COMMIT_HASH: &str = env!("NU_COMMIT_HASH");

/// The most an LZ4 block can expand. Bounds the size prefix of a damaged pickle.
const LZ4_MAX_RATIO: usize = 255;

/// The most stack decoding a pickle's body may use. Postcard has no recursion limit, so a crafted
/// pickle nested deeply enough would overflow the stack instead of failing to load.
const MAX_DECODE_STACK: usize = 1 << 20;

/// Parser info entries that hold an id as a plain `Expr::Int`. The relocation hooks only see
/// typed ids, so these are rewritten by hand.
const PARSER_INFO_IDS: &[(&str, IdKind)] = &[
    ("block_id", IdKind::Block),
    ("main_block_id", IdKind::Block),
    (HELP_DECL_ID_PARSER_INFO, IdKind::Decl),
];

/// Who made a pickle. Loading requires the version and build to match the nushell that loads it.
///
/// Everything after the format version is postcard, which writes enum variants as numbers and
/// nothing else about the types. Two builds whose serialized types differ could misread each
/// other's pickles instead of failing to decode, so the build is checked, not just the version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PickleHeader {
    pub nu_version: String,
    /// The git commit of the build (what `version` reports as `commit_hash`), empty when the
    /// build couldn't tell.
    pub build: String,
    /// Every experimental option and whether it was enabled. They can change what the parser and
    /// compiler emit, so `nu` runs a pickle with them instead of its own.
    pub experimental_options: Vec<(String, bool)>,
}

impl PickleHeader {
    /// The header this nushell writes.
    fn current() -> Self {
        Self {
            nu_version: env!("CARGO_PKG_VERSION").into(),
            build: NU_COMMIT_HASH.into(),
            experimental_options: nu_experimental::ALL
                .iter()
                .map(|option| (option.identifier().into(), option.get()))
                .collect(),
        }
    }

    /// Everything about this header that keeps its pickle from loading into this nushell, one
    /// sentence each. Empty if the pickle loads.
    fn differences(&self) -> Vec<String> {
        let (version, build) = (env!("CARGO_PKG_VERSION"), NU_COMMIT_HASH);
        let commit = |build: &str| match build {
            "" => "an unknown commit".to_string(),
            build => build.to_string(),
        };

        let mut differences = vec![];
        if self.nu_version != version {
            differences.push(format!(
                "expected version {version} and got {}",
                self.nu_version
            ));
        }
        if self.build != build {
            differences.push(format!(
                "expected commit {} and got {}",
                commit(build),
                commit(&self.build)
            ));
        }
        differences
    }
}

/// Everything the loader has to resolve before it can read the [`Body`].
///
/// Written and read without the relocation hooks, so it holds the producer's raw ids.
#[derive(Serialize, Deserialize)]
struct Linkage {
    bases: IdBases,
    decls: Vec<DeclImport>,
    vars: Vec<VarImport>,
}

#[derive(Serialize, Deserialize)]
struct DeclImport {
    id: usize,
    /// The names the pickling shell calls the declaration by, sorted. Every one of them has to
    /// name the same declaration where the pickle runs.
    names: Vec<String>,
    shape: DeclShape,
}

/// A variable from outside the program: one of the built-in ones (`$nu`, `$in`, `$env`, `$ans`),
/// which have fixed ids, or a constant such as `$NU_LIB_DIRS`, which is found by name like the
/// source would.
#[derive(Serialize, Deserialize)]
struct VarImport {
    id: usize,
    name: String,
}

/// What parsing and compiling a call relied on about the declaration it calls.
///
/// The parser shaped the arguments after the signature (which ones are closures, which flags
/// take values), and the compiler looked at the command type, whether the command wants the AST
/// of its arguments and its default redirections. If any of those differ, the IR is wrong for
/// the declaration even when the name is the same.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct DeclShape {
    command_type: CommandType,
    /// Hash of the signature without names, descriptions, variables, defaults and completions,
    /// plus `requires_ast_for_arguments` and `pipe_redirection`.
    hash: u64,
}

impl DeclShape {
    fn of(decl: &dyn Command) -> Result<Self, ShellError> {
        let mut signature = decl.signature();
        signature.name.clear();
        signature.description.clear();
        signature.extra_description.clear();
        signature.search_terms.clear();
        signature.category = Category::Default;
        signature.complete = None;
        for positional in signature
            .required_positional
            .iter_mut()
            .chain(&mut signature.optional_positional)
            .chain(&mut signature.rest_positional)
        {
            positional.desc.clear();
            positional.completion = None;
            positional.var_id = None;
            positional.default_value = None;
        }
        for flag in &mut signature.named {
            flag.desc.clear();
            flag.completion = None;
            flag.var_id = None;
            flag.default_value = None;
        }

        let mut shape = encode(&signature)?;
        shape.push(u8::from(decl.requires_ast_for_arguments()));
        shape.extend(format!("{:?}", decl.pipe_redirection()).bytes());
        Ok(Self {
            command_type: decl.command_type(),
            hash: content_hash(&shape),
        })
    }
}

/// The program itself. Written and read with the relocation hooks active, and LZ4 compressed.
///
/// The span id table the parser filled is left out; see [`IdKind::Span`].
#[derive(Serialize, Deserialize)]
struct Body {
    files: Vec<PickledFile>,
    vars: Vec<Variable>,
    /// How the parser computed the values it stored, so loading can compute them again on the
    /// machine the program runs on.
    parse_time_values: Vec<(VarId, ParseTimeValue)>,
    decls: Vec<PickledDecl>,
    blocks: Vec<Block>,
    modules: Vec<PickledModule>,
    scope: Vec<ScopeFrame>,
    block: Block,
}

/// A source file of the program. Embedded so errors can show source and spans stay valid.
#[derive(Serialize, Deserialize)]
struct PickledFile {
    /// Relative to the script's directory if `relative`, see [`relative_path`]. Otherwise a name
    /// that isn't a path, or a path that can't be made relative.
    name: String,
    relative: bool,
    content: Vec<u8>,
    span: Span,
}

/// The declarations a parse can create. Other kinds (plugins, predeclarations) are refused.
#[derive(Serialize, Deserialize)]
enum PickledDecl {
    Custom {
        signature: Box<Signature>,
        block_id: BlockId,
        attributes: Vec<(String, Value)>,
        examples: Vec<CustomExample>,
    },
    Extern {
        signature: Box<Signature>,
        attributes: Vec<(String, Value)>,
        examples: Vec<CustomExample>,
        span: Span,
    },
    Alias {
        name: String,
        wrapped_call: Expression,
        description: String,
        extra_description: String,
    },
}

impl PickledDecl {
    fn of(decl: &dyn Command) -> Result<Self, ShellError> {
        if let Some(alias) = decl.as_alias() {
            return Ok(PickledDecl::Alias {
                name: alias.name.clone(),
                wrapped_call: alias.wrapped_call.clone(),
                description: alias.description.clone(),
                extra_description: alias.extra_description.clone(),
            });
        }

        let attributes = decl.attributes();
        let examples = decl
            .examples()
            .into_iter()
            .map(|example| CustomExample {
                example: example.example.into(),
                description: example.description.into(),
                result: example.result,
            })
            .collect();
        let signature = Box::new(decl.signature());
        match (decl.command_type(), decl.block_id(), decl.decl_span()) {
            (CommandType::Custom, Some(block_id), _) => Ok(PickledDecl::Custom {
                signature,
                block_id,
                attributes,
                examples,
            }),
            (CommandType::External, None, Some(span)) => Ok(PickledDecl::Extern {
                signature,
                attributes,
                examples,
                span,
            }),
            (command_type, ..) => Err(save_error(format!(
                "the {command_type} command `{}` can't be pickled",
                decl.name()
            ))),
        }
    }

    /// Rebuild the declaration, once the program's blocks and the declarations before it are in
    /// `working_set`.
    fn into_command(self, working_set: &StateWorkingSet) -> Box<dyn Command> {
        match self {
            PickledDecl::Custom {
                signature,
                block_id,
                attributes,
                examples,
            } => (*signature).into_block_command(block_id, attributes, examples),
            PickledDecl::Extern {
                signature,
                attributes,
                examples,
                span,
            } => Box::new(KnownExternal {
                signature,
                attributes,
                examples,
                span,
            }),
            PickledDecl::Alias {
                name,
                wrapped_call,
                description,
                extra_description,
            } => {
                // Like the parser, keep a copy of the aliased command for its signature. Aliases
                // can't see later definitions, so a local target is in the working set already.
                let command = match &wrapped_call.expr {
                    Expr::Call(call) => Some(working_set.get_decl(call.decl_id).clone_box()),
                    _ => None,
                };
                Box::new(Alias {
                    name,
                    command,
                    wrapped_call,
                    description,
                    extra_description,
                })
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PickledModule {
    module: Module,
    comments: Vec<Span>,
}

/// Whether `contents` is a pickle rather than source code.
pub fn is_pickle(contents: &[u8]) -> bool {
    contents.starts_with(MAGIC)
}

/// Whether the file at `path` is a pickle, told by its first bytes alone, so that `run` can tell
/// without reading a large file whole.
///
/// Only a regular file can be a pickle: reading a pipe or FIFO here would take its bytes from
/// whoever reads it next. The file is rewound afterwards, because `/dev/stdin` redirected from a
/// file shares its offset with the next reader that opens it.
pub fn is_pickle_file(path: &Path) -> bool {
    if !path.metadata().is_ok_and(|metadata| metadata.is_file()) {
        return false;
    }
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut magic = Vec::with_capacity(MAGIC.len());
    let read = (&mut file)
        .take(MAGIC.len() as u64)
        .read_to_end(&mut magic)
        .is_ok();
    let rewound = file.rewind().is_ok();
    read && rewound && is_pickle(&magic)
}

/// The error for a pickle that `source` or `use` would parse as source code. Only `nu` and `run`
/// run one.
pub(crate) fn sourced_pickle_error(span: Span) -> ParseError {
    ParseError::LabeledErrorWithHelp {
        error: "Can't read a pickle as source code".into(),
        label: "this file was made by `pickle`".into(),
        help: "run it with `nu` or `run`, or get its source back with `depickle`".into(),
        span,
    }
}

/// Refuse a pickle made with other experimental options than the ones this nushell runs with.
///
/// The options can change what the parser and compiler emit. `nu` sets a pickle's options at
/// startup, before anything reads them, but a shell that loads a pickle while it runs, as `run`
/// does, can't change its own. A pickle [`load`] refuses for another reason passes, so that `load`
/// reports that reason.
pub fn check_experimental_options(contents: &[u8]) -> Result<(), ShellError> {
    let (info, _) = read_header(contents)?;
    let Some(header) = info.header.filter(|_| info.problems.is_empty()) else {
        return Ok(());
    };
    let state = |enabled: bool| if enabled { "enabled" } else { "disabled" };
    let differences: Vec<_> = header
        .experimental_options
        .iter()
        .filter_map(|(name, enabled)| {
            let option = nu_experimental::ALL
                .iter()
                .find(|option| option.identifier() == name)?;
            (option.get() != *enabled).then(|| {
                format!(
                    "expected experimental option `{name}` {} and got {}",
                    state(option.get()),
                    state(*enabled)
                )
            })
        })
        .collect();
    if differences.is_empty() {
        return Ok(());
    }
    Err(LabeledError::new(
        "Can't load pickled program: it was made with different experimental options",
    )
    .with_code("nu::pickle::experimental_options")
    .with_help(format!(
        "{}\npickle the source again in this shell, or run the pickle with `nu`",
        differences.join("\n")
    ))
    .into())
}

/// Serialize the program [`parse`](crate::parse) just produced into `working_set`, with `block`
/// being the block it returned. The working set has to be
/// [`standalone`](StateWorkingSet::standalone), so the program doesn't share what it parsed with
/// the shell, and should parse into the default overlay, where the program will run.
///
/// Paths of the program's files are stored relative to `root`, the script's directory. [`load`]
/// only accepts the result in the same nushell version and build (git commit).
pub fn save(
    working_set: &StateWorkingSet,
    block: &Block,
    root: &Path,
) -> Result<Vec<u8>, ShellError> {
    if !working_set.standalone {
        return Err(save_error("the program wasn't parsed standalone"));
    }
    if !working_set.parse_errors.is_empty() || !working_set.compile_errors.is_empty() {
        return Err(save_error("the program has parse or compile errors"));
    }
    if working_set.delta.num_virtual_paths() > 0 {
        return Err(save_error("the program adds virtual paths"));
    }

    let permanent = working_set.permanent_state;
    let bases = IdBases::of(permanent);
    let carried: HashSet<&str> = working_set
        .files()
        .skip(bases.files)
        .map(|file| &*file.name)
        .collect();
    // Constants are computed again when the pickle is loaded, by the AST evaluator, so the blocks
    // their expressions reach (like `(...)` subexpressions) keep their AST.
    let mut needs_ast = HashSet::new();
    for (_, value) in &working_set.parse_time_values {
        if let ParseTimeValue::Expression(expr, _) = value {
            let mut block_ids = vec![];
            expr.flat_map(
                working_set,
                &|expr: &Expression| match expr.expr {
                    Expr::Subexpression(block_id)
                    | Expr::Block(block_id)
                    | Expr::Closure(block_id)
                    | Expr::RowCondition(block_id) => vec![block_id.get()],
                    _ => vec![],
                },
                &mut block_ids,
            );
            needs_ast.extend(block_ids);
        }
    }
    let mut body = Body {
        files: working_set
            .files()
            .skip(bases.files)
            .map(|file| {
                let relative = relative_path(Path::new(&*file.name), root);
                PickledFile {
                    relative: relative.is_some(),
                    name: relative.unwrap_or_else(|| file.name.to_string()),
                    content: file.content.to_vec(),
                    span: file.covered_span,
                }
            })
            .collect(),
        vars: (bases.vars..working_set.num_vars())
            .map(|id| working_set.get_variable(VarId::new(id)).clone())
            .collect(),
        parse_time_values: working_set
            .parse_time_values
            .iter()
            .filter(|(var_id, _)| var_id.get() >= bases.vars)
            .cloned()
            .collect(),
        decls: (bases.decls..working_set.num_decls())
            .map(|id| PickledDecl::of(working_set.get_decl(DeclId::new(id))))
            .collect::<Result<_, _>>()?,
        blocks: working_set
            .delta
            .blocks
            .iter()
            .enumerate()
            .map(|(index, block)| stored_block(block, needs_ast.contains(&(bases.blocks + index))))
            .collect(),
        modules: (bases.modules..working_set.num_modules())
            .map(ModuleId::new)
            .map(|id| {
                let mut module = working_set.get_module(id).clone();
                // The parser only records absolute paths here, so a relative one is ours.
                if let Some((ParserPath::RealPath(path), _)) = &mut module.file
                    && let Some(relative) = relative_path(path, root)
                {
                    *path = PathBuf::from(relative);
                }
                PickledModule {
                    module,
                    comments: working_set
                        .get_module_comments(id)
                        .map(<[Span]>::to_vec)
                        .unwrap_or_default(),
                }
            })
            .collect(),
        scope: working_set.delta.scope.clone(),
        block: stored_block(block, false),
    };
    for block in body.blocks.iter_mut().chain([&mut body.block]) {
        // `run --full-reparse` reads its file again, so a file the program doesn't carry keeps
        // the path the parser found.
        map_file_names(block, |path| {
            relative_path(Path::new(path), root).filter(|_| carried.contains(path))
        });
        // A constant from outside the program is read from the engine where it runs, so a
        // capture of it (like the ones a called command passes up to its caller) needs no link.
        block.captures.retain(|(var_id, _)| {
            var_id.get() <= LAST_VARIABLE_ID.get()
                || var_id.get() >= bases.vars
                || permanent.get_constant(*var_id).is_none()
        });
    }

    let (encoded, collector) = relocation::collect_imports(ImportCollector::new(bases), || {
        for block in body.blocks.iter_mut().chain([&mut body.block]) {
            for_each_parser_info_id(block, |kind, id| {
                relocation::note_id(kind, id);
                Ok(id)
            })?;
        }
        encode(&body)
    });
    let encoded = encoded?;
    // A pickle that saves has to load: postcard can't decode everything serde can describe.
    // Without a relocation context this decodes ids unchanged.
    postcard::from_bytes::<Body>(&encoded)
        .map_err(|err| save_error(format!("the program doesn't read back: {err}")))?;

    let mut linkage = Linkage {
        bases,
        decls: vec![],
        vars: vec![],
    };
    for (kind, id) in collector.imports() {
        match kind {
            IdKind::Decl => {
                let decl_id = DeclId::new(id);
                linkage.decls.push(DeclImport {
                    id,
                    names: decl_names(permanent, decl_id)?,
                    shape: DeclShape::of(permanent.get_decl(decl_id))?,
                });
            }
            IdKind::Var if id <= LAST_VARIABLE_ID.get() => linkage.vars.push(VarImport {
                id,
                name: String::new(),
            }),
            IdKind::Var => {
                // A constant is found again by the name this shell makes it visible under.
                let var_id = VarId::new(id);
                let name = variable_name(permanent, var_id)
                    .filter(|_| permanent.get_constant(var_id).is_some())
                    .ok_or_else(|| {
                        let name = permanent.get_var(var_id).name.as_deref().unwrap_or(b"?");
                        save_error(format!(
                            "the program uses the variable `{}` from outside of it",
                            String::from_utf8_lossy(name)
                        ))
                    })?;
                linkage.vars.push(VarImport { id, name });
            }
            // The origin of the default overlay, which every engine state has in the same place.
            IdKind::Module if id == ModuleId::ZERO.get() => (),
            kind => {
                return Err(save_error(format!(
                    "the program uses {kind:?} {id} from outside of it"
                )));
            }
        }
    }

    let mut linked = encode(&linkage)?;
    // LZ4 because loading is what has to be fast: about 3.4 times smaller, and it decompresses
    // about 5 times faster than brotli, which would be 2.7 times smaller still.
    linked.extend(lz4_flex::compress_prepend_size(&encoded));
    let mut bytes = MAGIC.to_vec();
    bytes.extend(FORMAT_VERSION.to_le_bytes());
    bytes.extend(encode(&PickleHeader::current())?);
    // Neither postcard nor LZ4 notice damage, which could make any id point anywhere.
    bytes.extend(content_hash(&linked).to_le_bytes());
    bytes.extend(linked);
    Ok(bytes)
}

/// What a pickle says about itself, readable even when this nushell can't load it.
pub struct PickleInfo {
    pub format_version: u32,
    /// `None` when the pickle's format isn't this nushell's, so its header can't be read.
    pub header: Option<PickleHeader>,
    /// Why this nushell can't load the pickle, one sentence each; empty if it can.
    pub problems: Vec<String>,
}

/// Read the header of a pickle and compare it with this nushell.
pub fn info(contents: &[u8]) -> Result<PickleInfo, ShellError> {
    read_header(contents).map(|(info, _)| info)
}

/// The source files a pickle carries, by name: the script and every file it `use`s or `source`s,
/// with the paths of the script's own files relative to its directory and `/` separators. Only
/// pickles this nushell can load can be read.
pub fn sources(contents: &[u8]) -> Result<Vec<(String, Vec<u8>)>, ShellError> {
    let (_, reader) = read_linkage(contents)?;
    // Without a relocation context, ids decode as they were written; only the files are used.
    let body: Body = decode(&decompress(reader)?)?;
    Ok(body
        .files
        .into_iter()
        .map(|file| (file.name, file.content))
        .collect())
}

/// Load a pickle made by [`save`] into `working_set`, which must not contain any changes yet and
/// is made [`standalone`](StateWorkingSet::standalone), so the program's files are its own even
/// where this engine has loaded the same ones.
///
/// The program's file paths are resolved against `base`, the pickle's directory.
///
/// The experimental options aren't compared. The caller runs the program with the ones in the
/// pickle's header, which `nu` sets at startup before anything reads them. A caller that can't set
/// them, like `run`, refuses a pickle with other ones through [`check_experimental_options`].
///
/// The values the parser computed (`const` values, parameter defaults) are computed again here,
/// on this machine, as parsing the source would. If one fails, the error is added to
/// `working_set.parse_errors`, as parsing would have.
///
/// Returns the top-level block. Like after [`parse`](crate::parse), the working set holds
/// everything else the program needs and has to be merged before the block is evaluated.
pub fn load(
    working_set: &mut StateWorkingSet,
    contents: &[u8],
    base: &Path,
) -> Result<Arc<Block>, ShellError> {
    let (linkage, reader) = read_linkage(contents)?;

    let delta = &working_set.delta;
    if delta.num_files() + delta.num_vars() + delta.num_decls() + delta.num_blocks() > 0
        || delta.num_modules() > 0
    {
        return Err(load_error("the working set already has changes"));
    }
    working_set.standalone = true;
    let relocator = link(working_set, &linkage)?;

    let encoded = decompress(reader)?;
    let (body, relocator) = relocation::relocate(relocator, || decode::<Body>(&encoded));
    let mut body = body?;
    let carried: HashSet<&str> = body
        .files
        .iter()
        .filter(|file| file.relative)
        .map(|file| file.name.as_str())
        .collect();
    for block in body.blocks.iter_mut().chain([&mut body.block]) {
        for_each_parser_info_id(block, |kind, id| {
            relocator.map_id(kind, id).map_err(load_error)
        })?;
        map_file_names(block, |path| {
            carried
                .contains(path)
                .then(|| resolve_path(path, base).to_string_lossy().into_owned())
        });
    }

    for file in body.files {
        let name = match file.relative {
            true => resolve_path(&file.name, base)
                .to_string_lossy()
                .into_owned(),
            false => file.name,
        };
        let _ = working_set.add_file(&name, &file.content);
    }
    for variable in body.vars {
        // Only allocates the id; the variable replaces what it adds.
        let var_id =
            working_set.add_variable_without_scope(Span::unknown(), Type::Any, variable.mutable);
        if let Some(slot) = working_set.get_variable_mut(var_id) {
            *slot = variable;
        }
    }

    // Compute the parse-time values again, here, the way the parser did. Constants update their
    // variable, parameter defaults the signatures of the blocks, before the commands are built
    // from them. The blocks and modules go in first: constants can contain subexpressions, and
    // `$module` records are built from the module.
    let blocks_base = working_set.num_blocks();
    for block in body.blocks {
        working_set.add_block(Arc::new(block));
    }
    for PickledModule {
        mut module,
        comments,
    } in body.modules
    {
        if let Some((ParserPath::RealPath(path), _)) = &mut module.file
            && path.is_relative()
        {
            *path = resolve_path(&path.to_string_lossy(), base);
        }
        let name = String::from_utf8_lossy(&module.name).into_owned();
        working_set.add_module(&name, module, comments);
    }
    let mut defaults = HashMap::new();
    for (var_id, value) in body.parse_time_values {
        let value = match value {
            ParseTimeValue::Expression(expr, declared) => {
                let value = match evaluate_again(working_set, &expr) {
                    Ok(value) => value,
                    Err(err) => {
                        let err = err.wrap(working_set, expr.span);
                        working_set.error(err);
                        continue;
                    }
                };
                // Like `const x: T = ...` does when parsing.
                let (found, span) = (value.get_type(), value.span());
                match declared {
                    Some(declared) if !type_compatible(&declared, &found) => {
                        working_set.error(ParseError::TypeMismatch(declared, found, expr.span));
                        continue;
                    }
                    Some(Type::Glob) => match value {
                        Value::String { val, .. } => Value::glob(val, false, span),
                        value => value,
                    },
                    _ => value,
                }
            }
            ParseTimeValue::ModuleConstants(module_id) => {
                let span = working_set.get_variable(var_id).declaration_span;
                let (record, errors) = working_set
                    .get_module(module_id)
                    .constants_record(working_set, span);
                working_set.parse_errors.extend(errors);
                let Some(record) = record else { continue };
                record
            }
        };
        if working_set.get_variable(var_id).const_val.is_some() {
            working_set.set_variable_type(var_id, value.get_type());
            working_set.set_variable_const_val(var_id, value);
        } else {
            defaults.insert(var_id, value);
        }
    }
    if !defaults.is_empty() {
        for block_id in blocks_base..working_set.num_blocks() {
            set_defaults(
                &mut working_set.get_block_mut(BlockId::new(block_id)).signature,
                &defaults,
            );
        }
        for decl in &mut body.decls {
            if let PickledDecl::Custom { signature, .. } | PickledDecl::Extern { signature, .. } =
                decl
            {
                set_defaults(signature, &defaults);
            }
        }
    }

    for decl in body.decls {
        let decl = decl.into_command(working_set);
        working_set.add_decl(decl);
    }
    // `add_decl` and `add_module` bound names in the current overlay; the pickled scope frames
    // are the ones the parser left behind.
    working_set.delta.scope = body.scope;

    Ok(Arc::new(body.block))
}

/// A pickle's format version and, if it is this nushell's format, its header, compared with this
/// nushell. Also returns the bytes after them.
fn read_header(contents: &[u8]) -> Result<(PickleInfo, &[u8]), ShellError> {
    let contents = contents
        .strip_prefix(MAGIC)
        .ok_or_else(|| load_error("the file is not a pickle"))?;
    let (format_version, reader) = contents
        .split_first_chunk()
        .ok_or_else(|| load_error("the pickle is damaged: it has no format version"))?;
    let format_version = u32::from_le_bytes(*format_version);
    if format_version != FORMAT_VERSION {
        let problems = vec![format!(
            "expected pickle format {FORMAT_VERSION} and got {format_version}"
        )];
        let info = PickleInfo {
            format_version,
            header: None,
            problems,
        };
        return Ok((info, reader));
    }
    let (header, reader): (PickleHeader, _) =
        postcard::take_from_bytes(reader).map_err(decode_error)?;
    let info = PickleInfo {
        format_version,
        problems: header.differences(),
        header: Some(header),
    };
    Ok((info, reader))
}

/// Read a pickle up to its body, refusing it unless it was made by this nushell and is intact.
fn read_linkage(contents: &[u8]) -> Result<(Linkage, &[u8]), ShellError> {
    let (info, reader) = read_header(contents)?;
    if !info.problems.is_empty() {
        let error =
            LabeledError::new("Can't load pickled program: it was made by a different nushell")
                .with_code("nu::pickle::incompatible")
                .with_help(format!(
                    "{}\npickle the source again with this nushell",
                    info.problems.join("\n")
                ));
        return Err(error.into());
    }
    let (checksum, reader) = reader
        .split_first_chunk()
        .ok_or_else(|| load_error("the pickle is damaged: it has no checksum"))?;
    if u64::from_le_bytes(*checksum) != content_hash(reader) {
        return Err(load_error(
            "the pickle is damaged: its checksum doesn't match",
        ));
    }
    postcard::take_from_bytes(reader).map_err(decode_error)
}

/// Resolve what the program needs from outside of it against `working_set`.
fn link(working_set: &StateWorkingSet, linkage: &Linkage) -> Result<IdRelocator, ShellError> {
    let mut relocator = IdRelocator::new(linkage.bases, IdBases::of(working_set.permanent_state));
    for import in &linkage.vars {
        let var_id = if import.id <= LAST_VARIABLE_ID.get() {
            VarId::new(import.id)
        } else {
            working_set
                .find_variable(import.name.as_bytes())
                .filter(|var_id| working_set.get_constant(*var_id).is_ok())
                .ok_or_else(|| {
                    ShellError::from(
                        LabeledError::new(format!(
                            "Can't load pickled program: the constant `{}` is not defined",
                            import.name
                        ))
                        .with_help(DEFINED_BY_SHELL_HELP),
                    )
                })?
        };
        relocator.bind(IdKind::Var, import.id, var_id.get());
    }
    relocator.bind(IdKind::Module, ModuleId::ZERO.get(), ModuleId::ZERO.get());
    for import in &linkage.decls {
        let decl_id = resolve_decl(working_set, import)?;
        relocator.bind(IdKind::Decl, import.id, decl_id.get());
    }
    Ok(relocator)
}

/// The encoded body after the linkage.
fn decompress(reader: &[u8]) -> Result<Vec<u8>, ShellError> {
    let damaged = |err: String| load_error(format!("the pickle is damaged: {err}"));
    let (size, compressed) =
        lz4_flex::block::uncompressed_size(reader).map_err(|err| damaged(err.to_string()))?;
    if size > compressed.len().saturating_mul(LZ4_MAX_RATIO) {
        return Err(damaged(format!(
            "{size} bytes can't come from {}",
            compressed.len()
        )));
    }
    lz4_flex::decompress(compressed, size).map_err(|err| damaged(err.to_string()))
}

/// Decode `encoded`, refusing values nested so deeply that decoding them would take more than
/// [`MAX_DECODE_STACK`].
fn decode<T: DeserializeOwned>(encoded: &[u8]) -> Result<T, ShellError> {
    let too_deep = Cell::new(false);
    let mut deserializer = postcard::Deserializer::from_flavor(StackLimit {
        slice: Slice::new(encoded),
        base: stack_position(),
        too_deep: &too_deep,
    });
    T::deserialize(&mut deserializer).map_err(|err| match too_deep.get() {
        true => load_error("the pickle is nested too deeply"),
        false => decode_error(err),
    })
}

/// Reads a [`Slice`] and fails once decoding uses more than [`MAX_DECODE_STACK`] of stack.
/// Every level of nesting reads at least one byte, so it is checked on every read.
struct StackLimit<'de> {
    slice: Slice<'de>,
    /// [`stack_position`] when decoding started.
    base: usize,
    too_deep: &'de Cell<bool>,
}

impl<'de> Flavor<'de> for StackLimit<'de> {
    type Remainder = &'de [u8];
    type Source = &'de [u8];

    #[inline]
    fn pop(&mut self) -> postcard::Result<u8> {
        if stack_position().abs_diff(self.base) > MAX_DECODE_STACK {
            self.too_deep.set(true);
            return Err(postcard::Error::DeserializeBadEncoding);
        }
        self.slice.pop()
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        self.slice.size_hint()
    }

    #[inline]
    fn try_take_n(&mut self, ct: usize) -> postcard::Result<&'de [u8]> {
        self.slice.try_take_n(ct)
    }

    fn finalize(self) -> postcard::Result<&'de [u8]> {
        self.slice.finalize()
    }
}

/// An address in the current stack frame. The distance between two is the stack the calls
/// between them use.
#[inline]
fn stack_position() -> usize {
    let marker = 0u8;
    std::ptr::addr_of!(marker) as usize
}

/// Evaluate a parse-time expression again. `path self` reads the file being parsed, so the one
/// holding `expr` is pushed while it runs.
fn evaluate_again(
    working_set: &mut StateWorkingSet,
    expr: &Expression,
) -> Result<Value, ShellError> {
    let file = working_set
        .find_file_by_span(expr.span)
        .map(|file| PathBuf::from(&*file.name));
    let pushed = file.is_some_and(|file| working_set.files.push(file, expr.span).is_ok());
    let value = eval_constant(working_set, expr);
    if pushed {
        working_set.files.pop();
    }
    value
}

/// Put the parameter defaults computed at load into `signature`.
fn set_defaults(signature: &mut Signature, defaults: &HashMap<VarId, Value>) {
    let default_of = |var_id: Option<VarId>| var_id.and_then(|var_id| defaults.get(&var_id));
    for positional in signature
        .required_positional
        .iter_mut()
        .chain(&mut signature.optional_positional)
        .chain(&mut signature.rest_positional)
    {
        if let Some(value) = default_of(positional.var_id) {
            positional.default_value = Some(value.clone());
        }
    }
    for flag in &mut signature.named {
        if let Some(value) = default_of(flag.var_id) {
            flag.default_value = Some(value.clone());
        }
    }
}

/// Find the declaration an imported one corresponds to in `working_set`: the one that all of its
/// names call, with the same shape.
fn resolve_decl(working_set: &StateWorkingSet, import: &DeclImport) -> Result<DeclId, ShellError> {
    let matches = |decl_id: DeclId| {
        DeclShape::of(working_set.get_decl(decl_id)).is_ok_and(|shape| shape == import.shape)
    };
    let found: Vec<_> = import
        .names
        .iter()
        .map(|name| working_set.find_decl(name.as_bytes()))
        .collect();
    if let Some(&Some(decl_id)) = found.first()
        && found.iter().all(|other| *other == Some(decl_id))
        && matches(decl_id)
    {
        return Ok(decl_id);
    }

    // A builtin hidden by a definition of the same name isn't found by name. Only builtins have
    // unique names, so accept one if it is the only one with that name and shape.
    let name = import.names.first().map_or("", String::as_str);
    let builtin = matches!(
        import.shape.command_type,
        CommandType::Builtin | CommandType::Keyword | CommandType::Plugin
    );
    let named = || {
        (0..working_set.num_decls())
            .map(DeclId::new)
            .filter(|decl_id| builtin && working_set.get_decl(*decl_id).name() == name)
    };
    let mut candidates = named().filter(|decl_id| matches(*decl_id));
    let reason = match (candidates.next(), candidates.next()) {
        (Some(decl_id), None) => return Ok(decl_id),
        (Some(_), Some(_)) => "is ambiguous",
        (None, _) if named().next().is_some() || found.iter().any(Option::is_some) => {
            "has a different signature now"
        }
        (None, _) => "is not defined",
    };
    let help = match builtin {
        true => "pickle the source again with this nushell and its plugins",
        false => DEFINED_BY_SHELL_HELP,
    };
    Err(LabeledError::new(format!(
        "Can't load pickled program: the {} command `{}` {reason}",
        import.shape.command_type,
        import.names.join("`, `")
    ))
    .with_help(help)
    .into())
}

/// A copy of `block` to store in a pickle, without its AST unless `keep_ast`.
///
/// Evaluation only runs IR. The AST of a compiled block is read by introspection like `explain`,
/// and blocks without IR (subexpressions and keyword bodies) are inlined into their parent's IR
/// and never run on their own. The AST evaluator could only reach them through `IrAstRef`
/// arguments, which no command requests (`requires_ast_for_arguments_inventory` in nu-command's
/// tests keeps it that way), and when loading computes parse-time values again.
fn stored_block(block: &Block, keep_ast: bool) -> Block {
    let mut block = block.clone();
    if !keep_ast {
        block.pipelines = vec![];
    }
    block
}

/// Call `f` with every id stored as a plain integer in the parser info of `block`'s IR, and
/// replace it with the result.
fn for_each_parser_info_id(
    block: &mut Block,
    mut f: impl FnMut(IdKind, usize) -> Result<usize, ShellError>,
) -> Result<(), ShellError> {
    let Some(ir_block) = &mut block.ir_block else {
        return Ok(());
    };
    for instruction in &mut ir_block.instructions {
        if let Instruction::PushParserInfo { name, info } = instruction
            && let Expr::Int(id) = &mut info.expr
            && let Some((_, kind)) = PARSER_INFO_IDS
                .iter()
                .find(|(key, _)| key.as_bytes() == &ir_block.data[*name])
        {
            let old = usize::try_from(*id)
                .map_err(|_| load_error(format!("invalid {kind:?} id {id} in parser info")))?;
            let new = f(*kind, old)?;
            *id = i64::try_from(new)
                .map_err(|_| load_error(format!("{kind:?} id {new} is out of range")))?;
        }
    }
    Ok(())
}

/// Replace the file paths in the `block_id_name` parser info of `block`'s `source`,
/// `source-env` and `run` calls with what `f` returns for them, if anything. Those commands
/// run the file the parser compiled and only use its path for `FILE_PWD` and `CURRENT_FILE`, so
/// it is stored like the names of the program's files.
fn map_file_names(block: &mut Block, f: impl Fn(&str) -> Option<String>) {
    let Some(ir_block) = &mut block.ir_block else {
        return;
    };
    for instruction in &mut ir_block.instructions {
        if let Instruction::PushParserInfo { name, info } = instruction
            && &ir_block.data[*name] == b"block_id_name"
            && let Expr::Filepath(path, _) = &mut info.expr
            && let Some(new) = f(path)
        {
            *path = new;
        }
    }
}

/// `path` relative to `root`, with `/` separators so it can be resolved on any platform. `None`
/// unless `path` is absolute and has such a path, which Windows paths on another drive don't.
fn relative_path(path: &Path, root: &Path) -> Option<String> {
    if !path.is_absolute() {
        return None;
    }
    let relative = pathdiff::diff_paths(path, root)?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy()),
            Component::ParentDir => parts.push("..".into()),
            Component::CurDir => (),
            Component::Prefix(_) | Component::RootDir => return None,
        }
    }
    Some(parts.join("/"))
}

/// Resolve a path made by [`relative_path`] against `base`.
fn resolve_path(path: &str, base: &Path) -> PathBuf {
    base.join(path.split('/').collect::<PathBuf>())
}

/// The name (with `$`) `engine_state` makes `var_id` visible under, if any.
fn variable_name(engine_state: &EngineState, var_id: VarId) -> Option<String> {
    let visible = |name: &[u8]| {
        engine_state
            .active_overlays(&[])
            .rev()
            .find_map(|overlay| overlay.vars.get(name))
            == Some(&var_id)
    };
    engine_state
        .active_overlays(&[])
        .flat_map(|overlay| overlay.vars.iter())
        .find(|(name, other)| **other == var_id && visible(name))
        .map(|(name, _)| String::from_utf8_lossy(name).into_owned())
}

/// The names `engine_state` makes `decl_id` visible under, sorted. A builtin that a definition
/// of the same name hides keeps its own name, which no other builtin has. Other commands have to
/// be visible, as they are found by name only.
fn decl_names(engine_state: &EngineState, decl_id: DeclId) -> Result<Vec<String>, ShellError> {
    let mut names: Vec<_> = engine_state
        .active_overlays(&[])
        .flat_map(|overlay| overlay.decls.iter())
        .filter(|(name, other)| {
            **other == decl_id && engine_state.find_decl(name, &[]) == Some(decl_id)
        })
        .map(|(name, _)| String::from_utf8_lossy(name).into_owned())
        .collect();
    names.sort();
    names.dedup();
    let decl = engine_state.get_decl(decl_id);
    match decl.command_type() {
        _ if !names.is_empty() => Ok(names),
        CommandType::Builtin | CommandType::Keyword | CommandType::Plugin => {
            Ok(vec![decl.name().into()])
        }
        command_type => Err(save_error(format!(
            "the program uses the {command_type} command `{}`, which has no name in this shell",
            decl.name()
        ))),
    }
}

/// FNV-1a over little-endian words, so a hash stays the same across builds and platforms, and
/// hashing a whole pickle takes a fraction of loading it.
fn content_hash(content: &[u8]) -> u64 {
    let step = |hash: u64, value: u64| (hash ^ value).wrapping_mul(0x0100_0000_01b3);
    let (words, rest) = content.as_chunks::<8>();
    let hash = words.iter().fold(0xcbf2_9ce4_8422_2325, |hash, word| {
        step(hash, u64::from_le_bytes(*word))
    });
    rest.iter()
        .fold(hash, |hash, byte| step(hash, u64::from(*byte)))
}

fn save_error(msg: impl Into<String>) -> ShellError {
    LabeledError::new(format!("Can't pickle program: {}", msg.into())).into()
}

/// Help for a missing definition that the program took from the shell it was pickled in, like a
/// command or constant from that shell's configuration.
const DEFINED_BY_SHELL_HELP: &str = "the shell that made the pickle defined it, for example in its \
configuration. Define it in the script, or run the pickle where it's defined";

fn load_error(msg: impl Into<String>) -> ShellError {
    LabeledError::new(format!("Can't load pickled program: {}", msg.into())).into()
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>, ShellError> {
    postcard::to_stdvec(value).map_err(|err| save_error(err.to_string()))
}

fn decode_error(err: postcard::Error) -> ShellError {
    load_error(format!("the pickle is damaged or incompatible: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_nesting_is_an_error_not_a_stack_overflow() {
        // `list<list<...<any>>>` takes one byte per level.
        let [list, any] =
            postcard::to_stdvec(&Type::List(Box::new(Type::Any))).expect("encodes")[..]
        else {
            panic!("a nested type takes a byte per level")
        };
        let nested = |depth: usize| [vec![list; depth], vec![any]].concat();

        assert!(decode::<Type>(&nested(100)).is_ok());
        let err = decode::<Type>(&nested(1_000_000)).expect_err("too deep");
        assert!(err.to_string().contains("nested too deeply"), "{err}");
    }
}
