use serde::Deserialize;
use serde::Serialize;

use crate::ast::Call;
use crate::ast::Expression;
use crate::engine::Command;
use crate::engine::EngineState;
use crate::engine::Stack;
use crate::BlockId;
use crate::PipelineData;
use crate::ShellError;
use crate::SyntaxShape;
use crate::Type;
use crate::VarId;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flag {
    pub long: String,
    pub short: Option<char>,
    pub arg: Option<SyntaxShape>,
    pub required: bool,
    pub desc: String,

    // For custom commands
    pub var_id: Option<VarId>,
    pub default_value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionalArg {
    pub name: String,
    pub desc: String,
    pub shape: SyntaxShape,

    // For custom commands
    pub var_id: Option<VarId>,
    pub default_value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    Default,
    Conversions,
    Core,
    Bits,
    Bytes,
    Date,
    Env,
    Experimental,
    FileSystem,
    Filters,
    Formats,
    Math,
    Misc,
    Network,
    Random,
    Platform,
    Shells,
    Strings,
    System,
    Viewers,
    Hash,
    Generators,
    Chart,
    Custom(String),
    Deprecated,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Category::Default => "default",
            Category::Conversions => "conversions",
            Category::Core => "core",
            Category::Date => "date",
            Category::Env => "env",
            Category::Experimental => "experimental",
            Category::FileSystem => "filesystem",
            Category::Filters => "filters",
            Category::Formats => "formats",
            Category::Math => "math",
            Category::Misc => "misc",
            Category::Network => "network",
            Category::Random => "random",
            Category::Platform => "platform",
            Category::Shells => "shells",
            Category::Strings => "strings",
            Category::System => "system",
            Category::Viewers => "viewers",
            Category::Hash => "hash",
            Category::Generators => "generators",
            Category::Chart => "chart",
            Category::Custom(name) => name,
            Category::Deprecated => "deprecated",
            Category::Bytes => "bytes",
            Category::Bits => "bits",
        };

        write!(f, "{}", msg)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signature {
    pub name: String,
    pub usage: String,
    pub extra_usage: String,
    pub search_terms: Vec<String>,
    pub required_positional: Vec<PositionalArg>,
    pub optional_positional: Vec<PositionalArg>,
    pub rest_positional: Option<PositionalArg>,
    pub vectorizes_over_list: bool,
    pub named: Vec<Flag>,
    pub input_type: Type,
    pub output_type: Type,
    pub input_output_types: Vec<(Type, Type)>,
    pub allow_variants_without_examples: bool,
    pub is_filter: bool,
    pub creates_scope: bool,
    // Signature category used to classify commands stored in the list of declarations
    pub category: Category,
}

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.usage == other.usage
            && self.required_positional == other.required_positional
            && self.optional_positional == other.optional_positional
            && self.rest_positional == other.rest_positional
            && self.is_filter == other.is_filter
    }
}

impl Eq for Signature {}

impl Signature {
    pub fn new(name: impl Into<String>) -> Signature {
        Signature {
            name: name.into(),
            usage: String::new(),
            extra_usage: String::new(),
            search_terms: vec![],
            required_positional: vec![],
            optional_positional: vec![],
            rest_positional: None,
            vectorizes_over_list: false,
            input_type: Type::Any,
            output_type: Type::Any,
            input_output_types: vec![],
            allow_variants_without_examples: false,
            named: vec![],
            is_filter: false,
            creates_scope: false,
            category: Category::Default,
        }
    }

    // Add a default help option to a signature
    pub fn add_help(mut self) -> Signature {
        // default help flag
        let flag = Flag {
            long: "help".into(),
            short: Some('h'),
            arg: None,
            desc: "Display the help message for this command".into(),
            required: false,
            var_id: None,
            default_value: None,
        };
        self.named.push(flag);
        self
    }

    // Build an internal signature with default help option
    pub fn build(name: impl Into<String>) -> Signature {
        Signature::new(name.into()).add_help()
    }

    /// Add a description to the signature
    pub fn usage(mut self, msg: impl Into<String>) -> Signature {
        self.usage = msg.into();
        self
    }

    /// Add an extra description to the signature
    pub fn extra_usage(mut self, msg: impl Into<String>) -> Signature {
        self.extra_usage = msg.into();
        self
    }

    /// Add search terms to the signature
    pub fn search_terms(mut self, terms: Vec<String>) -> Signature {
        self.search_terms = terms;
        self
    }

    /// Update signature's fields from a Command trait implementation
    pub fn update_from_command(mut self, command: &dyn Command) -> Signature {
        self.search_terms = command
            .search_terms()
            .into_iter()
            .map(|term| term.to_string())
            .collect();
        self.extra_usage = command.extra_usage().to_string();
        self.usage = command.usage().to_string();
        self
    }

    /// Add a required positional argument to the signature
    pub fn required(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.required_positional.push(PositionalArg {
            name: name.into(),
            desc: desc.into(),
            shape: shape.into(),
            var_id: None,
            default_value: None,
        });

        self
    }

    /// Add an optional positional argument to the signature
    pub fn optional(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.optional_positional.push(PositionalArg {
            name: name.into(),
            desc: desc.into(),
            shape: shape.into(),
            var_id: None,
            default_value: None,
        });

        self
    }

    pub fn rest(
        mut self,
        name: &str,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.rest_positional = Some(PositionalArg {
            name: name.into(),
            desc: desc.into(),
            shape: shape.into(),
            var_id: None,
            default_value: None,
        });

        self
    }

    /// Is this command capable of operating on its input via cell paths?
    pub fn operates_on_cell_paths(&self) -> bool {
        self.required_positional
            .iter()
            .chain(self.rest_positional.iter())
            .any(|pos| {
                matches!(
                    pos,
                    PositionalArg {
                        shape: SyntaxShape::CellPath,
                        ..
                    }
                )
            })
    }

    /// Add an optional named flag argument to the signature
    pub fn named(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> Signature {
        let (name, s) = self.check_names(name, short);

        self.named.push(Flag {
            long: name,
            short: s,
            arg: Some(shape.into()),
            required: false,
            desc: desc.into(),
            var_id: None,
            default_value: None,
        });

        self
    }

    /// Add a required named flag argument to the signature
    pub fn required_named(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> Signature {
        let (name, s) = self.check_names(name, short);

        self.named.push(Flag {
            long: name,
            short: s,
            arg: Some(shape.into()),
            required: true,
            desc: desc.into(),
            var_id: None,
            default_value: None,
        });

        self
    }

    /// Add a switch to the signature
    pub fn switch(
        mut self,
        name: impl Into<String>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> Signature {
        let (name, s) = self.check_names(name, short);

        self.named.push(Flag {
            long: name,
            short: s,
            arg: None,
            required: false,
            desc: desc.into(),
            var_id: None,
            default_value: None,
        });

        self
    }

    /// Changes the input type of the command signature
    pub fn input_type(mut self, input_type: Type) -> Signature {
        self.input_type = input_type;
        self
    }

    /// Changes the output type of the command signature
    pub fn output_type(mut self, output_type: Type) -> Signature {
        self.output_type = output_type;
        self
    }

    pub fn vectorizes_over_list(mut self, vectorizes_over_list: bool) -> Signature {
        self.vectorizes_over_list = vectorizes_over_list;
        self
    }

    /// Set the input-output type signature variants of the command
    pub fn input_output_types(mut self, input_output_types: Vec<(Type, Type)>) -> Signature {
        self.input_output_types = input_output_types;
        self
    }

    /// Changes the signature category
    pub fn category(mut self, category: Category) -> Signature {
        self.category = category;

        self
    }

    /// Sets that signature will create a scope as it parses
    pub fn creates_scope(mut self) -> Signature {
        self.creates_scope = true;
        self
    }

    // Is it allowed for the type signature to feature a variant that has no corresponding example?
    pub fn allow_variants_without_examples(mut self, allow: bool) -> Signature {
        self.allow_variants_without_examples = allow;
        self
    }

    pub fn call_signature(&self) -> String {
        let mut one_liner = String::new();
        one_liner.push_str(&self.name);
        one_liner.push(' ');

        // Note: the call signature needs flags first because on the nu commandline,
        // flags will precede the script file name. Flags for internal commands can come
        // either before or after (or around) positional parameters, so there isn't a strong
        // preference, so we default to the more constrained example.
        if self.named.len() > 1 {
            one_liner.push_str("{flags} ");
        }

        for positional in &self.required_positional {
            one_liner.push_str(&get_positional_short_name(positional, true));
        }
        for positional in &self.optional_positional {
            one_liner.push_str(&get_positional_short_name(positional, false));
        }

        if let Some(rest) = &self.rest_positional {
            let _ = write!(one_liner, "...{}", get_positional_short_name(rest, false));
        }

        // if !self.subcommands.is_empty() {
        //     one_liner.push_str("<subcommand> ");
        // }

        one_liner
    }

    /// Get list of the short-hand flags
    pub fn get_shorts(&self) -> Vec<char> {
        self.named.iter().filter_map(|f| f.short).collect()
    }

    /// Get list of the long-hand flags
    pub fn get_names(&self) -> Vec<&str> {
        self.named.iter().map(|f| f.long.as_str()).collect()
    }

    /// Checks if short or long are already present
    /// Panics if one of them is found
    fn check_names(&self, name: impl Into<String>, short: Option<char>) -> (String, Option<char>) {
        let s = short.map(|c| {
            debug_assert!(
                !self.get_shorts().contains(&c),
                "There may be duplicate short flags, such as -h"
            );
            c
        });

        let name = {
            let name: String = name.into();
            debug_assert!(
                !self.get_names().contains(&name.as_str()),
                "There may be duplicate name flags, such as --help"
            );
            name
        };

        (name, s)
    }

    pub fn get_positional(&self, position: usize) -> Option<PositionalArg> {
        if position < self.required_positional.len() {
            self.required_positional.get(position).cloned()
        } else if position < (self.required_positional.len() + self.optional_positional.len()) {
            self.optional_positional
                .get(position - self.required_positional.len())
                .cloned()
        } else {
            self.rest_positional.clone()
        }
    }

    pub fn num_positionals(&self) -> usize {
        let mut total = self.required_positional.len() + self.optional_positional.len();

        for positional in &self.required_positional {
            if let SyntaxShape::Keyword(..) = positional.shape {
                // Keywords have a required argument, so account for that
                total += 1;
            }
        }
        for positional in &self.optional_positional {
            if let SyntaxShape::Keyword(..) = positional.shape {
                // Keywords have a required argument, so account for that
                total += 1;
            }
        }
        total
    }

    pub fn num_positionals_after(&self, idx: usize) -> usize {
        let mut total = 0;

        for (curr, positional) in self.required_positional.iter().enumerate() {
            match positional.shape {
                SyntaxShape::Keyword(..) => {
                    // Keywords have a required argument, so account for that
                    if curr > idx {
                        total += 2;
                    }
                }
                _ => {
                    if curr > idx {
                        total += 1;
                    }
                }
            }
        }
        total
    }

    /// Find the matching long flag
    pub fn get_long_flag(&self, name: &str) -> Option<Flag> {
        for flag in &self.named {
            if flag.long == name {
                return Some(flag.clone());
            }
        }
        None
    }

    /// Find the matching long flag
    pub fn get_short_flag(&self, short: char) -> Option<Flag> {
        for flag in &self.named {
            if let Some(short_flag) = &flag.short {
                if *short_flag == short {
                    return Some(flag.clone());
                }
            }
        }
        None
    }

    /// Set the filter flag for the signature
    pub fn filter(mut self) -> Signature {
        self.is_filter = true;
        self
    }

    /// Create a placeholder implementation of Command as a way to predeclare a definition's
    /// signature so other definitions can see it. This placeholder is later replaced with the
    /// full definition in a second pass of the parser.
    pub fn predeclare(self) -> Box<dyn Command> {
        Box::new(Predeclaration { signature: self })
    }

    /// Combines a signature and a block into a runnable block
    pub fn into_block_command(self, block_id: BlockId) -> Box<dyn Command> {
        Box::new(BlockCommand {
            signature: self,
            block_id,
        })
    }
}

#[derive(Clone)]
struct Predeclaration {
    signature: Signature,
}

impl Command for Predeclaration {
    fn name(&self) -> &str {
        &self.signature.name
    }

    fn signature(&self) -> Signature {
        self.signature.clone()
    }

    fn usage(&self) -> &str {
        &self.signature.usage
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, crate::ShellError> {
        panic!("Internal error: can't run a predeclaration without a body")
    }
}

fn get_positional_short_name(arg: &PositionalArg, is_required: bool) -> String {
    match &arg.shape {
        SyntaxShape::Keyword(name, ..) => {
            if is_required {
                format!("{} <{}> ", String::from_utf8_lossy(name), arg.name)
            } else {
                format!("({} <{}>) ", String::from_utf8_lossy(name), arg.name)
            }
        }
        _ => {
            if is_required {
                format!("<{}> ", arg.name)
            } else {
                format!("({}) ", arg.name)
            }
        }
    }
}

#[derive(Clone)]
struct BlockCommand {
    signature: Signature,
    block_id: BlockId,
}

impl Command for BlockCommand {
    fn name(&self) -> &str {
        &self.signature.name
    }

    fn signature(&self) -> Signature {
        self.signature.clone()
    }

    fn usage(&self) -> &str {
        &self.signature.usage
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<crate::PipelineData, crate::ShellError> {
        Err(ShellError::GenericError(
            "Internal error: can't run custom command with 'run', use block_id".to_string(),
            "".to_string(),
            None,
            None,
            Vec::new(),
        ))
    }

    fn get_block_id(&self) -> Option<BlockId> {
        Some(self.block_id)
    }
}
