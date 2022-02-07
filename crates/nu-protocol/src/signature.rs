<<<<<<< HEAD
use crate::syntax_shape::SyntaxShape;
use crate::type_shape::Type;
use indexmap::IndexMap;
use nu_source::{DbgDocBldr, DebugDocBuilder, PrettyDebug, PrettyDebugWithSource};
use serde::{Deserialize, Serialize};

/// The types of named parameter that a command can have
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum NamedType {
    /// A flag without any associated argument. eg) `foo --bar, foo -b`
    Switch(Option<char>),
    /// A mandatory flag, with associated argument. eg) `foo --required xyz, foo -r xyz`
    Mandatory(Option<char>, SyntaxShape),
    /// An optional flag, with associated argument. eg) `foo --optional abc, foo -o abc`
    Optional(Option<char>, SyntaxShape),
}

impl NamedType {
    pub fn get_short(&self) -> Option<char> {
        match self {
            NamedType::Switch(s) => *s,
            NamedType::Mandatory(s, _) => *s,
            NamedType::Optional(s, _) => *s,
        }
    }

    pub fn get_type_description(&self) -> (String, String, String) {
        let empty_string = ("".to_string(), "".to_string(), "".to_string());
        match self {
            NamedType::Switch(f) => match f {
                Some(flag) => ("switch_flag".to_string(), flag.to_string(), "".to_string()),
                None => empty_string,
            },
            NamedType::Mandatory(f, shape) => match f {
                Some(flag) => (
                    "mandatory_flag".to_string(),
                    flag.to_string(),
                    shape.syntax_shape_name().to_string(),
                ),
                None => empty_string,
            },
            NamedType::Optional(f, shape) => match f {
                Some(flag) => (
                    "optional_flag".to_string(),
                    flag.to_string(),
                    shape.syntax_shape_name().to_string(),
                ),
                None => empty_string,
            },
        }
    }
}

/// The type of positional arguments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PositionalType {
    /// A mandatory positional argument with the expected shape of the value
    Mandatory(String, SyntaxShape),
    /// An optional positional argument with the expected shape of the value
    Optional(String, SyntaxShape),
}

impl PrettyDebug for PositionalType {
    /// Prepare the PositionalType for pretty-printing
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            PositionalType::Mandatory(string, shape) => {
                DbgDocBldr::description(string)
                    + DbgDocBldr::delimit("(", shape.pretty(), ")")
                        .into_kind()
                        .group()
            }
            PositionalType::Optional(string, shape) => {
                DbgDocBldr::description(string)
                    + DbgDocBldr::operator("?")
                    + DbgDocBldr::delimit("(", shape.pretty(), ")")
                        .into_kind()
                        .group()
            }
        }
    }
}

impl PositionalType {
    /// Helper to create a mandatory positional argument type
    pub fn mandatory(name: &str, ty: SyntaxShape) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), ty)
    }

    /// Helper to create a mandatory positional argument with an "any" type
    pub fn mandatory_any(name: &str) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), SyntaxShape::Any)
    }

    /// Helper to create a mandatory positional argument with a block type
    pub fn mandatory_block(name: &str) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), SyntaxShape::Block)
    }

    /// Helper to create a optional positional argument type
    pub fn optional(name: &str, ty: SyntaxShape) -> PositionalType {
        PositionalType::Optional(name.to_string(), ty)
    }

    /// Helper to create a optional positional argument with an "any" type
    pub fn optional_any(name: &str) -> PositionalType {
        PositionalType::Optional(name.to_string(), SyntaxShape::Any)
    }

    /// Gets the name of the positional argument
    pub fn name(&self) -> &str {
        match self {
            PositionalType::Mandatory(s, _) => s,
            PositionalType::Optional(s, _) => s,
        }
    }

    /// Gets the expected type of a positional argument
    pub fn syntax_type(&self) -> SyntaxShape {
        match *self {
            PositionalType::Mandatory(_, t) => t,
            PositionalType::Optional(_, t) => t,
        }
    }

    pub fn get_type_description(&self) -> (String, String) {
        match &self {
            PositionalType::Mandatory(c, s) => (c.to_string(), s.syntax_shape_name().to_string()),
            PositionalType::Optional(c, s) => (c.to_string(), s.syntax_shape_name().to_string()),
        }
    }
}

type Description = String;

/// The full signature of a command. All commands have a signature similar to a function signature.
/// Commands will use this information to register themselves with Nu's core engine so that the command
/// can be invoked, help can be displayed, and calls to the command can be error-checked.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Signature {
    /// The name of the command. Used when calling the command
    pub name: String,
    /// Usage instructions about the command
    pub usage: String,
    /// Longer or more verbose usage statement
    pub extra_usage: String,
    /// The list of positional arguments, both required and optional, and their corresponding types and help text
    pub positional: Vec<(PositionalType, Description)>,
    /// After the positional arguments, a catch-all for the rest of the arguments that might follow, their type, and help text
    pub rest_positional: Option<(String, SyntaxShape, Description)>,
    /// The named flags with corresponding type and help text
    pub named: IndexMap<String, (NamedType, Description)>,
    /// The type of values being sent out from the command into the pipeline, if any
    pub yields: Option<Type>,
    /// The type of values being read in from the pipeline into the command, if any
    pub input: Option<Type>,
    /// If the command is expected to filter data, or to consume it (as a sink)
    pub is_filter: bool,
=======
use serde::Deserialize;
use serde::Serialize;

use crate::ast::Call;
use crate::engine::Command;
use crate::engine::EngineState;
use crate::engine::Stack;
use crate::BlockId;
use crate::PipelineData;
use crate::SyntaxShape;
use crate::VarId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flag {
    pub long: String,
    pub short: Option<char>,
    pub arg: Option<SyntaxShape>,
    pub required: bool,
    pub desc: String,
    // For custom commands
    pub var_id: Option<VarId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionalArg {
    pub name: String,
    pub desc: String,
    pub shape: SyntaxShape,
    // For custom commands
    pub var_id: Option<VarId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Category {
    Default,
    Conversions,
    Core,
    Date,
    Env,
    Experimental,
    FileSystem,
    Filters,
    Formats,
    Math,
    Network,
    Random,
    Platform,
    Shells,
    Strings,
    System,
    Viewers,
    Hash,
    Generators,
    Custom(String),
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
            Category::Network => "network",
            Category::Random => "random",
            Category::Platform => "platform",
            Category::Shells => "shells",
            Category::Strings => "strings",
            Category::System => "system",
            Category::Viewers => "viewers",
            Category::Hash => "hash",
            Category::Generators => "generators",
            Category::Custom(name) => name,
        };

        write!(f, "{}", msg)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signature {
    pub name: String,
    pub usage: String,
    pub extra_usage: String,
    pub required_positional: Vec<PositionalArg>,
    pub optional_positional: Vec<PositionalArg>,
    pub rest_positional: Option<PositionalArg>,
    pub named: Vec<Flag>,
    pub is_filter: bool,
    pub creates_scope: bool,
    // Signature category used to classify commands stored in the list of declarations
    pub category: Category,
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.usage == other.usage
<<<<<<< HEAD
            && self.positional == other.positional
=======
            && self.required_positional == other.required_positional
            && self.optional_positional == other.optional_positional
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            && self.rest_positional == other.rest_positional
            && self.is_filter == other.is_filter
    }
}

impl Eq for Signature {}

impl Signature {
<<<<<<< HEAD
    pub fn shift_positional(&mut self) {
        self.positional = Vec::from(&self.positional[1..]);
    }

    pub fn remove_named(&mut self, name: &str) {
        self.named.remove(name);
    }

    pub fn allowed(&self) -> Vec<String> {
        let mut allowed = indexmap::IndexSet::new();

        for (name, (t, _)) in &self.named {
            if let Some(c) = t.get_short() {
                allowed.insert(format!("-{}", c));
            }
            allowed.insert(format!("--{}", name));
        }

        for (ty, _) in &self.positional {
            let shape = ty.syntax_type();
            allowed.insert(shape.display());
        }

        if let Some((_, shape, _)) = &self.rest_positional {
            allowed.insert(shape.display());
        }

        allowed.into_iter().collect()
    }
}

impl PrettyDebugWithSource for Signature {
    /// Prepare a Signature for pretty-printing
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        DbgDocBldr::typed(
            "signature",
            DbgDocBldr::description(&self.name)
                + DbgDocBldr::preceded(
                    DbgDocBldr::space(),
                    DbgDocBldr::intersperse(
                        self.positional
                            .iter()
                            .map(|(ty, _)| ty.pretty_debug(source)),
                        DbgDocBldr::space(),
                    ),
                ),
        )
    }
}

impl Signature {
    /// Create a new command signature with the given name
    pub fn new(name: impl Into<String>) -> Signature {
=======
    pub fn new(name: impl Into<String>) -> Signature {
        // default help flag
        let flag = Flag {
            long: "help".into(),
            short: Some('h'),
            arg: None,
            desc: "Display this help message".into(),
            required: false,
            var_id: None,
        };

>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        Signature {
            name: name.into(),
            usage: String::new(),
            extra_usage: String::new(),
<<<<<<< HEAD
            positional: vec![],
            rest_positional: None,
            named: indexmap::indexmap! {"help".into() => (NamedType::Switch(Some('h')), "Display this help message".into())},
            is_filter: false,
            yields: None,
            input: None,
        }
    }

    /// Create a new signature
=======
            required_positional: vec![],
            optional_positional: vec![],
            rest_positional: None,
            named: vec![flag],
            is_filter: false,
            creates_scope: false,
            category: Category::Default,
        }
    }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    pub fn build(name: impl Into<String>) -> Signature {
        Signature::new(name.into())
    }

    /// Add a description to the signature
    pub fn desc(mut self, usage: impl Into<String>) -> Signature {
        self.usage = usage.into();
        self
    }

    /// Add a required positional argument to the signature
    pub fn required(
        mut self,
        name: impl Into<String>,
<<<<<<< HEAD
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.positional.push((
            PositionalType::Mandatory(name.into(), ty.into()),
            desc.into(),
        ));
=======
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.required_positional.push(PositionalArg {
            name: name.into(),
            desc: desc.into(),
            shape: shape.into(),
            var_id: None,
        });
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce

        self
    }

<<<<<<< HEAD
    /// Add an optional positional argument to the signature
    pub fn optional(
        mut self,
        name: impl Into<String>,
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.positional.push((
            PositionalType::Optional(name.into(), ty.into()),
            desc.into(),
        ));
=======
    /// Add a required positional argument to the signature
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
        });
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce

        self
    }

    /// Add an optional named flag argument to the signature
    pub fn named(
        mut self,
        name: impl Into<String>,
<<<<<<< HEAD
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> Signature {
        let s = short.map(|c| {
            debug_assert!(!self.get_shorts().contains(&c));
            c
        });
        self.named.insert(
            name.into(),
            (NamedType::Optional(s, ty.into()), desc.into()),
        );
=======
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
        });
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce

        self
    }

    /// Add a required named flag argument to the signature
    pub fn required_named(
        mut self,
        name: impl Into<String>,
<<<<<<< HEAD
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> Signature {
        let s = short.map(|c| {
            debug_assert!(!self.get_shorts().contains(&c));
            c
        });

        self.named.insert(
            name.into(),
            (NamedType::Mandatory(s, ty.into()), desc.into()),
        );

=======
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
        });

>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        self
    }

    /// Add a switch to the signature
    pub fn switch(
        mut self,
        name: impl Into<String>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> Signature {
<<<<<<< HEAD
=======
        let (name, s) = self.check_names(name, short);

        self.named.push(Flag {
            long: name,
            short: s,
            arg: None,
            required: false,
            desc: desc.into(),
            var_id: None,
        });

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
            one_liner.push_str(&format!("...{}", get_positional_short_name(rest, false)));
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
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        let s = short.map(|c| {
            debug_assert!(
                !self.get_shorts().contains(&c),
                "There may be duplicate short flags, such as -h"
            );
            c
        });

<<<<<<< HEAD
        self.named
            .insert(name.into(), (NamedType::Switch(s), desc.into()));
        self
=======
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
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    }

    /// Set the filter flag for the signature
    pub fn filter(mut self) -> Signature {
        self.is_filter = true;
        self
    }

<<<<<<< HEAD
    /// Set the type for the "rest" of the positional arguments
    /// Note: Not naming the field in your struct holding the rest values "rest", can
    /// cause errors when deserializing
    pub fn rest(
        mut self,
        name: impl Into<String>,
        ty: SyntaxShape,
        desc: impl Into<String>,
    ) -> Signature {
        self.rest_positional = Some((name.into(), ty, desc.into()));
        self
    }

    /// Add a type for the output of the command to the signature
    pub fn yields(mut self, ty: Type) -> Signature {
        self.yields = Some(ty);
        self
    }

    /// Add a type for the input of the command to the signature
    pub fn input(mut self, ty: Type) -> Signature {
        self.input = Some(ty);
        self
    }

    /// Get list of the short-hand flags
    pub fn get_shorts(&self) -> Vec<char> {
        let mut shorts = Vec::new();
        for (_, (t, _)) in &self.named {
            if let Some(c) = t.get_short() {
                shorts.push(c);
            }
        }
        shorts
=======
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
        panic!("Internal error: can't run custom command with 'run', use block_id");
    }

    fn get_block_id(&self) -> Option<BlockId> {
        Some(self.block_id)
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    }
}
