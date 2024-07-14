mod alias;
mod all;
mod any;
mod append;
mod assignment;
mod break_;
mod bytes;
mod cal;
mod cd;
mod chunks;
mod compact;
mod complete;
mod config_env_default;
mod config_nu_default;
mod continue_;
mod conversions;
#[cfg(feature = "sqlite")]
mod database;
mod date;
mod debug_info;
mod def;
mod default;
mod detect_columns;
mod do_;
mod drop;
mod du;
mod each;
mod echo;
mod empty;
mod error_make;
mod every;
mod exec;
mod export_def;
mod fill;
mod filter;
mod find;
mod first;
mod flatten;
mod for_;
mod format;
mod generate;
mod get;
mod glob;
mod griddle;
mod group_by;
mod hash_;
mod headers;
mod help;
mod histogram;
mod ignore;
mod insert;
mod inspect;
mod interleave;
mod into_datetime;
mod into_filesize;
mod into_int;
mod join;
mod last;
mod length;
mod let_;
mod lines;
mod loop_;
mod ls;
mod match_;
mod math;
mod merge;
mod mktemp;
mod move_;
mod mut_;
mod network;
mod nu_check;
mod open;
mod par_each;
mod parse;
mod path;
mod platform;
mod prepend;
mod print;
#[cfg(feature = "sqlite")]
mod query;
mod random;
mod range;
mod redirection;
mod reduce;
mod reject;
mod rename;
mod return_;
mod reverse;
mod rm;
mod roll;
mod rotate;
mod run_external;
mod save;
mod select;
mod semicolon;
mod seq;
mod seq_char;
mod seq_date;
mod skip;
mod sort;
mod sort_by;
mod source_env;
mod split_by;
mod split_column;
mod split_row;
mod str_;
mod table;
mod take;
mod tee;
mod terminal;
mod to_text;
mod touch;
mod transpose;
mod try_;
mod ucp;
#[cfg(unix)]
mod ulimit;

mod debug;
mod umkdir;
mod uname;
mod uniq;
mod uniq_by;
mod update;
mod upsert;
mod url;
mod use_;
mod where_;
mod which;
mod while_;
mod with_env;
mod wrap;
mod zip;
