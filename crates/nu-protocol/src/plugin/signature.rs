use crate::{PluginExample, Signature};
use serde::Deserialize;
use serde::Serialize;

use crate::engine::Command;
use crate::{BlockId, Category, Flag, PositionalArg, SyntaxShape, Type};

/// A simple wrapper for Signature that includes examples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSignature {
    pub sig: Signature,
    pub examples: Vec<PluginExample>,
}

impl PluginSignature {
    pub fn new(sig: Signature, examples: Vec<PluginExample>) -> Self {
        Self { sig, examples }
    }

    /// Add a default help option to a signature
    pub fn add_help(mut self) -> PluginSignature {
        self.sig = self.sig.add_help();
        self
    }

    /// Build an internal signature with default help option
    pub fn build(name: impl Into<String>) -> PluginSignature {
        let sig = Signature::new(name.into()).add_help();
        Self::new(sig, vec![])
    }

    /// Add a description to the signature
    pub fn usage(mut self, msg: impl Into<String>) -> PluginSignature {
        self.sig = self.sig.usage(msg);
        self
    }

    /// Add an extra description to the signature
    pub fn extra_usage(mut self, msg: impl Into<String>) -> PluginSignature {
        self.sig = self.sig.extra_usage(msg);
        self
    }

    /// Add search terms to the signature
    pub fn search_terms(mut self, terms: Vec<String>) -> PluginSignature {
        self.sig = self.sig.search_terms(terms);
        self
    }

    /// Update signature's fields from a Command trait implementation
    pub fn update_from_command(mut self, command: &dyn Command) -> PluginSignature {
        self.sig = self.sig.update_from_command(command);
        self
    }

    /// Allow unknown signature parameters
    pub fn allows_unknown_args(mut self) -> PluginSignature {
        self.sig = self.sig.allows_unknown_args();
        self
    }

    /// Add a required positional argument to the signature
    pub fn required(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> PluginSignature {
        self.sig = self.sig.required(name, shape, desc);
        self
    }

    /// Add an optional positional argument to the signature
    pub fn optional(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> PluginSignature {
        self.sig = self.sig.optional(name, shape, desc);
        self
    }

    pub fn rest(
        mut self,
        name: &str,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> PluginSignature {
        self.sig = self.sig.rest(name, shape, desc);
        self
    }

    /// Is this command capable of operating on its input via cell paths?
    pub fn operates_on_cell_paths(&self) -> bool {
        self.sig.operates_on_cell_paths()
    }

    /// Add an optional named flag argument to the signature
    pub fn named(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> PluginSignature {
        self.sig = self.sig.named(name, shape, desc, short);
        self
    }

    /// Add a required named flag argument to the signature
    pub fn required_named(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> PluginSignature {
        self.sig = self.sig.required_named(name, shape, desc, short);
        self
    }

    /// Add a switch to the signature
    pub fn switch(
        mut self,
        name: impl Into<String>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> PluginSignature {
        self.sig = self.sig.switch(name, desc, short);
        self
    }

    /// Changes the input type of the command signature
    pub fn input_output_type(mut self, input_type: Type, output_type: Type) -> PluginSignature {
        self.sig.input_output_types.push((input_type, output_type));
        self
    }

    /// Set the input-output type signature variants of the command
    pub fn input_output_types(mut self, input_output_types: Vec<(Type, Type)>) -> PluginSignature {
        self.sig = self.sig.input_output_types(input_output_types);
        self
    }

    /// Changes the signature category
    pub fn category(mut self, category: Category) -> PluginSignature {
        self.sig = self.sig.category(category);
        self
    }

    /// Sets that signature will create a scope as it parses
    pub fn creates_scope(mut self) -> PluginSignature {
        self.sig = self.sig.creates_scope();
        self
    }

    // Is it allowed for the type signature to feature a variant that has no corresponding example?
    pub fn allow_variants_without_examples(mut self, allow: bool) -> PluginSignature {
        self.sig = self.sig.allow_variants_without_examples(allow);
        self
    }

    pub fn call_signature(&self) -> String {
        self.sig.call_signature()
    }

    /// Get list of the short-hand flags
    pub fn get_shorts(&self) -> Vec<char> {
        self.sig.get_shorts()
    }

    /// Get list of the long-hand flags
    pub fn get_names(&self) -> Vec<&str> {
        self.sig.get_names()
    }

    pub fn get_positional(&self, position: usize) -> Option<PositionalArg> {
        self.sig.get_positional(position)
    }

    pub fn num_positionals(&self) -> usize {
        self.sig.num_positionals()
    }

    pub fn num_positionals_after(&self, idx: usize) -> usize {
        self.sig.num_positionals_after(idx)
    }

    /// Find the matching long flag
    pub fn get_long_flag(&self, name: &str) -> Option<Flag> {
        self.sig.get_long_flag(name)
    }

    /// Find the matching long flag
    pub fn get_short_flag(&self, short: char) -> Option<Flag> {
        self.sig.get_short_flag(short)
    }

    /// Set the filter flag for the signature
    pub fn filter(mut self) -> PluginSignature {
        self.sig = self.sig.filter();
        self
    }

    /// Create a placeholder implementation of Command as a way to predeclare a definition's
    /// signature so other definitions can see it. This placeholder is later replaced with the
    /// full definition in a second pass of the parser.
    pub fn predeclare(self) -> Box<dyn Command> {
        self.sig.predeclare()
    }

    /// Combines a signature and a block into a runnable block
    pub fn into_block_command(self, block_id: BlockId) -> Box<dyn Command> {
        self.sig.into_block_command(block_id)
    }

    pub fn formatted_flags(self) -> String {
        self.sig.formatted_flags()
    }

    pub fn plugin_examples(mut self, examples: Vec<PluginExample>) -> PluginSignature {
        self.examples = examples;
        self
    }
}
