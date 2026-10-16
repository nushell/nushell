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
/// while sharing it starts over with an empty cache of its own (see
/// [`EngineState::merge_delta`]), so engines that diverge never see each other's entries for the
/// declarations they add.
///
/// The two signature maps mirror the two ways the parser reads a signature: the effective
/// signature from
/// [`StateWorkingSet::get_signature_shared`](super::StateWorkingSet::get_signature_shared) (a
/// command backed by a block reports its block's signature) and the declaration's own
/// `Command::signature()` from
/// [`StateWorkingSet::get_decl_signature_shared`](super::StateWorkingSet::get_decl_signature_shared).
/// The two differ only for declarations backed by a block, so only those have an entry in
/// `declared`.
///
/// For the same reason, the output type a signature gives for an input type never changes either.
/// Computing it tests the input against every input/output pair and unions the matching outputs,
/// and the parser does so for every call it parses, so the two output maps remember, per
/// declaration, the answer for up to `MAX_INPUT_TYPES` input types of up to `MAX_COLUMNS` columns
/// each (see
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

    /// The most record and table columns, nested ones included, that an input type may hold for its
    /// output type to be remembered. A larger type (the type of a big record literal, say) is
    /// rarely piped into a command twice; remembering it would keep a deep copy of it for the life
    /// of the engine, and comparing other input types with it can cost more than computing their
    /// output types.
    const MAX_COLUMNS: usize = 16;

    /// Lock one of the maps. A panic while it was locked left it consistent (every entry is
    /// complete), so a poisoned lock is used as is.
    pub(super) fn lock<T>(map: &Mutex<T>) -> MutexGuard<'_, T> {
        map.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Whether `ty` holds at most [`Self::MAX_COLUMNS`] record and table columns, nested ones
    /// included. Counting stops at the first column past the limit.
    fn is_small(ty: &Type) -> bool {
        fn fits(ty: &Type, budget: &mut usize) -> bool {
            match ty {
                Type::Record(columns) | Type::Table(columns) => {
                    match budget.checked_sub(columns.len()) {
                        Some(rest) => *budget = rest,
                        None => return false,
                    }
                    columns.iter().all(|(_, ty)| fits(ty, budget))
                }
                Type::List(ty) => fits(ty, budget),
                Type::OneOf(types) => types.iter().all(|ty| fits(ty, budget)),
                _ => true,
            }
        }
        let mut budget = Self::MAX_COLUMNS;
        fits(ty, &mut budget)
    }

    /// The output type remembered in `map` for `decl_id` and `input`, or else computed with
    /// `compute` and remembered if the declaration has room for another input type. The output
    /// type for a large input type (see [`Self::MAX_COLUMNS`]) is always computed.
    pub(super) fn output_type(
        map: &Mutex<HashMap<DeclId, OutputTypes, FxBuildHasher>>,
        decl_id: DeclId,
        input: Option<&Type>,
        compute: impl FnOnce() -> Option<Type>,
    ) -> Option<Type> {
        if input.is_some_and(|ty| !Self::is_small(ty)) {
            return compute();
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CollectionColumns;

    /// `n` columns of type `ty`.
    fn columns(n: usize, ty: &Type) -> CollectionColumns<Type> {
        (0..n).map(|i| (format!("c{i}"), ty.clone())).collect()
    }

    #[test]
    fn only_types_with_few_columns_are_small() {
        let limit = SignatureCache::MAX_COLUMNS;
        assert!(SignatureCache::is_small(&Type::String));
        assert!(SignatureCache::is_small(&Type::Record(columns(
            limit,
            &Type::Int
        ))));
        assert!(!SignatureCache::is_small(&Type::Record(columns(
            limit + 1,
            &Type::Int
        ))));
        // Nested columns count, through lists and unions too.
        let half = Type::Record(columns(limit / 2, &Type::Int));
        assert!(!SignatureCache::is_small(&Type::Record(columns(
            limit / 2,
            &half
        ))));
        assert!(!SignatureCache::is_small(&Type::list(Type::Table(
            columns(limit + 1, &Type::Int)
        ))));
        assert!(!SignatureCache::is_small(&Type::one_of([
            Type::Record(columns(limit, &Type::Int)),
            Type::Table(columns(1, &Type::Int)),
        ])));
    }

    #[test]
    fn output_types_of_large_input_types_are_not_remembered() {
        let map = Mutex::default();
        let decl_id = DeclId::new(0);
        let large = Type::Record(columns(SignatureCache::MAX_COLUMNS + 1, &Type::Int));
        let output = SignatureCache::output_type(&map, decl_id, Some(&large), || Some(Type::Int));
        assert_eq!(output, Some(Type::Int));
        assert!(SignatureCache::lock(&map).is_empty());

        let small = Type::Record(columns(1, &Type::Int));
        SignatureCache::output_type(&map, decl_id, Some(&small), || Some(Type::Int));
        assert_eq!(SignatureCache::lock(&map)[&decl_id].len(), 1);
    }
}
