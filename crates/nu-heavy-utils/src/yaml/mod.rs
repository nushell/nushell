mod parse;
use nu_protocol::FromValue;
pub use parse::*;

mod serialize;
pub use serialize::*;

#[non_exhaustive]
#[derive(Debug, Clone, Default, FromValue)]
pub enum Spec {
    #[default]
    #[nu_value(rename = "1.1")]
    V1_1,

    #[nu_value(rename = "1.2")]
    V1_2,
}

#[derive(strum::Display, strum::EnumString, Debug, )]
#[strum(parse_err_ty = UnknownTagError, parse_err_fn = UnknownTagError::new)]
enum KnownTag {
    // YAML known tags
    #[strum(to_string = "!!map", serialize = "tag:yaml.org,2002:map")]
    Map,

    #[strum(to_string = "!!seq", serialize = "tag:yaml.org,2002:seq")]
    Seq,

    #[strum(to_string = "!!str", serialize = "tag:yaml.org,2002:str")]
    Str,

    #[strum(to_string = "!!null", serialize = "tag:yaml.org,2002:null")]
    Null,

    #[strum(to_string = "!!bool", serialize = "tag:yaml.org,2002:bool")]
    Bool,

    #[strum(to_string = "!!int", serialize = "tag:yaml.org,2002:int")]
    Int,

    #[strum(to_string = "!!float", serialize = "tag:yaml.org,2002:float")]
    Float,

    #[strum(to_string = "!!binary", serialize = "tag:yaml.org,2002:binary")]
    Binary,

    #[strum(to_string = "!!omap", serialize = "tag:yaml.org,2002:omap")]
    OMap,

    #[strum(to_string = "!!pairs", serialize = "tag:yaml.org,2002:pairs")]
    Pairs,

    #[strum(to_string = "!!set", serialize = "tag:yaml.org,2002:set")]
    Set,

    #[strum(to_string = "!!merge", serialize = "tag:yaml.org,2002:merge")]
    Merge,

    #[strum(to_string = "!!timestamp", serialize = "tag:yaml.org,2002:timestamp")]
    Timestamp,

    #[strum(to_string = "!!value", serialize = "tag:yaml.org,2002:value")]
    Value, // not really relevant, just for completeness

    #[strum(to_string = "!!yaml", serialize = "tag:yaml.org,2002:yaml")]
    Yaml,

    // Nushell custom tags
    #[strum(to_string = "!glob", serialize = "tag:nushell.sh,2026:glob")]
    Glob,

    #[strum(to_string = "!filesize", serialize = "tag:nushell.sh,2026:filesize")]
    Filesize,

    #[strum(to_string = "!duration", serialize = "tag:nushell.sh,2026:duration")]
    Duration,

    #[strum(to_string = "!date", serialize = "tag:nushell.sh,2026:date")]
    Date,

    #[strum(to_string = "!range", serialize = "tag:nushell.sh,2026:range")]
    Range,

    #[strum(to_string = "!closure", serialize = "tag:nushell.sh,2026:closure")]
    Closure,

    #[strum(to_string = "!error", serialize = "tag:nushell.sh,2026:error")]
    Error,

    #[strum(to_string = "!cell-path", serialize = "tag:nushell.sh,2026:cell-path")]
    CellPath,
}

impl KnownTag {
    pub const NUSHELL_PREFIX: &str = "tag:nushell.sh,2026:";
}

struct UnknownTagError(String);

impl UnknownTagError {
    fn new(tag: impl ToString) -> Self {
        Self(tag.to_string())
    }
}