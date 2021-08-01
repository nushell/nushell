use nu_protocol::hir::Block;
use nu_source::Spanned;
use std::{fmt::Debug, sync::Arc};

pub trait ParserScope: Debug {
    fn get_signature(&self, name: &str) -> Option<nu_protocol::Signature>;

    fn has_signature(&self, name: &str) -> bool;

    fn add_definition(&self, block: Arc<Block>);

    fn get_definitions(&self) -> Vec<Arc<Block>>;

    fn get_alias(&self, name: &str) -> Option<Vec<Spanned<String>>>;

    fn remove_alias(&self, name: &str);

    fn add_alias(&self, name: &str, replacement: Vec<Spanned<String>>);

    fn enter_scope(&self);

    fn exit_scope(&self);
}
