use crate::{DeclId, Signature, Type};
use rustc_hash::FxBuildHasher;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

/// Signatures of an [`EngineState`](super::EngineState)'s declarations, each built the first time
/// the parser needs it, and the output types the parser computes from them.
///
/// `Command::signature()` rebuilds a `Signature` (several heap allocations) on every call, and the
/// parser asks for one at least twice per call site (argument parsing and pipeline type checking),
/// so parsing would otherwise rebuild the signature of a command every time it is called, in every
/// parse. A declaration in an `EngineState` never changes (merging a delta only appends), so a
/// signature built for one stays valid for as long as the `EngineState` lives.
///
/// The `EngineState` holds the cache in an `Arc`, so a clone (which the engine makes for every
/// closure it evaluates) shares it instead of copying it. Engines that share it have the same
/// declarations, so every entry is right for all of them; an engine that appends declarations
/// while sharing it first takes its own copy (see [`EngineState::merge_delta`]), so engines that
/// diverge never see each other's entries for the declarations they add.
///
/// The two signature maps mirror the two ways the parser reads a signature: the effective
/// signature from
/// [`StateWorkingSet::get_signature_shared`](super::StateWorkingSet::get_signature_shared) (a
/// command backed by a block reports its block's signature) and the declaration's own
/// `Command::signature()` from
/// [`StateWorkingSet::get_decl_signature_shared`](super::StateWorkingSet::get_decl_signature_shared).
///
/// For the same reason, the output type a signature gives for an input type never changes either.
/// Computing it tests the input against every input/output pair and unions the matching outputs,
/// and the parser does so for every call it parses, so the two output maps remember, per
/// declaration, the answer for up to `MAX_INPUT_TYPES` input types (see
/// [`StateWorkingSet::call_output_type`](super::StateWorkingSet::call_output_type) and
/// [`StateWorkingSet::decl_output_type`](super::StateWorkingSet::decl_output_type)).
///
/// A `Mutex` (rarely contended: only parsing takes it) keeps `EngineState` `Sync`.
///
/// [`EngineState::merge_delta`]: super::EngineState::merge_delta
#[derive(Default)]
pub(super) struct SignatureCache {
    pub(super) effective: Mutex<HashMap<DeclId, Arc<Signature>, FxBuildHasher>>,
    pub(super) declared: Mutex<HashMap<DeclId, Arc<Signature>, FxBuildHasher>>,
    pub(super) call_outputs: Mutex<HashMap<DeclId, OutputTypes, FxBuildHasher>>,
    pub(super) declared_outputs: Mutex<HashMap<DeclId, OutputTypes, FxBuildHasher>>,
}

/// The output types a signature gives, for each input type seen.
pub(super) type OutputTypes = Vec<(Option<Type>, Option<Type>)>;

impl SignatureCache {
    /// The most input types whose output type is remembered per declaration. A command sees few
    /// distinct input types, but a script can pipe any number of record or table shapes into one
    /// command, and every lookup compares the input with each remembered one, so the list stops
    /// growing here; the output type for any other input type is computed every time.
    const MAX_INPUT_TYPES: usize = 16;

    /// Lock one of the maps. A panic while it was locked left it consistent (every entry is
    /// complete), so a poisoned lock is used as is.
    pub(super) fn lock<T>(map: &Mutex<T>) -> MutexGuard<'_, T> {
        map.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The output type remembered in `map` for `decl_id` and `input`, or else computed with
    /// `compute` and remembered if the declaration has room for another input type.
    pub(super) fn output_type(
        map: &Mutex<HashMap<DeclId, OutputTypes, FxBuildHasher>>,
        decl_id: DeclId,
        input: Option<&Type>,
        compute: impl FnOnce() -> Option<Type>,
    ) -> Option<Type> {
        let remembered = |outputs: &OutputTypes| {
            outputs
                .iter()
                .find(|(seen, _)| seen.as_ref() == input)
                .map(|(_, output)| output.clone())
        };
        if let Some(output) = Self::lock(map).get(&decl_id).and_then(remembered) {
            return output;
        }
        // Compute without holding the lock; another parse sharing the cache may remember the same
        // input type meanwhile, so check again before adding it.
        let output = compute();
        let mut map = Self::lock(map);
        let outputs = map.entry(decl_id).or_default();
        if outputs.len() < Self::MAX_INPUT_TYPES && remembered(outputs).is_none() {
            outputs.push((input.cloned(), output.clone()));
        }
        output
    }
}

impl Clone for SignatureCache {
    fn clone(&self) -> Self {
        Self {
            effective: Mutex::new(Self::lock(&self.effective).clone()),
            declared: Mutex::new(Self::lock(&self.declared).clone()),
            call_outputs: Mutex::new(Self::lock(&self.call_outputs).clone()),
            declared_outputs: Mutex::new(Self::lock(&self.declared_outputs).clone()),
        }
    }
}
