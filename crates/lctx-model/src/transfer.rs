//! Transfers and their algebra (DESIGN §15.6).
//!
//! A **transfer** states that a value at an in-place reaches an out-place, owned by one callable,
//! under a condition, in a context (none, or the call site of a composed transfer), with a kind
//! and a provenance class. Sequential composition is the explicit table in [`compose_seq`], never
//! an order over codebook codes. A value crossing an unresolved or unsummarized call is an
//! obligation, not a transfer kind. Composition across a call ([`compose_call`]) is matched by
//! call site: caller transfer ∘ call binding ∘ callee transfer, with the callee condition's
//! formal atoms substituted and its callee-local atoms eliminated under bounded work.

use crate::calls::{BindingStatus, CallBinding};
use crate::condition::Diagram;
use crate::decl::codebook::Codebook;
use crate::id::{Code, Id, recipes};
use crate::obligation::{ObligationKind, from_kernel};
use crate::vocab::{AccessPath, PathRelation, Place, PlaceRoot};

crate::codebook!(
    /// What reaches the out-place. Append-only.
    TransferKind = "transfer_kind" {
        /// The same value arrives unchanged.
        Identity = 0 => "identity",
        /// A value computed from the input arrives.
        Derived = 1 => "derived",
        /// The input decides which value arrives: a predicate atom whose evaluation reads it.
        Control = 2 => "control",
    }
);

crate::codebook!(
    /// Where a transfer comes from. Models are data; a provider's summary may disagree with ours,
    /// visibly. Append-only.
    ProvenanceClass = "provenance_class" {
        FlowLocal = 0 => "flow_local",
        DerivedSummary = 1 => "derived_summary",
        Composed = 2 => "composed",
        AuthoredModel = 3 => "authored_model",
        /// A provider's taint-in-taint-out summary (Pysa TITO).
        ProviderSummary = 4 => "provider_summary",
        CatalogFieldLink = 5 => "catalog_field_link",
    }
);

/// The result of composing two transfer kinds in sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Composition {
    Transfer(TransferKind),
    /// A control influence on an atom that guards a value transfer: a selection relation, not a
    /// sequential transfer.
    Selection,
}

/// The sequential composition table: `first` then `then`.
///
/// | first \ then | identity | derived | control |
/// |---|---|---|---|
/// | identity | identity | derived | selection |
/// | derived | derived | derived | selection |
/// | control | selection | selection | selection |
pub fn compose_seq(first: TransferKind, then: TransferKind) -> Composition {
    use TransferKind::*;
    match (first, then) {
        (Control, _) | (_, Control) => Composition::Selection,
        (Identity, Identity) => Composition::Transfer(Identity),
        (Identity | Derived, Identity | Derived) => Composition::Transfer(Derived),
    }
}

/// One transfer.
#[derive(Clone, Debug)]
pub struct Transfer {
    pub owner: Id,
    pub input: Place,
    pub output: Place,
    pub kind: TransferKind,
    pub condition: Diagram,
    /// The call site a composed or instantiated transfer is specific to.
    pub context: Option<Id>,
    pub provenance: ProvenanceClass,
}

impl Transfer {
    /// The transfer's identity (DESIGN §15.3): owner, places, kind, condition, context and
    /// provenance class. Derivations, depth and witnesses never enter it.
    pub fn id(&self) -> Id {
        recipes::transfer(
            self.owner,
            self.input.id(),
            self.output.id(),
            Code(self.kind),
            self.condition.id(),
            self.context,
            Code(self.provenance),
        )
    }

    /// The semantic key alternatives merge on: everything but the condition.
    pub fn key(&self) -> (Id, Id, Id, i16, Option<Id>, i16) {
        (
            self.owner,
            self.input.id(),
            self.output.id(),
            self.kind.code(),
            self.context,
            self.provenance.code(),
        )
    }
}

/// Merge alternatives with equal semantic keys by OR-ing their conditions. Distinct keys stay
/// separate rows; the output is ordered by key.
pub fn merge(transfers: Vec<Transfer>) -> Result<Vec<Transfer>, ObligationKind> {
    let mut by_key: std::collections::BTreeMap<_, Transfer> = std::collections::BTreeMap::new();
    for t in transfers {
        match by_key.get_mut(&t.key()) {
            Some(existing) => {
                existing.condition = existing.condition.or(&t.condition).map_err(from_kernel)?;
            }
            None => {
                by_key.insert(t.key(), t);
            }
        }
    }
    Ok(by_key.into_values().collect())
}

/// Everything [`compose_call`] needs at one call site besides the two transfers.
#[derive(Clone, Debug)]
pub struct CallSite<'a> {
    /// The call site occurrence; the composed transfer's context and the caller-side result
    /// place's root.
    pub site: Id,
    /// The binding of the caller transfer's out-place (an actual) to the callee transfer's
    /// in-place root (a formal).
    pub binding: &'a CallBinding,
    /// Replacements for the callee condition's formal atoms, in caller atoms.
    pub formal_atoms: &'a [(Id, &'a Diagram)],
}

/// The outcome of composing across one call.
#[derive(Clone, Debug)]
pub enum CallComposition {
    Transfer(Transfer),
    /// The callee reads a different sub-value of the actual than the caller delivers: no flow.
    Disjoint,
    /// A control influence met a value transfer (see [`Composition::Selection`]).
    Selection,
    /// The call cannot carry the value: an unbound, ambiguous or refused binding, a mismatched
    /// binding, or a condition over its budget.
    Obligation(ObligationKind),
}

/// Compose a caller transfer into an actual, the actual's binding and a callee transfer from the
/// bound formal (DESIGN §15.6).
///
/// - The binding must be bound and must connect exactly the caller's out-place root (an
///   occurrence) and the callee's in-place root (the formal).
/// - Paths: the caller delivers its in-place `X` at `actual.Q`; the callee reads `formal.P`.
///   When `Q` is a prefix of `P`, `X.(P−Q)` flows; when `P` is a prefix of `Q`, `X` flows into the
///   out-place at `(Q−P)`; different sub-values do not flow ([`CallComposition::Disjoint`]); an
///   unknown suffix that hides the relation keeps the flow as `derived` from `X` with an unknown
///   suffix, never as identity.
/// - A callee out-place at its own return becomes the call site's result; a formal-rooted
///   out-place maps back through the binding onto the actual's root; any other root is kept.
/// - The condition is the caller condition ∧ the callee condition with formal atoms substituted
///   and every other callee atom eliminated existentially, under the kernel's budgets.
pub fn compose_call(caller: &Transfer, site: &CallSite<'_>, callee: &Transfer) -> CallComposition {
    let binding = site.binding;
    if binding.status != BindingStatus::Bound {
        return CallComposition::Obligation(
            binding.obligation().unwrap_or(ObligationKind::AmbiguousBinding),
        );
    }
    let actual_matches = binding
        .actual
        .is_some_and(|a| caller.output.root == PlaceRoot::Occurrence(a));
    let formal_matches = binding
        .formal
        .as_ref()
        .is_some_and(|f| callee.input.root == PlaceRoot::Formal(f.clone()));
    if !actual_matches || !formal_matches || binding.site != site.site {
        return CallComposition::Obligation(ObligationKind::AmbiguousBinding);
    }
    let mut kind = match compose_seq(caller.kind, callee.kind) {
        Composition::Transfer(kind) => kind,
        Composition::Selection => return CallComposition::Selection,
    };
    let base_output = match &callee.output.root {
        PlaceRoot::Return { callable } if *callable == callee.owner => {
            Place::root(PlaceRoot::Occurrence(site.site)).extended(&callee.output.path)
        }
        PlaceRoot::Formal(f) if Some(f) == binding.formal.as_ref() => {
            Place::root(caller.output.root.clone()).extended(&callee.output.path)
        }
        _ => callee.output.clone(),
    };
    let (input, output) = match callee.input.path.strip_prefix(&caller.output.path) {
        PathRelation::Rest(rest) => (caller.input.extended(&rest), base_output),
        PathRelation::Shorter(rest) => (caller.input.clone(), base_output.extended(&rest)),
        PathRelation::Disjoint => return CallComposition::Disjoint,
        PathRelation::Unknown => {
            kind = TransferKind::Derived;
            (caller.input.extended(&AccessPath::unknown()), base_output)
        }
    };
    let condition = (|| {
        let substituted = callee
            .condition
            .substitute_atoms(site.formal_atoms)
            .map_err(from_kernel)?;
        let formal_targets: std::collections::BTreeSet<Id> = site
            .formal_atoms
            .iter()
            .flat_map(|(_, d)| d.support().iter().copied())
            .collect();
        let locals: Vec<Id> = substituted
            .support()
            .iter()
            .copied()
            .filter(|a| !formal_targets.contains(a) && !caller.condition.support().contains(a))
            .collect();
        let projected = substituted.exists(&locals).map_err(from_kernel)?;
        caller.condition.and(&projected).map_err(from_kernel)
    })();
    match condition {
        Ok(condition) => CallComposition::Transfer(Transfer {
            owner: caller.owner,
            input,
            output,
            kind,
            condition,
            context: Some(site.site),
            provenance: ProvenanceClass::Composed,
        }),
        Err(obligation) => CallComposition::Obligation(obligation),
    }
}

/// The obligation a value crossing a call with no instantiable summary leaves (DESIGN §15.6):
/// nothing is inferred through an unknown callee.
pub fn open_call(target: crate::calls::TargetKind) -> ObligationKind {
    match target {
        crate::calls::TargetKind::Unresolved => ObligationKind::UnresolvedTarget,
        _ => ObligationKind::CallTransfer,
    }
}
