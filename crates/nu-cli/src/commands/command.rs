use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use nu_engine::Command;
use nu_errors::ShellError;
use nu_protocol::hir::{self, Block};
use nu_protocol::{CallInfo, EvaluatedArgs, ReturnSuccess, Signature, UntaggedValue, Value};
use parking_lot::Mutex;
use serde::Deserialize;
use std::ops::Deref;
use std::sync::atomic::AtomicBool;

pub struct RunnableContext {
    pub input: InputStream,
    pub shell_manager: ShellManager,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub scope: Scope,
    pub name: Tag,
}

impl RunnableContext {
    pub fn get_command(&self, name: &str) -> Option<Command> {
        self.scope.get_command(name)
    }
}
