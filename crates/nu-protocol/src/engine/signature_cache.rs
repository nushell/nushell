use crate::{DeclId, Signature};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

/// Signatures of an [`EngineState`](super::EngineState)'s declarations, each built the first time
/// the parser needs it.
///
/// `Command::signature()` rebuilds a `Signature` (several heap allocations) on every call, and the
/// parser asks for one at least twice per call site (argument parsing and pipeline type checking),
/// so parsing would otherwise rebuild the signature of a command every time it is called, in every
/// parse. A declaration in an `EngineState` never changes (merging a delta only appends), so a
/// signature built for one stays valid for as long as the `EngineState` lives. A clone shares every
/// declaration up to the moment it was cloned, so it keeps the signatures built so far.
///
/// The two maps mirror the two ways the parser reads a signature: the effective signature from
/// [`StateWorkingSet::get_signature_shared`](super::StateWorkingSet::get_signature_shared) (a
/// command backed by a block reports its block's signature) and the declaration's own
/// `Command::signature()` from
/// [`StateWorkingSet::get_decl_signature_shared`](super::StateWorkingSet::get_decl_signature_shared).
/// A `Mutex` (never contended in practice) keeps `EngineState` `Sync`.
#[derive(Default)]
pub(super) struct SignatureCache {
    pub(super) effective: Mutex<HashMap<DeclId, Arc<Signature>>>,
    pub(super) declared: Mutex<HashMap<DeclId, Arc<Signature>>>,
}

impl SignatureCache {
    /// Lock one of the maps. A panic while it was locked left it consistent (every entry is
    /// complete), so a poisoned lock is used as is.
    pub(super) fn lock(
        map: &Mutex<HashMap<DeclId, Arc<Signature>>>,
    ) -> MutexGuard<'_, HashMap<DeclId, Arc<Signature>>> {
        map.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Clone for SignatureCache {
    fn clone(&self) -> Self {
        Self {
            effective: Mutex::new(Self::lock(&self.effective).clone()),
            declared: Mutex::new(Self::lock(&self.declared).clone()),
        }
    }
}
