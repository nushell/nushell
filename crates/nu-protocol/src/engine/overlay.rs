use crate::{DeclId, ModuleId, OverlayId, VarId};
use rustc_hash::FxBuildHasher;
use std::{
    collections::HashMap,
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};

/// The longest name ever inserted into any [`DeclNameMap`] in this process.
static LONGEST_DECL_NAME: AtomicUsize = AtomicUsize::new(0);

/// An upper bound on the length of every declaration name: every name any [`DeclNameMap`] holds,
/// in any engine state, is at most this long, so no lookup of a longer name can succeed.
///
/// `find_decl` searches only [`DeclNameMap`]s (declarations and predeclarations, in every scope
/// and overlay), so the parser uses this to skip building command-name candidates that cannot
/// match (see `find_longest_decl_with_prefix` in nu-parser). The bound is shared by every engine
/// in the process and only grows, so a long name declared anywhere, even in a scope that is gone,
/// makes it less tight; that only lets the parser build longer candidates, as it did without it.
pub fn longest_decl_name() -> usize {
    LONGEST_DECL_NAME.load(Ordering::Relaxed)
}

/// Name → id map for declarations that remembers the longest name it has ever held.
///
/// Command resolution tries the longest possible command name first and shortens it a word at a
/// time (`find_longest_decl_with_prefix` in nu-parser), looking each candidate up in every map on
/// the scope chain. It only builds candidates up to [`longest_decl_name`], the longest name in any
/// map, but most maps hold much shorter names, such as a script's own definitions or a module's.
/// Knowing the longest name it holds lets [`DeclNameMap::get`] reject a longer candidate before
/// hashing it. The bound only grows (removals leave it alone), so it is always an upper bound on
/// the keys present.
///
/// Reads go through `Deref` to the underlying `HashMap`; all mutation goes through the inherent
/// methods so the bound stays valid.
///
/// The parser looks names up here for every command word it sees, so the map uses the Fx hash
/// (a multiply per 8 bytes) rather than SipHash. Nothing depends on the order of its entries.
/// Unlike SipHash, the Fx hash has no per-process key, so a file could declare names chosen to
/// collide and make its own parse slow, in the LSP or `nu-check` as much as when it runs. That is
/// accepted for the speed, as rustc does, since the names come from the code being parsed.
#[derive(Debug, Clone, Default)]
pub struct DeclNameMap {
    map: HashMap<Vec<u8>, DeclId, FxBuildHasher>,
    longest_name: usize,
}

impl DeclNameMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a declaration by name; names longer than any key ever inserted are rejected
    /// without hashing.
    pub fn get(&self, name: &[u8]) -> Option<&DeclId> {
        if name.len() > self.longest_name {
            return None;
        }
        self.map.get(name)
    }

    pub fn insert(&mut self, name: Vec<u8>, decl_id: DeclId) -> Option<DeclId> {
        self.longest_name = self.longest_name.max(name.len());
        LONGEST_DECL_NAME.fetch_max(name.len(), Ordering::Relaxed);
        self.map.insert(name, decl_id)
    }

    pub fn remove(&mut self, name: &[u8]) -> Option<DeclId> {
        self.map.remove(name)
    }

    pub fn remove_entry(&mut self, name: &[u8]) -> Option<(Vec<u8>, DeclId)> {
        self.map.remove_entry(name)
    }
}

impl Deref for DeclNameMap {
    type Target = HashMap<Vec<u8>, DeclId, FxBuildHasher>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<'a> IntoIterator for &'a DeclNameMap {
    type Item = (&'a Vec<u8>, &'a DeclId);
    type IntoIter = std::collections::hash_map::Iter<'a, Vec<u8>, DeclId>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl IntoIterator for DeclNameMap {
    type Item = (Vec<u8>, DeclId);
    type IntoIter = std::collections::hash_map::IntoIter<Vec<u8>, DeclId>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl Extend<(Vec<u8>, DeclId)> for DeclNameMap {
    fn extend<I: IntoIterator<Item = (Vec<u8>, DeclId)>>(&mut self, iter: I) {
        for (name, decl_id) in iter {
            self.insert(name, decl_id);
        }
    }
}

impl FromIterator<(Vec<u8>, DeclId)> for DeclNameMap {
    fn from_iter<I: IntoIterator<Item = (Vec<u8>, DeclId)>>(iter: I) -> Self {
        let mut map = Self::default();
        map.extend(iter);
        map
    }
}

pub static DEFAULT_OVERLAY_NAME: &str = "zero";

/// Tells whether a decl is visible or not
///
/// Looked up for every declaration a name lookup finds (see [`VisibilityStack`]), so it uses the
/// Fx hash like [`DeclNameMap`].
#[derive(Debug, Clone)]
pub struct Visibility {
    decl_ids: HashMap<DeclId, bool, FxBuildHasher>,
}

/// Name bindings introduced while parsing a single block/closure scope.
///
/// Nested scopes discard their name maps on `exit_scope`; this snapshot is stored on the
/// [`Block`](crate::ast::Block) so `scope` commands can report locals at runtime.
///
/// # Lifecycle
///
/// 1. **Parse**: [`StateWorkingSet::snapshot_scope_bindings`] copies decls/modules from the
///    innermost scope frame into a `ScopeBindings` attached to the block, immediately before
///    the matching `exit_scope`.
/// 2. **Eval**: whole blocks push bindings on [`Stack::active_scope_bindings`] in
///    `eval_ir_block`. Keyword bodies that are IR-inlined record
///    [`ScopeRegion`](crate::ir::ScopeRegion)s on the parent [`IrBlock`](crate::ir::IrBlock);
///    `scope` matches the current instruction index against those regions.
#[derive(Debug, Clone, Default)]
pub struct ScopeBindings {
    pub decls: HashMap<Vec<u8>, DeclId>,
    pub modules: HashMap<Vec<u8>, ModuleId>,
    pub visibility: Visibility,
}

impl ScopeBindings {
    pub fn is_empty(&self) -> bool {
        self.decls.is_empty() && self.modules.is_empty() && self.visibility.decl_ids.is_empty()
    }

    /// Merge decls, modules, and visibility from an overlay frame (other wins on name clash).
    pub fn extend_from_overlay(&mut self, overlay: &OverlayFrame) {
        self.decls
            .extend(overlay.decls.iter().map(|(k, v)| (k.clone(), *v)));
        self.modules
            .extend(overlay.modules.iter().map(|(k, v)| (k.clone(), *v)));
        self.visibility.merge_with(overlay.visibility.clone());
    }

    /// Merge another bindings map on top of this one (other wins on name clash).
    pub fn extend_from_bindings(&mut self, other: &ScopeBindings) {
        self.decls
            .extend(other.decls.iter().map(|(k, v)| (k.clone(), *v)));
        self.modules
            .extend(other.modules.iter().map(|(k, v)| (k.clone(), *v)));
        self.visibility.merge_with(other.visibility.clone());
    }
}

impl Visibility {
    pub fn new() -> Self {
        Visibility {
            decl_ids: HashMap::default(),
        }
    }

    pub fn is_decl_id_visible(&self, decl_id: &DeclId) -> bool {
        *self.decl_ids.get(decl_id).unwrap_or(&true) // by default it's visible
    }

    pub fn hide_decl_id(&mut self, decl_id: &DeclId) {
        self.decl_ids.insert(*decl_id, false);
    }

    pub fn use_decl_id(&mut self, decl_id: &DeclId) {
        self.decl_ids.insert(*decl_id, true);
    }

    /// Overwrite own values with the other
    pub fn merge_with(&mut self, other: Visibility) {
        self.decl_ids.extend(other.decl_ids);
    }

    /// Take new values from the other but keep own values
    pub fn append(&mut self, other: &Visibility) {
        for (decl_id, visible) in other.decl_ids.iter() {
            if !self.decl_ids.contains_key(decl_id) {
                self.decl_ids.insert(*decl_id, *visible);
            }
        }
    }
}

/// Decl visibility resolved across the overlay frames walked so far, innermost frame first.
///
/// Name lookups walk the active overlays from the innermost one outwards. A decl is visible
/// unless one of the frames walked so far has an explicit entry hiding it, and the innermost
/// frame with an entry for the decl wins. This borrows each frame's [`Visibility`] instead of
/// merging the maps: merging copied every entry of every frame on every lookup, which made
/// `find_decl` (called for every command word the parser sees) cost as much as the maps were
/// large.
#[derive(Debug, Default)]
pub struct VisibilityStack<'a> {
    /// The first frames pushed, in order. A lookup rarely walks more than a few frames that
    /// hide declarations (once the standard library is loaded, the permanent overlay is one), so
    /// they are kept here rather than in a `Vec`, which would allocate on every lookup.
    inline: [Option<&'a Visibility>; 4],
    /// The frames pushed after `inline` is full, in order.
    spilled: Vec<&'a Visibility>,
}

impl<'a> VisibilityStack<'a> {
    /// Add the visibility of the next (outer) frame. Frames pushed earlier take precedence.
    ///
    /// A frame that hides nothing can never answer a lookup, so it is not recorded; this keeps
    /// the common lookup (no hidden declarations anywhere) free of allocation.
    pub fn push(&mut self, visibility: &'a Visibility) {
        if visibility.decl_ids.is_empty() {
            return;
        }
        match self.inline.iter_mut().find(|layer| layer.is_none()) {
            Some(layer) => *layer = Some(visibility),
            None => self.spilled.push(visibility),
        }
    }

    /// Whether `decl_id` is visible given the frames pushed so far.
    pub fn is_decl_id_visible(&self, decl_id: &DeclId) -> bool {
        self.inline
            .iter()
            .map_while(|layer| *layer)
            .chain(self.spilled.iter().copied())
            .find_map(|visibility| visibility.decl_ids.get(decl_id))
            .copied()
            .unwrap_or(true) // by default it's visible
    }
}

#[derive(Debug, Clone)]
pub struct ScopeFrame {
    /// List of both active and inactive overlays in this ScopeFrame.
    ///
    /// The order does not have any meaning. Indexed locally (within this ScopeFrame) by
    /// OverlayIds in active_overlays.
    pub overlays: Vec<(Vec<u8>, OverlayFrame)>,

    /// List of currently active overlays.
    ///
    /// Order is significant: The last item points at the last activated overlay.
    pub active_overlays: Vec<OverlayId>,

    /// Removed overlays from previous scope frames / permanent state
    pub removed_overlays: Vec<Vec<u8>>,

    /// temporary storage for predeclarations
    pub predecls: DeclNameMap,
}

impl ScopeFrame {
    pub fn new() -> Self {
        Self {
            overlays: vec![],
            active_overlays: vec![],
            removed_overlays: vec![],
            predecls: DeclNameMap::new(),
        }
    }

    pub fn with_empty_overlay(name: Vec<u8>, origin: ModuleId, prefixed: bool) -> Self {
        Self {
            overlays: vec![(name, OverlayFrame::from_origin(origin, prefixed))],
            active_overlays: vec![OverlayId::new(0)],
            removed_overlays: vec![],
            predecls: DeclNameMap::new(),
        }
    }

    pub fn get_var(&self, var_name: &[u8]) -> Option<&VarId> {
        for overlay_id in self.active_overlays.iter().rev() {
            if let Some(var_id) = self
                .overlays
                .get(overlay_id.get())
                .expect("internal error: missing overlay")
                .1
                .vars
                .get(var_name)
            {
                return Some(var_id);
            }
        }

        None
    }

    pub fn active_overlay_ids(&self, removed_overlays: &mut Vec<Vec<u8>>) -> Vec<OverlayId> {
        for name in &self.removed_overlays {
            if !removed_overlays.contains(name) {
                removed_overlays.push(name.clone());
            }
        }

        self.active_overlays
            .iter()
            .filter(|id| {
                !removed_overlays
                    .iter()
                    .any(|name| name == self.get_overlay_name(**id))
            })
            .copied()
            .collect()
    }

    pub fn active_overlays<'a, 'b>(
        &'b self,
        removed_overlays: &'a mut Vec<Vec<u8>>,
    ) -> impl DoubleEndedIterator<Item = &'b OverlayFrame> + 'a
    where
        'b: 'a,
    {
        // Same filtering as `active_overlay_ids`, but iterated lazily: this runs for every scope
        // frame on every declaration or variable lookup, so it must not allocate.
        for name in &self.removed_overlays {
            if !removed_overlays.contains(name) {
                removed_overlays.push(name.clone());
            }
        }
        let removed_overlays: &'a Vec<Vec<u8>> = removed_overlays;

        self.active_overlays
            .iter()
            .filter(move |id| {
                !removed_overlays
                    .iter()
                    .any(|name| name == self.get_overlay_name(**id))
            })
            .map(|id| self.get_overlay(*id))
    }

    pub fn active_overlay_names(&self, removed_overlays: &mut Vec<Vec<u8>>) -> Vec<&[u8]> {
        self.active_overlay_ids(removed_overlays)
            .iter()
            .map(|id| self.get_overlay_name(*id))
            .collect()
    }

    pub fn get_overlay_name(&self, overlay_id: OverlayId) -> &[u8] {
        &self
            .overlays
            .get(overlay_id.get())
            .expect("internal error: missing overlay")
            .0
    }

    pub fn get_overlay(&self, overlay_id: OverlayId) -> &OverlayFrame {
        &self
            .overlays
            .get(overlay_id.get())
            .expect("internal error: missing overlay")
            .1
    }

    pub fn get_overlay_mut(&mut self, overlay_id: OverlayId) -> &mut OverlayFrame {
        &mut self
            .overlays
            .get_mut(overlay_id.get())
            .expect("internal error: missing overlay")
            .1
    }

    pub fn find_overlay(&self, name: &[u8]) -> Option<OverlayId> {
        self.overlays
            .iter()
            .position(|(n, _)| n == name)
            .map(OverlayId::new)
    }

    pub fn find_active_overlay(&self, name: &[u8]) -> Option<OverlayId> {
        self.overlays
            .iter()
            .position(|(n, _)| n == name)
            .map(OverlayId::new)
            .filter(|id| self.active_overlays.contains(id))
    }
}

#[derive(Debug, Clone)]
pub struct OverlayFrame {
    pub vars: HashMap<Vec<u8>, VarId>,
    pub predecls: DeclNameMap, // temporary storage for predeclarations
    pub decls: DeclNameMap,
    pub modules: HashMap<Vec<u8>, ModuleId>,
    pub shadowed_vars: Vec<VarId>,
    pub visibility: Visibility,
    pub origin: ModuleId, // The original module the overlay was created from
    pub prefixed: bool,   // Whether the overlay has definitions prefixed with its name
}

impl OverlayFrame {
    pub fn from_origin(origin: ModuleId, prefixed: bool) -> Self {
        Self {
            vars: HashMap::new(),
            predecls: DeclNameMap::new(),
            decls: DeclNameMap::new(),
            modules: HashMap::new(),
            shadowed_vars: Vec::new(),
            visibility: Visibility::new(),
            origin,
            prefixed,
        }
    }

    pub fn insert_decl(&mut self, name: Vec<u8>, decl_id: DeclId) -> Option<DeclId> {
        self.decls.insert(name, decl_id)
    }

    pub fn insert_module(&mut self, name: Vec<u8>, module_id: ModuleId) -> Option<ModuleId> {
        self.modules.insert(name, module_id)
    }

    pub fn insert_variable(&mut self, name: Vec<u8>, variable_id: VarId) -> Option<VarId> {
        let res = self.vars.insert(name, variable_id);
        if let Some(old_id) = res {
            self.shadowed_vars.push(old_id);
        }
        res
    }

    pub fn get_decl(&self, name: &[u8]) -> Option<DeclId> {
        self.decls.get(name).cloned()
    }
}

impl Default for Visibility {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ScopeFrame {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod visibility_stack_tests {
    use super::*;

    /// A frame that hides `hidden` and explicitly shows `shown`.
    fn frame(hidden: &[usize], shown: &[usize]) -> Visibility {
        let mut visibility = Visibility::new();
        for id in hidden {
            visibility.hide_decl_id(&DeclId::new(*id));
        }
        for id in shown {
            visibility.use_decl_id(&DeclId::new(*id));
        }
        visibility
    }

    #[test]
    fn innermost_frame_with_an_entry_wins_past_the_inline_frames() {
        // Frame `i` hides decl `i` and shows decl `i + 1`; frames that hide nothing are skipped.
        let frames: Vec<Visibility> = (0..7).map(|i| frame(&[i], &[i + 1])).collect();
        let empty = Visibility::new();
        let mut stack = VisibilityStack::default();
        for visibility in &frames {
            stack.push(&empty);
            stack.push(visibility);
        }
        // Decl 0 is only hidden; every other decl is shown by the frame before the one hiding it.
        assert!(!stack.is_decl_id_visible(&DeclId::new(0)));
        for id in 1..8 {
            assert!(stack.is_decl_id_visible(&DeclId::new(id)), "decl {id}");
        }
        // A decl no frame mentions is visible.
        assert!(stack.is_decl_id_visible(&DeclId::new(100)));

        // The same frames with the hiding order reversed: the innermost entry decides.
        let mut stack = VisibilityStack::default();
        for visibility in frames.iter().rev() {
            stack.push(visibility);
        }
        assert!(!stack.is_decl_id_visible(&DeclId::new(6)));
        assert!(stack.is_decl_id_visible(&DeclId::new(7)));
        assert!(!stack.is_decl_id_visible(&DeclId::new(1)));
    }
}
