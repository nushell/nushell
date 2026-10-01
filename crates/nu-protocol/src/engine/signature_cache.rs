use crate::{DeclId, Signature, Type};
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
/// signature built for one stays valid for as long as the `EngineState` lives. A clone shares every
/// declaration up to the moment it was cloned, so it keeps the signatures built so far.
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
/// declaration, the answer for each input type seen (see
/// [`StateWorkingSet::call_output_type`](super::StateWorkingSet::call_output_type) and
/// [`StateWorkingSet::decl_output_type`](super::StateWorkingSet::decl_output_type)). A command sees
/// few distinct input types, so each declaration keeps a short list compared with `==`.
///
/// A `Mutex` (never contended in practice) keeps `EngineState` `Sync`.
#[derive(Default)]
pub(super) struct SignatureCache {
    pub(super) effective: Mutex<HashMap<DeclId, Arc<Signature>>>,
    pub(super) declared: Mutex<HashMap<DeclId, Arc<Signature>>>,
    pub(super) call_outputs: Mutex<HashMap<DeclId, OutputTypes>>,
    pub(super) declared_outputs: Mutex<HashMap<DeclId, OutputTypes>>,
}

/// The output types a signature gives, for each input type seen.
pub(super) type OutputTypes = Vec<(Option<Type>, Option<Type>)>;

impl SignatureCache {
    /// Lock one of the maps. A panic while it was locked left it consistent (every entry is
    /// complete), so a poisoned lock is used as is.
    pub(super) fn lock<T>(map: &Mutex<T>) -> MutexGuard<'_, T> {
        map.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The output type remembered in `map` for `decl_id` and `input`, computed with `compute` and
    /// remembered the first time.
    pub(super) fn output_type(
        map: &Mutex<HashMap<DeclId, OutputTypes>>,
        decl_id: DeclId,
        input: Option<&Type>,
        compute: impl FnOnce() -> Option<Type>,
    ) -> Option<Type> {
        if let Some((_, output)) = Self::lock(map)
            .get(&decl_id)
            .and_then(|outputs| outputs.iter().find(|(seen, _)| seen.as_ref() == input))
        {
            return output.clone();
        }
        let output = compute();
        Self::lock(map)
            .entry(decl_id)
            .or_default()
            .push((input.cloned(), output.clone()));
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
