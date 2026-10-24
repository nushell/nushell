//! Compile-time witnesses for the `nu-protocol` types in the plugin protocol's frozen tier.
//!
//! Plugins built against one Nushell version keep running on later versions as long as the
//! plugin protocol version (`PLUGIN_PROTOCOL_VERSION` in `nu-plugin-protocol`) stays
//! compatible. The types below cross the plugin wire with derived serde, so changing one of
//! them changes the wire. Each witness names every variant and field of one type, which makes
//! the build stop here when one is added, removed, or renamed.
//!
//! When a witness stops compiling, pick the protocol bump from the table in the
//! `PLUGIN_PROTOCOL_VERSION` docs. Then update the witness, add the new variant or field to the
//! corpus in `nu-plugin-protocol`'s `src/tests.rs`, and regenerate its `protocol_snapshots`.
//! Every field is matched as `_`, so changing a field's type or a serde attribute still
//! compiles. Only the snapshots catch that, and only for the values in their corpus.
//!
//! The snapshots also stand in for the types that have no witness here: the ones with
//! hand-written serde (`Record`, `Range`, `CellPath`, `PathMember`, the `Id` types, and
//! `ShellError`, which travels as [`LabeledError`]), the ones with private fields (`Filesize`,
//! `OneOf`, `CollectionColumns`), and `nu_utils::NuCow` inside `Completion::List`. `Config` and
//! the IR/AST types are not frozen; see the `PLUGIN_PROTOCOL_VERSION` docs.
//!
//! Nothing here runs. Each witness is a non-capturing closure coerced to a function pointer, so
//! it is type-checked and then discarded.

use crate::{
    BuiltinCompletion, ByteStreamType, Category, CommandWideCompleter, Completion, DataSource,
    DynamicSuggestion, ErrorLabel, Flag, LabeledError, PipelineMetadata, PluginExample,
    PluginMetadata, PluginSignature, PositionalArg, SignalAction, Signature, Span, Spanned,
    SuggestionKind, SyntaxShape, Type, Value,
    ast::{Assignment, Bits, Boolean, Comparison, Math, Operator},
    casing::Casing,
    engine::{Closure, CommandType},
};

const _: fn(&Value) = |value| match value {
    Value::Bool {
        val: _,
        internal_span: _,
    }
    | Value::Int {
        val: _,
        internal_span: _,
    }
    | Value::Float {
        val: _,
        internal_span: _,
    }
    | Value::String {
        val: _,
        internal_span: _,
    }
    | Value::Glob {
        val: _,
        no_expand: _,
        internal_span: _,
    }
    | Value::Filesize {
        val: _,
        internal_span: _,
    }
    | Value::Duration {
        val: _,
        internal_span: _,
    }
    | Value::Date {
        val: _,
        internal_span: _,
    }
    | Value::Range {
        val: _,
        signals: _,
        internal_span: _,
    }
    | Value::Record {
        val: _,
        internal_span: _,
    }
    | Value::List {
        vals: _,
        signals: _,
        internal_span: _,
    }
    | Value::Closure {
        val: _,
        internal_span: _,
    }
    | Value::Error {
        error: _,
        internal_span: _,
    }
    | Value::Binary {
        val: _,
        internal_span: _,
    }
    | Value::CellPath {
        val: _,
        internal_span: _,
    }
    | Value::Custom {
        val: _,
        internal_span: _,
    }
    | Value::Nothing { internal_span: _ } => {}
};

const _: fn(&Span) = |span| {
    let Span { start: _, end: _ } = span;
};

const _: fn(&Spanned<()>) = |spanned| {
    let Spanned { item: _, span: _ } = spanned;
};

const _: fn(&Closure) = |closure| {
    let Closure {
        block_id: _,
        captures: _,
    } = closure;
};

const _: fn(&LabeledError) = |error| {
    let LabeledError {
        msg: _,
        labels: _,
        code: _,
        url: _,
        help: _,
        inner: _,
    } = error;
};

const _: fn(&ErrorLabel) = |label| {
    let ErrorLabel { text: _, span: _ } = label;
};

const _: fn(&PipelineMetadata) = |metadata| {
    let PipelineMetadata {
        data_source: _,
        path_columns: _,
        content_type: _,
        custom: _,
    } = metadata;
};

const _: fn(&DataSource) = |source| match source {
    DataSource::HtmlThemes | DataSource::FilePath(_) | DataSource::None => {}
};

const _: fn(&ByteStreamType) = |ty| match ty {
    ByteStreamType::Binary | ByteStreamType::String | ByteStreamType::Unknown => {}
};

const _: fn(&SignalAction) = |action| match action {
    SignalAction::Interrupt | SignalAction::Reset => {}
};

const _: fn(&Operator) = |operator| match operator {
    Operator::Comparison(_)
    | Operator::Math(_)
    | Operator::Boolean(_)
    | Operator::Bits(_)
    | Operator::Assignment(_) => {}
};

const _: fn(&Comparison) = |comparison| match comparison {
    Comparison::Equal
    | Comparison::NotEqual
    | Comparison::LessThan
    | Comparison::GreaterThan
    | Comparison::LessThanOrEqual
    | Comparison::GreaterThanOrEqual
    | Comparison::RegexMatch
    | Comparison::NotRegexMatch
    | Comparison::In
    | Comparison::NotIn
    | Comparison::Has
    | Comparison::NotHas
    | Comparison::StartsWith
    | Comparison::NotStartsWith
    | Comparison::EndsWith
    | Comparison::NotEndsWith => {}
};

const _: fn(&Math) = |math| match math {
    Math::Add
    | Math::Subtract
    | Math::Multiply
    | Math::Divide
    | Math::FloorDivide
    | Math::Modulo
    | Math::Pow
    | Math::Concatenate => {}
};

const _: fn(&Boolean) = |boolean| match boolean {
    Boolean::Or | Boolean::Xor | Boolean::And => {}
};

const _: fn(&Bits) = |bits| match bits {
    Bits::BitOr | Bits::BitXor | Bits::BitAnd | Bits::ShiftLeft | Bits::ShiftRight => {}
};

const _: fn(&Assignment) = |assignment| match assignment {
    Assignment::Assign
    | Assignment::AddAssign
    | Assignment::SubtractAssign
    | Assignment::MultiplyAssign
    | Assignment::DivideAssign
    | Assignment::ConcatenateAssign => {}
};

const _: fn(&Casing) = |casing| match casing {
    Casing::Sensitive | Casing::Insensitive => {}
};

const _: fn(&PluginMetadata) = |metadata| {
    let PluginMetadata {
        version: _,
        protocol_version: _,
        nushell_version: _,
    } = metadata;
};

const _: fn(&PluginSignature) = |signature| {
    let PluginSignature {
        sig: _,
        examples: _,
    } = signature;
};

const _: fn(&PluginExample) = |example| {
    let PluginExample {
        example: _,
        description: _,
        result: _,
    } = example;
};

const _: fn(&Signature) = |signature| {
    let Signature {
        name: _,
        description: _,
        extra_description: _,
        search_terms: _,
        required_positional: _,
        optional_positional: _,
        rest_positional: _,
        named: _,
        input_output_types: _,
        allow_variants_without_examples: _,
        is_filter: _,
        creates_scope: _,
        allows_unknown_args: _,
        complete: _,
        category: _,
    } = signature;
};

const _: fn(&Flag) = |flag| {
    let Flag {
        long: _,
        short: _,
        arg: _,
        required: _,
        desc: _,
        completion: _,
        var_id: _,
        default_value: _,
    } = flag;
};

const _: fn(&PositionalArg) = |arg| {
    let PositionalArg {
        name: _,
        desc: _,
        shape: _,
        completion: _,
        var_id: _,
        default_value: _,
    } = arg;
};

const _: fn(&Category) = |category| match category {
    Category::Bits
    | Category::Bytes
    | Category::Chart
    | Category::Conversions
    | Category::Core
    | Category::Custom(_)
    | Category::Database
    | Category::Date
    | Category::Debug
    | Category::Default
    | Category::Deprecated
    | Category::Removed
    | Category::Env
    | Category::Experimental
    | Category::FileSystem
    | Category::Filters
    | Category::Formats
    | Category::Generators
    | Category::Hash
    | Category::History
    | Category::Math
    | Category::Misc
    | Category::Network
    | Category::Path
    | Category::Platform
    | Category::Plugin
    | Category::Random
    | Category::Shells
    | Category::Strings
    | Category::System
    | Category::Viewers => {}
};

const _: fn(&Completion) = |completion| match completion {
    Completion::Command(_) | Completion::List(_) | Completion::Builtin(_) => {}
};

const _: fn(&BuiltinCompletion) = |completion| match completion {
    BuiltinCompletion::NuFile {
        std_virtual_path: _,
    }
    | BuiltinCompletion::ModuleExports
    | BuiltinCompletion::EnvVar
    | BuiltinCompletion::Command { internal_only: _ } => {}
};

const _: fn(&CommandWideCompleter) = |completer| match completer {
    CommandWideCompleter::External | CommandWideCompleter::Command(_) => {}
};

const _: fn(&SyntaxShape) = |shape| match shape {
    SyntaxShape::Any
    | SyntaxShape::Binary
    | SyntaxShape::Block
    | SyntaxShape::Boolean
    | SyntaxShape::CellPath
    | SyntaxShape::Closure(_)
    | SyntaxShape::DateTime
    | SyntaxShape::Directory
    | SyntaxShape::Duration
    | SyntaxShape::Error
    | SyntaxShape::Expression
    | SyntaxShape::ExternalArgument
    | SyntaxShape::Filepath
    | SyntaxShape::Filesize
    | SyntaxShape::Float
    | SyntaxShape::FullCellPath
    | SyntaxShape::GlobPattern
    | SyntaxShape::Int
    | SyntaxShape::ImportPattern
    | SyntaxShape::Keyword(_, _)
    | SyntaxShape::List(_)
    | SyntaxShape::MathExpression
    | SyntaxShape::MatchBlock
    | SyntaxShape::Nothing
    | SyntaxShape::Number
    | SyntaxShape::OneOf(_)
    | SyntaxShape::Operator
    | SyntaxShape::Range
    | SyntaxShape::Record(_)
    | SyntaxShape::RowCondition
    | SyntaxShape::Signature
    | SyntaxShape::ExternalSignature
    | SyntaxShape::String
    | SyntaxShape::Table(_)
    | SyntaxShape::VarWithOptType => {}
};

const _: fn(&Type) = |ty| match ty {
    Type::Any
    | Type::Binary
    | Type::Block
    | Type::Bool
    | Type::CellPath
    | Type::Closure
    | Type::Custom(_)
    | Type::Date
    | Type::Duration
    | Type::Error
    | Type::Filesize
    | Type::Float
    | Type::Int
    | Type::List(_)
    | Type::Nothing
    | Type::Number
    | Type::OneOf(_)
    | Type::Range
    | Type::Record(_)
    | Type::String
    | Type::Glob
    | Type::Table(_) => {}
};

const _: fn(&DynamicSuggestion) = |suggestion| {
    let DynamicSuggestion {
        value: _,
        display_override: _,
        description: _,
        extra: _,
        append_whitespace: _,
        match_indices: _,
        span: _,
        kind: _,
    } = suggestion;
};

const _: fn(&SuggestionKind) = |kind| match kind {
    SuggestionKind::Command(_, _)
    | SuggestionKind::Value(_)
    | SuggestionKind::CellPath
    | SuggestionKind::Directory
    | SuggestionKind::File
    | SuggestionKind::Flag
    | SuggestionKind::Module
    | SuggestionKind::Operator
    | SuggestionKind::Variable => {}
};

const _: fn(&CommandType) = |ty| match ty {
    CommandType::Builtin
    | CommandType::Custom
    | CommandType::Keyword
    | CommandType::External
    | CommandType::Alias
    | CommandType::Plugin => {}
};
