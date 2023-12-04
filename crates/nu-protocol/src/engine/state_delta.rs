use super::{usage::Usage, Command, EngineState, OverlayFrame, ScopeFrame, VirtualPath};
use crate::ast::Block;
use crate::{Module, Variable};

/// A delta (or change set) between the current global state and a possible future global state. Deltas
/// can be applied to the global state to update it to contain both previous state and the state held
/// within the delta.
pub struct StateDelta {
    pub(super) files: Vec<(String, usize, usize)>,
    pub(crate) file_contents: Vec<(Vec<u8>, usize, usize)>,
    pub(super) virtual_paths: Vec<(String, VirtualPath)>,
    pub(super) vars: Vec<Variable>,          // indexed by VarId
    pub(super) decls: Vec<Box<dyn Command>>, // indexed by DeclId
    pub blocks: Vec<Block>,                  // indexed by BlockId
    pub(super) modules: Vec<Module>,         // indexed by ModuleId
    pub(super) usage: Usage,
    pub scope: Vec<ScopeFrame>,
    #[cfg(feature = "plugin")]
    pub(super) plugins_changed: bool, // marks whether plugin file should be updated
}

impl StateDelta {
    pub fn new(engine_state: &EngineState) -> Self {
        let last_overlay = engine_state.last_overlay(&[]);
        let scope_frame = ScopeFrame::with_empty_overlay(
            engine_state.last_overlay_name(&[]).to_owned(),
            last_overlay.origin,
            last_overlay.prefixed,
        );

        StateDelta {
            files: vec![],
            file_contents: vec![],
            virtual_paths: vec![],
            vars: vec![],
            decls: vec![],
            blocks: vec![],
            modules: vec![],
            scope: vec![scope_frame],
            usage: Usage::new(),
            #[cfg(feature = "plugin")]
            plugins_changed: false,
        }
    }

    pub fn num_files(&self) -> usize {
        self.files.len()
    }

    pub fn num_virtual_paths(&self) -> usize {
        self.virtual_paths.len()
    }

    pub fn num_decls(&self) -> usize {
        self.decls.len()
    }

    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    pub fn num_modules(&self) -> usize {
        self.modules.len()
    }

    pub fn last_scope_frame_mut(&mut self) -> &mut ScopeFrame {
        self.scope
            .last_mut()
            .expect("internal error: missing required scope frame")
    }

    pub fn last_scope_frame(&self) -> &ScopeFrame {
        self.scope
            .last()
            .expect("internal error: missing required scope frame")
    }

    pub fn last_overlay_mut(&mut self) -> Option<&mut OverlayFrame> {
        let last_scope = self
            .scope
            .last_mut()
            .expect("internal error: missing required scope frame");

        if let Some(last_overlay_id) = last_scope.active_overlays.last() {
            Some(
                &mut last_scope
                    .overlays
                    .get_mut(*last_overlay_id)
                    .expect("internal error: missing required overlay")
                    .1,
            )
        } else {
            None
        }
    }

    pub fn last_overlay(&self) -> Option<&OverlayFrame> {
        let last_scope = self
            .scope
            .last()
            .expect("internal error: missing required scope frame");

        if let Some(last_overlay_id) = last_scope.active_overlays.last() {
            Some(
                &last_scope
                    .overlays
                    .get(*last_overlay_id)
                    .expect("internal error: missing required overlay")
                    .1,
            )
        } else {
            None
        }
    }

    pub fn enter_scope(&mut self) {
        self.scope.push(ScopeFrame::new());
    }

    pub fn exit_scope(&mut self) {
        self.scope.pop();
    }

    pub fn get_file_contents(&self) -> &[(Vec<u8>, usize, usize)] {
        &self.file_contents
    }
}
