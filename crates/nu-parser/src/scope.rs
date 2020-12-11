use nu_source::Spanned;
use std::fmt::Debug;

pub trait ParserScope: Debug {
    fn get_signature(&self, name: &str) -> Option<nu_protocol::Signature>;

    fn has_signature(&self, name: &str) -> bool;

    fn get_alias(&self, name: &str) -> Option<Vec<Spanned<String>>>;

    fn add_alias(&self, name: &str, replacement: Vec<Spanned<String>>);

    fn enter_scope(&self);

    fn exit_scope(&self);
}
