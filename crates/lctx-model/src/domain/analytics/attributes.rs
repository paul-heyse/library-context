//! Structural FCA attributes and optional layers use exact native/normalized inputs.
use super::{
    build::{Data, invalid, need},
    *,
};
use crate::domain::{normalized::entities::*, resources::ResourceBudget};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
/// Borrowed immutable correspondence for one selected analytic invocation. Indexing never
/// resolves premises eagerly: missing/qualified rows are checked at the original consumer.
type CallableContext = (Id<CallableEntity>, Id<attribution::AnalysisContext>);
type SymbolContext = (Id<calls::ProviderSymbol>, Id<attribution::AnalysisContext>);
type ParameterCache = BTreeMap<
    (
        Id<syntax::ParameterSyntaxObservation>,
        Id<attribution::AnalysisContext>,
    ),
    Option<Rc<normalized::parameter_correspondence::SourceParameterCorrespondence>>,
>;
type ClassCache =
    BTreeMap<(Id<types::TypeTerm>, Id<attribution::AnalysisContext>), Option<Rc<ClassSet>>>;
pub(super) struct Correspondence<'a> {
    resolutions: BTreeMap<SymbolContext, Vec<&'a SymbolEntityResolution>>,
    members: BTreeMap<Id<types::TypeSequence>, Vec<&'a types::TypeSequenceMember>>,
    slots: BTreeMap<Id<types::CallableParameterList>, Vec<&'a types::CallableParameter>>,
    placements: BTreeMap<Option<Id<source::Occurrence>>, Vec<&'a syntax::SyntaxPlacement>>,
    parameter_declarations:
        BTreeMap<Id<source::Occurrence>, Vec<&'a declarations::ParameterDeclaration>>,
    entity_declarations:
        BTreeMap<Id<calls::ProviderSymbol>, Vec<&'a declarations::SymbolDeclaration>>,
    pub(super) variants:
        BTreeMap<CallableContext, Vec<&'a normalized::callables::SignatureVariant>>,
    variant_slots: BTreeMap<
        Id<normalized::callables::SignatureVariant>,
        Vec<&'a normalized::callables::SignatureSlot>,
    >,
    parameters: BTreeMap<Id<source::Occurrence>, Vec<&'a syntax::ParameterSyntaxObservation>>,
    observations: BTreeMap<Id<source::Occurrence>, Vec<&'a types::TypeObservation>>,
    decorators: BTreeMap<Id<source::Occurrence>, Vec<&'a syntax::DeclarationDecorator>>,
    mention_candidates: BTreeMap<
        Id<normalized::links::MentionEntityAssessment>,
        Vec<&'a normalized::links::MentionEntityCandidate>,
    >,
    exposure_candidates: BTreeMap<Id<PublicExposure>, Vec<&'a PublicExposureCandidate>>,
    roles: BTreeSet<(Id<input::InputRevision>, Id<source::SourceArtifact>, i16)>,
    corpus: BTreeSet<(Id<input::InputRevision>, Id<input::InputRevision>)>,
    pub(super) owners: BTreeMap<Id<EntityRef>, Vec<&'a normalized::entities::OccurrenceOwnership>>,
    pub(super) metadata: BTreeMap<
        (Id<EntityRef>, Id<attribution::AnalysisContext>),
        Vec<&'a class_metadata::ClassMetadataObservation>,
    >,
    pub(super) class_members: BTreeMap<
        (Id<EntityRef>, Id<attribution::AnalysisContext>),
        Vec<&'a class_metadata::ClassMemberObservation>,
    >,
    pub(super) captures: BTreeMap<
        (Id<EntityRef>, Id<attribution::AnalysisContext>),
        Vec<&'a captures::CaptureObservation>,
    >,
    pub(super) exits: BTreeMap<Id<source::Occurrence>, Vec<&'a protocols::NativeExitObservation>>,
    pub(super) terminals:
        BTreeMap<Id<source::Occurrence>, Vec<&'a protocols::NativeTerminalObservation>>,
    pub(super) metadata_supports: BTreeMap<
        Id<class_metadata::ClassMetadataObservation>,
        Vec<&'a class_metadata::ClassMetadataSupport>,
    >,
    pub(super) member_supports: BTreeMap<
        Id<class_metadata::ClassMemberObservation>,
        Vec<&'a class_metadata::ClassMemberSupport>,
    >,
    pub(super) capture_supports:
        BTreeMap<Id<captures::CaptureObservation>, Vec<&'a captures::CaptureSupport>>,
    pub(super) exit_supports:
        BTreeMap<Id<protocols::NativeExitObservation>, Vec<&'a protocols::NativeExitSupport>>,
    pub(super) terminal_supports: BTreeMap<
        Id<protocols::NativeTerminalObservation>,
        Vec<&'a protocols::NativeTerminalSupport>,
    >,
    pub(super) native_signature_supports:
        BTreeMap<Id<types::NativeSignatureObservation>, Vec<&'a types::NativeSignatureSupport>>,
    pub(super) port_supports:
        BTreeMap<Id<types::SignatureTypeObservation>, Vec<&'a types::SignatureTypeSupport>>,
    pub(super) ports: BTreeMap<Id<calls::Signature>, Vec<&'a types::SignatureTypeObservation>>,
    pub(super) references: BTreeMap<Id<source::Occurrence>, Vec<&'a lexical::ReferenceObservation>>,
    pub(super) assessments: BTreeMap<
        Id<lexical::ReferenceObservation>,
        Vec<&'a normalized::links::ReferenceEntityAssessment>,
    >,
    pub(super) reference_candidates: BTreeMap<
        Id<normalized::links::ReferenceEntityAssessment>,
        Vec<&'a normalized::links::ReferenceEntityCandidate>,
    >,
    parameter_cache: RefCell<ParameterCache>,
    class_cache: RefCell<ClassCache>,
    _charge: RefCell<charged::StateCharge>,
}
fn grouped<'a, K: Ord, R: Record>(
    rows: &'a crate::domain::normalized::Rows<R>,
    key: impl Fn(&R) -> K,
    charge: &mut charged::StateCharge,
) -> Result<BTreeMap<K, Vec<&'a R>>, ModelError> {
    let mut out = BTreeMap::new();
    for row in rows.iter() {
        append_group(&mut out, key(row), row, charge)?;
    }
    Ok(out)
}
type EntityContextIndex<'a, R> =
    BTreeMap<(Id<EntityRef>, Id<attribution::AnalysisContext>), Vec<&'a R>>;
fn attributed_grouped<'a, R: Record>(
    rows: &'a crate::domain::normalized::Rows<R>,
    symbol: impl Fn(&R) -> Id<calls::ProviderSymbol>,
    resolutions: &BTreeMap<SymbolContext, Vec<&SymbolEntityResolution>>,
    charge: &mut charged::StateCharge,
) -> Result<EntityContextIndex<'a, R>, ModelError> {
    let mut scratch = charged::StateCharge::new(
        charge
            .budget()
            .ok_or_else(|| invalid("analytic attributed index has no budget"))?,
        "analytic-attributed-index-scratch",
    );
    let symbols = grouped(rows, symbol, &mut scratch)?;
    let mut out = BTreeMap::new();
    for ((symbol, ctx), resolutions) in resolutions {
        let mut entities = BTreeSet::new();
        for row in resolutions
            .iter()
            .filter(|r| r.status == ResolutionStatus::Resolved)
        {
            if let Some(entity) = row.entity
                && !entities.contains(&entity)
            {
                scratch.grow(160)?;
                entities.insert(entity);
            }
        }
        for entity in entities {
            for row in symbols.get(symbol).into_iter().flatten() {
                append_group(&mut out, (entity, *ctx), *row, charge)?;
            }
        }
    }
    for rows in out.values_mut() {
        rows.sort_unstable_by_key(|r| r.id());
        rows.dedup_by_key(|r| r.id());
    }
    Ok(out)
}
fn append_group<'a, K: Ord, R>(
    out: &mut BTreeMap<K, Vec<&'a R>>,
    key: K,
    row: &'a R,
    charge: &mut charged::StateCharge,
) -> Result<(), ModelError> {
    if !out.contains_key(&key) {
        charge.grow(std::mem::size_of::<K>() + std::mem::size_of::<Vec<&R>>() + 128)?;
    }
    let bucket = out.entry(key).or_default();
    if bucket.len() == bucket.capacity() {
        let capacity = bucket
            .capacity()
            .checked_mul(2)
            .ok_or_else(|| invalid("analytic index capacity overflow"))?
            .max(4);
        charge.grow(
            (capacity - bucket.capacity())
                .checked_mul(std::mem::size_of::<&R>())
                .ok_or_else(|| invalid("analytic index capacity overflow"))?,
        )?;
        bucket
            .try_reserve_exact(capacity - bucket.len())
            .map_err(|_| invalid("analytic index allocation refused"))?;
        if bucket.capacity() > capacity {
            charge.grow(
                (bucket.capacity() - capacity)
                    .checked_mul(std::mem::size_of::<&R>())
                    .ok_or_else(|| invalid("analytic index capacity overflow"))?,
            )?;
        }
    }
    bucket.push(row);
    Ok(())
}
fn selected_ids<R: Record>(
    rows: impl Iterator<Item = Id<R>>,
    maximum: usize,
    charge: &mut charged::StateCharge,
) -> Result<Vec<Id<R>>, ModelError> {
    charge.grow(
        maximum
            .checked_mul(std::mem::size_of::<Id<R>>())
            .ok_or_else(|| invalid("analytic selected IDs overflow"))?,
    )?;
    let mut ids = Vec::with_capacity(maximum);
    if ids.capacity() > maximum {
        charge.grow(
            (ids.capacity() - maximum)
                .checked_mul(std::mem::size_of::<Id<R>>())
                .ok_or_else(|| invalid("analytic selected IDs overflow"))?,
        )?;
    }
    ids.extend(rows);
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}
impl<'a> Correspondence<'a> {
    pub(super) fn new(
        d: &'a Data,
        b: &ResourceBudget,
        typed: bool,
        concepts: bool,
        mentions: bool,
    ) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(b, "analytic-correspondence");
        let resolutions = if typed {
            grouped(
                &d.native.symbol_resolutions,
                |r| (r.symbol, r.context),
                &mut charge,
            )?
        } else {
            BTreeMap::new()
        };
        let members = if typed {
            grouped(&d.type_members, |r| r.sequence, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let slots = if typed {
            grouped(&d.callable_type_slots, |r| r.list, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let placements = if typed {
            grouped(&d.native.placements, |r| r.parent, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let parameter_declarations = if typed {
            grouped(
                &d.native.parameter_declarations,
                |r| r.declaration,
                &mut charge,
            )?
        } else {
            BTreeMap::new()
        };
        let entity_declarations = if typed {
            grouped(&d.native.entity_declarations, |r| r.symbol, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let variant_slots = if typed {
            grouped(&d.native.callable_slots, |r| r.variant, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let mut variants = BTreeMap::new();
        if typed {
            for row in d.native.callable_variants.iter() {
                if let Some(callable) = row.callable {
                    append_group(&mut variants, (callable, row.context), row, &mut charge)?;
                }
            }
        }
        let parameters = if typed {
            grouped(&d.parameter_syntax, |r| r.function, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let observations = if typed {
            grouped(&d.native.type_observations, |r| r.subject, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let decorators = if concepts {
            grouped(&d.native.decorators, |r| r.declaration, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let mention_candidates = if mentions {
            grouped(
                &d.native.relation_mention_entity_candidates,
                |r| r.assessment,
                &mut charge,
            )?
        } else {
            BTreeMap::new()
        };
        let exposure_candidates = if mentions {
            grouped(
                &d.native.entity_exposure_candidates,
                |r| r.exposure,
                &mut charge,
            )?
        } else {
            BTreeMap::new()
        };
        let mut roles = BTreeSet::new();
        if typed || mentions {
            for row in d.uses.iter() {
                let key = (row.input, row.artifact, row.role as i16);
                if !roles.contains(&key) {
                    charge.grow(std::mem::size_of_val(&key) + 128)?;
                    roles.insert(key);
                }
            }
        }
        let mut corpus = BTreeSet::new();
        if mentions {
            for row in d.corpus_libraries.iter() {
                let key = (row.corpus, row.library);
                if !corpus.contains(&key) {
                    charge.grow(std::mem::size_of_val(&key) + 128)?;
                    corpus.insert(key);
                }
            }
        }
        let owners = if concepts {
            grouped(&d.native.owners, |r| r.entity, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let metadata = if concepts {
            attributed_grouped(&d.class_metadata, |r| r.class, &resolutions, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let class_members = if concepts {
            attributed_grouped(&d.class_members, |r| r.class, &resolutions, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let captures = if concepts {
            attributed_grouped(&d.captures, |r| r.function, &resolutions, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let exits = if concepts {
            grouped(&d.exits, |r| r.subject, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let terminals = if concepts {
            grouped(&d.terminals, |r| r.subject, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let metadata_supports = if concepts {
            grouped(&d.metadata_supports, |r| r.assertion, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let member_supports = if concepts {
            grouped(&d.member_supports, |r| r.assertion, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let capture_supports = if concepts {
            grouped(&d.capture_supports, |r| r.assertion, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let exit_supports = if concepts {
            grouped(&d.exit_supports, |r| r.assertion, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let terminal_supports = if concepts {
            grouped(&d.terminal_supports, |r| r.assertion, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let native_signature_supports = if concepts {
            grouped(&d.native_signature_supports, |r| r.assertion, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let port_supports = if concepts {
            grouped(&d.port_supports, |r| r.assertion, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let references = if concepts {
            grouped(&d.native.references, |r| r.read, &mut charge)?
        } else {
            BTreeMap::new()
        };
        let assessments = if concepts {
            grouped(
                &d.native.reference_assessments,
                |r| r.reference,
                &mut charge,
            )?
        } else {
            BTreeMap::new()
        };
        let reference_candidates = if concepts {
            grouped(
                &d.native.reference_candidates,
                |r| r.assessment,
                &mut charge,
            )?
        } else {
            BTreeMap::new()
        };
        let mut ports = BTreeMap::new();
        if concepts {
            for row in d.native.signature_types.iter() {
                let signature = match d.native.signature_type_subjects.get(row.subject) {
                    Some(types::SignatureTypeSubject::Return { signature }) => Some(*signature),
                    Some(types::SignatureTypeSubject::Parameter { parameter }) => {
                        d.native.parameters.get(*parameter).map(|p| p.signature)
                    }
                    None => None,
                };
                if let Some(signature) = signature {
                    append_group(&mut ports, signature, row, &mut charge)?;
                }
            }
        }
        Ok(Self {
            owners,
            metadata,
            class_members,
            captures,
            exits,
            terminals,
            metadata_supports,
            member_supports,
            capture_supports,
            exit_supports,
            terminal_supports,
            native_signature_supports,
            port_supports,
            ports,
            references,
            assessments,
            reference_candidates,
            resolutions,
            members,
            slots,
            placements,
            parameter_declarations,
            entity_declarations,
            variants,
            variant_slots,
            parameters,
            observations,
            decorators,
            mention_candidates,
            exposure_candidates,
            roles,
            corpus,
            parameter_cache: RefCell::new(BTreeMap::new()),
            class_cache: RefCell::new(BTreeMap::new()),
            _charge: RefCell::new(charge),
        })
    }
    pub(super) fn by_entity<'b, R: Record>(
        &self,
        index: &'b BTreeMap<(Id<EntityRef>, Id<attribution::AnalysisContext>), Vec<&'a R>>,
        entity: Id<EntityRef>,
        ctx: Id<attribution::AnalysisContext>,
    ) -> &'b [&'a R] {
        index.get(&(entity, ctx)).map_or(&[], Vec::as_slice)
    }
    pub(super) fn candidates<K: Ord, R: Record>(
        &self,
        index: &BTreeMap<K, Vec<&'a R>>,
        keys: impl Iterator<Item = K>,
        charge: &mut charged::StateCharge,
    ) -> Result<Vec<&'a R>, ModelError> {
        let mut selected = BTreeMap::new();
        for key in keys {
            for row in index.get(&key).into_iter().flatten() {
                if let std::collections::btree_map::Entry::Vacant(entry) = selected.entry(row.id())
                {
                    charge.grow(160)?;
                    entry.insert(*row);
                }
            }
        }
        charge.grow(
            selected
                .len()
                .checked_mul(std::mem::size_of::<&R>())
                .ok_or_else(|| invalid("analytic candidate selection overflow"))?,
        )?;
        Ok(selected.into_values().collect())
    }
    pub(super) fn decorator(
        &self,
        d: &Data,
        decorator: &syntax::DeclarationDecorator,
        ctx: Id<attribution::AnalysisContext>,
        b: &ResourceBudget,
    ) -> Result<normalized::decorator_identity::Output, ModelError> {
        let mut charge = charged::StateCharge::new(b, "analytic-decorator-candidates");
        // Select structural possibilities only. The canonical selector retains all qualification,
        // ambiguity and target checks, including its early returns on a malformed head.
        let root = decorator.decorator;
        let first = self
            .placements
            .get(&Some(root))
            .map_or(&[][..], Vec::as_slice);
        charge.grow(
            (first.len() + 1)
                .checked_mul(64)
                .ok_or_else(|| invalid("decorator heads overflow"))?,
        )?;
        let mut parents = vec![Some(root)];
        parents.extend(
            first
                .iter()
                .filter(|p| p.field == lexical::SyntaxField::Child)
                .map(|p| Some(p.occurrence)),
        );
        let placements = self.candidates(&self.placements, parents.into_iter(), &mut charge)?;
        let placement_ids = selected_ids(
            placements.iter().map(|r| r.id()),
            placements.len(),
            &mut charge,
        )?;
        let references = self.candidates(
            &self.references,
            std::iter::once(root).chain(placements.iter().map(|p| p.occurrence)),
            &mut charge,
        )?;
        let reference_ids = selected_ids(
            references.iter().map(|r| r.id()),
            references.len(),
            &mut charge,
        )?;
        let assessments = self.candidates(
            &self.assessments,
            references.iter().map(|r| r.id()),
            &mut charge,
        )?;
        let assessment_ids = selected_ids(
            assessments.iter().map(|r| r.id()),
            assessments.len(),
            &mut charge,
        )?;
        let candidates = self.candidates(
            &self.reference_candidates,
            assessments.iter().map(|r| r.id()),
            &mut charge,
        )?;
        let candidate_ids = selected_ids(
            candidates.iter().map(|r| r.id()),
            candidates.len(),
            &mut charge,
        )?;
        normalized::decorator_identity::SelectedInputs {
            qualifications: d.native.qualifications.view(),
            occurrences: d.native.occurrences.view(),
            placements: normalized::RowsView::selected(&d.native.placements, &placement_ids)?,
            references: normalized::RowsView::selected(&d.native.references, &reference_ids)?,
            assessments: normalized::RowsView::selected(
                &d.native.reference_assessments,
                &assessment_ids,
            )?,
            candidates: normalized::RowsView::selected(
                &d.native.reference_candidates,
                &candidate_ids,
            )?,
            targets: d.native.reference_targets.view(),
            resolutions: d.native.lexical_resolutions.view(),
        }
        .select(decorator, ctx, b)
    }
    fn parameter(
        &self,
        d: &Data,
        parameter: &syntax::ParameterSyntaxObservation,
        ctx: Id<attribution::AnalysisContext>,
        b: &ResourceBudget,
    ) -> Result<
        Option<Rc<normalized::parameter_correspondence::SourceParameterCorrespondence>>,
        ModelError,
    > {
        if normalized::parameter_correspondence::source_parameter_container(
            &d.native.view(),
            parameter,
            ctx,
        )
        .is_none()
        {
            return Ok(None);
        }
        let mut cache = self.parameter_cache.borrow_mut();
        let key = (parameter.id(), ctx);
        if let Some(value) = cache.get(&key) {
            return Ok(value.clone());
        }
        self._charge.borrow_mut().grow(512)?;
        let mut scratch = charged::StateCharge::new(b, "analytic-parameter-selection");
        let placements = self
            .placements
            .get(&Some(parameter.parameter))
            .map_or(&[][..], Vec::as_slice);
        let placement_ids = selected_ids(
            placements.iter().map(|row| row.id()),
            placements.len(),
            &mut scratch,
        )?;
        scratch.grow(
            (placements.len() + 1)
                .checked_mul(128)
                .ok_or_else(|| invalid("analytic parameter formals overflow"))?,
        )?;
        let mut formals = BTreeSet::from([parameter.parameter]);
        formals.extend(placements.iter().map(|row| row.occurrence));
        let count = formals
            .iter()
            .try_fold(0usize, |total, formal| {
                total.checked_add(self.parameter_declarations.get(formal).map_or(0, Vec::len))
            })
            .ok_or_else(|| invalid("analytic parameter declarations overflow"))?;
        let declarations = selected_ids(
            formals
                .iter()
                .flat_map(|formal| {
                    self.parameter_declarations
                        .get(formal)
                        .into_iter()
                        .flatten()
                })
                .map(|row| row.id()),
            count,
            &mut scratch,
        )?;
        scratch.grow(
            declarations
                .len()
                .checked_mul(128)
                .ok_or_else(|| invalid("analytic parameter symbols overflow"))?,
        )?;
        let symbols = declarations
            .iter()
            .filter_map(|id| d.native.parameter_declarations.get(*id))
            .filter_map(|row| d.native.parameters.get(row.parameter))
            .filter_map(|row| d.native.signatures.get(row.signature))
            .map(|row| row.symbol)
            .collect::<BTreeSet<_>>();
        let count = symbols
            .iter()
            .try_fold(0usize, |total, symbol| {
                total.checked_add(self.entity_declarations.get(symbol).map_or(0, Vec::len))
            })
            .ok_or_else(|| invalid("analytic function declarations overflow"))?;
        let entity_declarations = selected_ids(
            symbols
                .iter()
                .flat_map(|symbol| self.entity_declarations.get(symbol).into_iter().flatten())
                .map(|row| row.id()),
            count,
            &mut scratch,
        )?;
        let mut view = d.native.view();
        view.placements = normalized::RowsView::selected(&d.native.placements, &placement_ids)?;
        view.parameter_declarations =
            normalized::RowsView::selected(&d.native.parameter_declarations, &declarations)?;
        view.entity_declarations =
            normalized::RowsView::selected(&d.native.entity_declarations, &entity_declarations)?;
        let value =
            normalized::parameter_correspondence::source_parameter_view(&view, parameter, ctx, b)?
                .map(Rc::new);
        cache.insert(key, value.clone());
        Ok(value)
    }
    fn receiver(
        &self,
        d: &Data,
        p: &normalized::parameter_correspondence::SourceParameterCorrespondence,
        callable: Id<CallableEntity>,
        ctx: Id<attribution::AnalysisContext>,
        b: &ResourceBudget,
    ) -> Result<bool, ModelError> {
        let mut charge = charged::StateCharge::new(b, "analytic-receiver-selection");
        let variants = self
            .variants
            .get(&(callable, ctx))
            .map_or(&[][..], Vec::as_slice);
        let ids = selected_ids(variants.iter().map(|r| r.id()), variants.len(), &mut charge)?;
        let count = variants
            .iter()
            .try_fold(0usize, |total, variant| {
                total.checked_add(self.variant_slots.get(&variant.id()).map_or(0, Vec::len))
            })
            .ok_or_else(|| invalid("analytic receiver slots overflow"))?;
        let slots = selected_ids(
            variants
                .iter()
                .flat_map(|variant| self.variant_slots.get(&variant.id()).into_iter().flatten())
                .map(|row| row.id()),
            count,
            &mut charge,
        )?;
        let mut view = d.native.view();
        view.callable_variants = normalized::RowsView::selected(&d.native.callable_variants, &ids)?;
        view.callable_slots = normalized::RowsView::selected(&d.native.callable_slots, &slots)?;
        Ok(normalized::parameter_correspondence::is_bound_receiver_view(&view, p, callable, ctx))
    }
}
fn declaration(
    d: &Data,
    entity: Id<EntityRef>,
) -> Result<Option<Id<source::Occurrence>>, ModelError> {
    let EntityRef::Callable { callable } = need(&d.native.refs, entity)? else {
        return Ok(None);
    };
    Ok(match need(&d.native.callables, *callable)? {
        CallableEntity::Source { declaration, .. } => Some(*declaration),
        _ => None,
    })
}
fn context(
    d: &Data,
    q: Id<assertion::AssertionQualification>,
    ctx: Id<attribution::AnalysisContext>,
) -> Result<bool, ModelError> {
    Ok(need(&d.native.qualifications, q)?.context == ctx)
}
fn receiver(
    d: &Data,
    prep: &Correspondence<'_>,
    entity: Id<EntityRef>,
    p: &normalized::parameter_correspondence::SourceParameterCorrespondence,
    ctx: Id<attribution::AnalysisContext>,
    b: &ResourceBudget,
) -> Result<bool, ModelError> {
    let Some(EntityRef::Callable { callable }) = d.native.refs.get(entity) else {
        return Ok(false);
    };
    prep.receiver(d, p, *callable, ctx, b)
}
/// Unknown/truncated/recursive aliases are excluded explicitly. The traversal follows modeled
/// structure, never type rendering, with bounded native lists and per-branch cycle detection.
#[allow(
    clippy::too_many_arguments,
    reason = "bounded recursion retains exact context, path and work state separately from the immutable correspondence"
)]
fn types(
    d: &Data,
    prep: &Correspondence<'_>,
    term: Id<types::TypeTerm>,
    ctx: Id<attribution::AnalysisContext>,
    classes: &mut BTreeSet<Id<EntityRef>>,
    path: &mut BTreeSet<Id<types::TypeTerm>>,
    work: &mut usize,
) -> Result<bool, ModelError> {
    *work += 1;
    if *work > 32768 || path.len() >= 256 {
        return Ok(false);
    }
    if !path.insert(term) {
        return Ok(false);
    }
    let mut children = vec![];
    let mut lists = vec![];
    let mut symbol = None;
    let mut parameters = None;
    let known = match need(&d.native.terms, term)? {
        types::TypeTerm::ClassInstance { class, arguments }
        | types::TypeTerm::SelfType { class, arguments }
        | types::TypeTerm::TypedDict {
            class, arguments, ..
        } => {
            symbol = Some(*class);
            lists.push(*arguments);
            true
        }
        types::TypeTerm::ClassObject { class } | types::TypeTerm::EnumLiteral { class, .. } => {
            symbol = Some(*class);
            true
        }
        types::TypeTerm::Union { members }
        | types::TypeTerm::Intersection { members }
        | types::TypeTerm::Overload {
            signatures: members,
            ..
        }
        | types::TypeTerm::Tuple { elements: members } => {
            lists.push(*members);
            true
        }
        types::TypeTerm::TypeOf { target }
        | types::TypeTerm::Annotated { target }
        | types::TypeTerm::Unpack { target }
        | types::TypeTerm::TypeGuard { target, .. }
        | types::TypeTerm::TypeForm { target } => {
            children.push(*target);
            true
        }
        types::TypeTerm::TypeAlias {
            target, untyped, ..
        } => {
            children.push(*target);
            !*untyped
        }
        types::TypeTerm::BoundMethod { receiver, function } => {
            children.extend([*receiver, *function]);
            true
        }
        types::TypeTerm::Generic { parameters, body } => {
            lists.push(*parameters);
            children.push(*body);
            true
        }
        types::TypeTerm::Callable {
            parameters: p,
            returns,
            param_spec,
            ..
        } => {
            parameters = Some(*p);
            children.push(*returns);
            children.extend(*param_spec);
            true
        }
        types::TypeTerm::ParamList {
            parameters: p,
            param_spec,
        } => {
            parameters = Some(*p);
            children.extend(*param_spec);
            true
        }
        types::TypeTerm::Literal { .. }
        | types::TypeTerm::None
        | types::TypeTerm::Never { .. }
        | types::TypeTerm::LiteralString
        | types::TypeTerm::SpecialForm { .. }
        | types::TypeTerm::Module { .. }
        | types::TypeTerm::TypeVar { .. }
        | types::TypeTerm::ParamSpec { .. }
        | types::TypeTerm::TypeVarTuple { .. }
        | types::TypeTerm::VariableForm { .. }
        | types::TypeTerm::Any {
            flavor: types::AnyFlavor::Explicit,
        } => true,
        _ => false,
    };
    if let Some(symbol) = symbol {
        for r in prep
            .resolutions
            .get(&(symbol, ctx))
            .into_iter()
            .flatten()
            .filter(|r| r.status == ResolutionStatus::Resolved)
        {
            if let Some(entity) = r.entity
                && matches!(d.native.refs.get(entity), Some(EntityRef::Class { .. }))
            {
                classes.insert(entity);
            }
        }
    }
    for sequence in lists {
        need(&d.type_sequences, sequence)?;
        for m in prep.members.get(&sequence).into_iter().flatten() {
            children.push(m.child);
        }
    }
    if let Some(list) = parameters {
        for p in prep.slots.get(&list).into_iter().flatten() {
            children.push(p.term);
        }
    }
    let mut complete = known;
    for child in children {
        complete &= types(d, prep, child, ctx, classes, path, work)?;
    }
    path.remove(&term);
    Ok(complete)
}
pub(super) struct ClassSet {
    values: BTreeSet<Id<EntityRef>>,
    _charge: charged::StateCharge,
}
impl std::ops::Deref for ClassSet {
    type Target = BTreeSet<Id<EntityRef>>;
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
pub(super) fn classes(
    d: &Data,
    prep: &Correspondence<'_>,
    term: Id<types::TypeTerm>,
    ctx: Id<attribution::AnalysisContext>,
    b: &ResourceBudget,
) -> Result<Option<Rc<ClassSet>>, ModelError> {
    let mut cache = prep.class_cache.borrow_mut();
    let key = (term, ctx);
    if let Some(value) = cache.get(&key) {
        return Ok(value.clone());
    }
    prep._charge.borrow_mut().grow(512)?;
    let _scratch = b.reserve(
        "analytic-type-structure",
        d.native
            .terms
            .len()
            .checked_mul(512)
            .and_then(|n| n.checked_add(32768 * 128))
            .ok_or_else(|| invalid("analytic type structure overflow"))?,
    )?;
    let mut values = BTreeSet::new();
    let complete = types(
        d,
        prep,
        term,
        ctx,
        &mut values,
        &mut BTreeSet::new(),
        &mut 0,
    )?;
    let value = if complete {
        let mut charge = charged::StateCharge::new(b, "analytic-class-selection");
        charge.grow(
            values
                .len()
                .checked_mul(256)
                .and_then(|n| n.checked_add(128))
                .ok_or_else(|| invalid("analytic classes overflow"))?,
        )?;
        Some(Rc::new(ClassSet {
            values,
            _charge: charge,
        }))
    } else {
        None
    };
    cache.insert(key, value.clone());
    Ok(value)
}
pub(super) fn type_layer(
    d: &Data,
    prep: &Correspondence<'_>,
    f: &AnalyticFrame,
    scope: &BTreeSet<Id<EntityRef>>,
    out: &mut Output,
    b: &ResourceBudget,
    layer_work: u64,
) -> Result<Option<u64>, ModelError> {
    let sf = need(&d.structural.frames, f.structural)?;
    let parent = need(&d.structural_invocations, sf.invocation)?;
    let mut members =
        BTreeMap::<Id<EntityRef>, BTreeMap<Id<EntityRef>, Id<types::TypeObservation>>>::new();
    let mut charge = charged::StateCharge::new(b, "analytic-type-layer");
    for entity in scope {
        let Some(declaration) = declaration(d, *entity)? else {
            continue;
        };
        for p in prep.parameters.get(&declaration).into_iter().flatten() {
            let Some(correspondence) = prep.parameter(d, p, parent.context, b)? else {
                continue;
            };
            if receiver(d, prep, *entity, &correspondence, parent.context, b)? {
                continue;
            }
            for observation in prep
                .observations
                .get(&correspondence.formal)
                .into_iter()
                .flatten()
                .filter(|o| o.role == types::TypeRole::Parameter && o.declared)
            {
                if !context(d, observation.qualification, parent.context)? {
                    continue;
                }
                let Some(classes) = classes(d, prep, observation.term, parent.context, b)? else {
                    continue;
                };
                for class in classes.iter().copied() {
                    let EntityRef::Class { class: class_id } = need(&d.native.refs, class)? else {
                        continue;
                    };
                    let ClassEntity::Source { declaration } =
                        need(&d.native.entity_classes, *class_id)?
                    else {
                        continue;
                    };
                    let source = need(&d.native.occurrences, *declaration)?.source;
                    let input = need(&d.native.artifacts, source)?.input;
                    if input != parent.input
                        || !prep.roles.contains(&(
                            parent.input,
                            source,
                            input::SourceRole::Release as i16,
                        ))
                    {
                        continue;
                    }
                    charge.grow(256)?;
                    members
                        .entry(class)
                        .or_default()
                        .entry(*entity)
                        .or_insert(observation.id());
                }
            }
        }
    }
    let Some(work) = build::clique_work(members.values().map(BTreeMap::len), layer_work) else {
        return Ok(None);
    };
    for (class, rows) in members {
        let rows = rows.into_iter().collect::<Vec<_>>();
        for (i, (left, l)) in rows.iter().enumerate() {
            for (right, r) in &rows[i + 1..] {
                build::add_pair(
                    out,
                    f.id(),
                    Layer::Type,
                    *left,
                    *right,
                    PairSource::Type {
                        left: *l,
                        right: *r,
                        class,
                    },
                )?;
            }
        }
    }
    Ok(Some(work))
}
/// Standalone finite layer helper; the production invocation publishes explicit layer outcomes.
pub fn mention_layer(
    d: &Data,
    f: &AnalyticFrame,
    scope: &BTreeSet<Id<EntityRef>>,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let prep = Correspondence::new(d, b, false, false, true)?;
    mention_layer_prepared(d, &prep, f, scope, out, b, build::MAX_WORK)?
        .ok_or_else(|| invalid("analytic mention layer pair bound"))?;
    Ok(())
}
pub(super) fn mention_layer_prepared(
    d: &Data,
    prep: &Correspondence<'_>,
    f: &AnalyticFrame,
    scope: &BTreeSet<Id<EntityRef>>,
    out: &mut Output,
    b: &ResourceBudget,
    layer_work: u64,
) -> Result<Option<u64>, ModelError> {
    let sf = need(&d.structural.frames, f.structural)?;
    let parent = need(&d.structural_invocations, sf.invocation)?;
    let mut charge = charged::StateCharge::new(b, "analytic-mention-layer");
    let mut passages = BTreeMap::<
        Id<documents::DocumentNode>,
        BTreeMap<Id<EntityRef>, Id<documents::DocumentMentionObservation>>,
    >::new();
    for a in d
        .native
        .relation_mention_entity_assessments
        .iter()
        .filter(|a| a.status == ResolutionStatus::Resolved)
    {
        let mention = need(&d.native.mentions, a.observation)?;
        if mention.class != documents::MentionClass::Exact {
            continue;
        }
        let q = need(&d.native.qualifications, mention.qualification)?;
        let artifact = match need(&d.native.scopes, q.scope)? {
            source::CoverageScope::Artifact { artifact } => Some(*artifact),
            source::CoverageScope::Module { module } => {
                Some(need(&d.native.modules, *module)?.source)
            }
            _ => None,
        };
        let Some(artifact) = artifact else {
            continue;
        };
        let document_input = need(&d.native.artifacts, artifact)?.input;
        let admitted_input = if document_input == parent.input {
            q.context == parent.context
        } else {
            prep.corpus.contains(&(document_input, parent.input))
        };
        // A corpus link admits documentary correspondence, not native context equality.
        // The candidate resolution below must still belong to this exact analytic context.
        if !admitted_input
            || !prep
                .roles
                .contains(&(document_input, artifact, input::SourceRole::Document as i16))
        {
            continue;
        }
        for c in prep.mention_candidates.get(&a.id()).into_iter().flatten() {
            for ec in prep
                .exposure_candidates
                .get(&c.exposure)
                .into_iter()
                .flatten()
            {
                let resolution = need(&d.native.symbol_resolutions, ec.resolution)?;
                if resolution.context != parent.context
                    || resolution.status != ResolutionStatus::Resolved
                {
                    continue;
                }
                let Some(entity) = resolution.entity else {
                    continue;
                };
                if !scope.contains(&entity) {
                    continue;
                }
                charge.grow(256)?;
                passages
                    .entry(mention.passage.id())
                    .or_default()
                    .entry(entity)
                    .or_insert(mention.id());
            }
        }
    }
    let Some(work) = build::clique_work(passages.values().map(BTreeMap::len), layer_work) else {
        return Ok(None);
    };
    for rows in passages.into_values() {
        let rows = rows.into_iter().collect::<Vec<_>>();
        for (i, (left, l)) in rows.iter().enumerate() {
            for (right, r) in &rows[i + 1..] {
                build::add_pair(
                    out,
                    f.id(),
                    Layer::Mention,
                    *left,
                    *right,
                    PairSource::Mention {
                        left: *l,
                        right: *r,
                    },
                )?;
            }
        }
    }
    Ok(Some(work))
}
pub(super) struct Fact {
    pub entity: Id<EntityRef>,
    pub attribute: Attribute,
    pub source: IncidenceSource,
}
impl HeapSize for Fact {
    fn heap_bytes(&self) -> usize {
        self.attribute
            .heap_bytes()
            .saturating_add(self.source.heap_bytes())
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "method-local incidence preserves exact object universe, context, policy and independent retained owners"
)]
fn facts(
    d: &Data,
    prep: &Correspondence<'_>,
    objects: &BTreeSet<Id<EntityRef>>,
    ctx: Id<attribution::AnalysisContext>,
    rca: bool,
    out: &Output,
    b: &ResourceBudget,
    policy: policy::AttributePolicy,
) -> Result<
    (
        charged::ChargedVec<Fact>,
        charged::ChargedVec<super::native_attributes::Selection>,
        charged::StateCharge,
    ),
    ModelError,
> {
    let mut facts = charged::ChargedVec::default();
    let mut selections = charged::ChargedVec::default();
    let mut charge = charged::StateCharge::new(b, "analytic-concept-incidence");
    for entity in objects {
        let Some(declaration) = declaration(d, *entity)? else {
            continue;
        };
        super::native_attributes::variants(
            d,
            prep,
            *entity,
            ctx,
            b,
            &mut facts,
            &mut charge,
            policy,
        )?;
        super::native_attributes::dependence(d, prep, *entity, ctx, &mut facts, &mut charge)?;
        for p in prep
            .parameters
            .get(&declaration)
            .into_iter()
            .flatten()
            .filter(|_| policy.signature_roles.source())
        {
            let Some(correspondence) = prep.parameter(d, p, ctx, b)? else {
                continue;
            };
            if policy.receiver == policy::ReceiverPolicy::ExcludeSourceBoundPreserveNative
                && receiver(d, prep, *entity, &correspondence, ctx, b)?
            {
                continue;
            }
            let shapes = correspondence
                .parameters
                .iter()
                .filter_map(|id| d.native.parameters.get(*id))
                .filter_map(|p| d.native.shapes.get(p.shape));
            for shape in shapes {
                if let Some(name) = &shape.name {
                    facts.push(
                        &mut charge,
                        Fact {
                            entity: *entity,
                            attribute: Attribute::Parameter {
                                name: name.clone(),
                                kind: shape.kind,
                            },
                            source: IncidenceSource::Parameter {
                                observation: p.id(),
                            },
                        },
                    )?;
                }
            }
            for o in prep
                .observations
                .get(&correspondence.formal)
                .into_iter()
                .flatten()
                .filter(|o| o.role == types::TypeRole::Parameter && o.declared)
            {
                if context(d, o.qualification, ctx)? && classes(d, prep, o.term, ctx, b)?.is_some()
                {
                    facts.push(
                        &mut charge,
                        Fact {
                            entity: *entity,
                            attribute: Attribute::ParameterType { term: o.term },
                            source: IncidenceSource::Type {
                                observation: o.id(),
                            },
                        },
                    )?;
                    super::native_attributes::metadata(
                        d,
                        prep,
                        *entity,
                        o,
                        TypePortRole::Parameter,
                        ctx,
                        b,
                        &mut facts,
                        &mut selections,
                        &mut charge,
                    )?;
                }
            }
        }
        let mut selected = charged::StateCharge::new(b, "analytic-object-observations");
        let mut subjects = vec![declaration];
        if let Some(owners) = prep.owners.get(entity) {
            selected.grow(
                owners
                    .len()
                    .checked_mul(std::mem::size_of::<Id<source::Occurrence>>())
                    .ok_or_else(|| invalid("analytic ownership selection overflow"))?,
            )?;
            subjects.extend(owners.iter().map(|o| o.occurrence));
        }
        for o in prep.candidates(&prep.observations, subjects.into_iter(), &mut selected)? {
            if !context(d, o.qualification, ctx)? {
                continue;
            }
            let attribute = if policy.signature_roles.source()
                && o.subject == declaration
                && o.role == types::TypeRole::Return
                && o.declared
                && classes(d, prep, o.term, ctx, b)?.is_some()
            {
                Some(Attribute::Returns { term: o.term })
            } else if o.role == types::TypeRole::Raised
                && prep
                    .owners
                    .get(entity)
                    .into_iter()
                    .flatten()
                    .any(|owner| owner.occurrence == o.subject)
            {
                let values = classes(d, prep, o.term, ctx, b)?;
                values.and_then(|c| {
                    if c.len() == 1 {
                        c.iter()
                            .next()
                            .copied()
                            .map(|class| Attribute::Raises { class })
                    } else {
                        None
                    }
                })
            } else {
                None
            };
            if let Some(attribute) = attribute {
                let returns = matches!(&attribute, Attribute::Returns { .. });
                facts.push(
                    &mut charge,
                    Fact {
                        entity: *entity,
                        attribute,
                        source: IncidenceSource::Type {
                            observation: o.id(),
                        },
                    },
                )?;
                if returns {
                    super::native_attributes::metadata(
                        d,
                        prep,
                        *entity,
                        o,
                        TypePortRole::Return,
                        ctx,
                        b,
                        &mut facts,
                        &mut selections,
                        &mut charge,
                    )?;
                }
            }
        }
        for dec in prep.decorators.get(&declaration).into_iter().flatten() {
            if !context(d, dec.qualification, ctx)? {
                continue;
            }
            super::native_attributes::decorator(
                d,
                prep,
                *entity,
                dec,
                ctx,
                &mut facts,
                &mut selections,
                &mut charge,
            )?;
        }
    }
    if rca {
        relational_facts(d, objects, out, &mut facts, &mut charge, b)?;
    }
    Ok((facts, selections, charge))
}
fn relational_facts(
    d: &Data,
    objects: &BTreeSet<Id<EntityRef>>,
    out: &Output,
    facts: &mut charged::ChargedVec<Fact>,
    charge: &mut charged::StateCharge,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut scope_charge = charged::StateCharge::new(b, "analytic-relational-scope-membership");
    let mut scope_members = BTreeSet::new();
    let mut frames = BTreeSet::new();
    for frame in out.frames.iter() {
        if !frames.contains(&frame.structural) {
            scope_charge.grow(128 + std::mem::size_of_val(&frame.structural))?;
            frames.insert(frame.structural);
        }
    }
    for member in d
        .structural
        .scope
        .iter()
        .filter(|member| frames.contains(&member.frame))
    {
        let key = (member.frame, member.entity);
        if !scope_members.contains(&key) {
            scope_charge.grow(128 + std::mem::size_of_val(&key))?;
            scope_members.insert(key);
        }
    }
    for arc in out.arcs.iter().filter(|a| {
        objects.contains(&a.source)
            && a.source != a.target
            && scope_members.contains(&(need(&out.frames, a.frame).unwrap().structural, a.target))
    }) {
        let alternative = need(&d.native.event_alternatives, arc.alternative)?;
        facts.push(
            charge,
            Fact {
                entity: arc.source,
                attribute: Attribute::Calls {
                    target: arc.target,
                    modality: need(
                        &d.native.qualifications,
                        need(
                            &d.native.targets,
                            need(&d.native.event_alternative_sources, alternative.source)?.target(),
                        )?
                        .qualification,
                    )?
                    .modality,
                    phase: need(
                        &d.native.targets,
                        need(&d.native.event_alternative_sources, alternative.source)?.target(),
                    )?
                    .phase,
                },
                source: IncidenceSource::Call { arc: arc.id() },
            },
        )?;
    }
    for h in d
        .structural
        .handoffs
        .iter()
        .filter(|h| frames.contains(&h.frame))
    {
        let producer = need(&d.native.event_alternatives, h.producer)?;
        let consumer = need(&d.native.event_alternatives, h.consumer)?;
        let (Some(source), Some(target)) = (producer.entity, consumer.entity) else {
            return Err(invalid("retained handoff endpoints absent"));
        };
        if source == target {
            continue;
        }
        let p = need(
            &d.native.targets,
            need(&d.native.event_alternative_sources, producer.source)?.target(),
        )?;
        let c = need(
            &d.native.targets,
            need(&d.native.event_alternative_sources, consumer.source)?.target(),
        )?;
        for (entity, target, takes) in [(source, target, false), (target, source, true)] {
            if objects.contains(&entity) {
                facts.push(
                    charge,
                    Fact {
                        entity,
                        attribute: Attribute::Handoff {
                            takes,
                            target,
                            producer_modality: need(&d.native.qualifications, p.qualification)?
                                .modality,
                            consumer_modality: need(&d.native.qualifications, c.qualification)?
                                .modality,
                            producer_phase: p.phase,
                            consumer_phase: c.phase,
                        },
                        source: IncidenceSource::Handoff { occurrence: h.id() },
                    },
                )?;
            }
        }
    }
    Ok(())
}
struct NamespaceGroup {
    access: Id<source::Module>,
    namespace: Vec<String>,
    candidates: Vec<(Id<EntityRef>, Id<structural::PublicCandidate>)>,
    objects: BTreeSet<Id<EntityRef>>,
    facts: Vec<usize>,
    selections: Vec<usize>,
    relational: Vec<usize>,
}
pub(super) struct ConceptPreparation {
    groups: Vec<NamespaceGroup>,
    facts: charged::ChargedVec<Fact>,
    selections: charged::ChargedVec<super::native_attributes::Selection>,
    relational: charged::ChargedVec<Fact>,
    _facts_charge: charged::StateCharge,
    _relational_charge: charged::StateCharge,
    _charge: charged::StateCharge,
}
impl ConceptPreparation {
    #[expect(
        clippy::too_many_arguments,
        reason = "one exact shared base retains selected frame/domain/context/policy and independent relational demand"
    )]
    pub(super) fn new(
        d: &Data,
        prep: &Correspondence<'_>,
        sf: &structural::StructuralFrame,
        public: &BTreeSet<Id<EntityRef>>,
        out: &Output,
        b: &ResourceBudget,
        policy: policy::AttributePolicy,
        rca: bool,
    ) -> Result<Self, ModelError> {
        let mut charge = charged::StateCharge::new(b, "analytic-concept-scopes");
        let mut groups = BTreeMap::<
            (Id<source::Module>, Vec<String>),
            Vec<(Id<EntityRef>, Id<structural::PublicCandidate>)>,
        >::new();
        for candidate in d
            .structural
            .public
            .iter()
            .filter(|c| c.frame == sf.id() && public.contains(&c.entity))
        {
            let member = need(&d.members, candidate.member)?;
            charge.grow(512 + member.path.heap_bytes())?;
            let mut namespace = member.path.clone();
            namespace.pop();
            groups
                .entry((member.access, namespace))
                .or_default()
                .push((candidate.entity, candidate.id()));
        }
        charge.grow(
            public
                .len()
                .checked_mul(128)
                .ok_or_else(|| invalid("concept selected objects overflow"))?,
        )?;
        let selected_objects = groups
            .values()
            .flat_map(|c| c.iter().map(|(e, _)| *e))
            .collect::<BTreeSet<_>>();
        let parent = need(&d.structural_invocations, sf.invocation)?;
        let (facts, selections, facts_charge) = if groups.is_empty() {
            (
                charged::ChargedVec::default(),
                charged::ChargedVec::default(),
                charged::StateCharge::new(b, "analytic-concept-incidence"),
            )
        } else {
            facts(
                d,
                prep,
                &selected_objects,
                parent.context,
                false,
                out,
                b,
                policy,
            )?
        };
        let mut relational = charged::ChargedVec::default();
        let mut relational_charge = charged::StateCharge::new(b, "analytic-relational-incidence");
        if rca && !groups.is_empty() {
            relational_facts(
                d,
                &selected_objects,
                out,
                &mut relational,
                &mut relational_charge,
                b,
            )?;
        }
        let mut groups = groups
            .into_iter()
            .map(|((access, namespace), candidates)| {
                let objects = candidates.iter().map(|(e, _)| *e).collect();
                NamespaceGroup {
                    access,
                    namespace,
                    candidates,
                    objects,
                    facts: vec![],
                    selections: vec![],
                    relational: vec![],
                }
            })
            .collect::<Vec<_>>();
        // One object-to-namespace map distributes each evidence position once; mathematical
        // entity/attribute deduplication is deferred to the individual context, never provenance.
        let mut destinations = BTreeMap::<Id<EntityRef>, Vec<usize>>::new();
        for (index, group) in groups.iter().enumerate() {
            for entity in &group.objects {
                charge.grow(192)?;
                destinations.entry(*entity).or_default().push(index);
            }
        }
        for (index, fact) in facts.iter().enumerate() {
            for group in destinations.get(&fact.entity).into_iter().flatten() {
                charge.grow(2 * size_of::<usize>())?;
                groups[*group].facts.push(index);
            }
        }
        for (index, selection) in selections.iter().enumerate() {
            for group in destinations.get(&selection.entity()).into_iter().flatten() {
                charge.grow(2 * size_of::<usize>())?;
                groups[*group].selections.push(index);
            }
        }
        for (index, fact) in relational.iter().enumerate() {
            for group in destinations.get(&fact.entity).into_iter().flatten() {
                charge.grow(2 * size_of::<usize>())?;
                groups[*group].relational.push(index);
            }
        }
        Ok(Self {
            groups,
            facts,
            selections,
            relational,
            _facts_charge: facts_charge,
            _relational_charge: relational_charge,
            _charge: charge,
        })
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "scoped concept publication retains exact invocation, object selectors and attribute policy"
)]
#[cfg(test)]
pub(super) fn concepts(
    d: &Data,
    prep: &Correspondence<'_>,
    sf: &structural::StructuralFrame,
    public: &BTreeSet<Id<EntityRef>>,
    result: &mut TechniqueResult,
    out: &mut Output,
    b: &ResourceBudget,
    policy: policy::AttributePolicy,
) -> Result<(), ModelError> {
    let prepared = ConceptPreparation::new(
        d,
        prep,
        sf,
        public,
        out,
        b,
        policy,
        result.method == analysis::AnalysisMethod::RelationalConcepts,
    )?;
    concepts_prepared(&prepared, result, out, b)
}
pub(super) fn concepts_prepared(
    prepared: &ConceptPreparation,
    result: &mut TechniqueResult,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    if prepared.groups.is_empty() {
        result.stop = Stop::EmptyDomain;
        return Ok(());
    }
    let rca = result.method == analysis::AnalysisMethod::RelationalConcepts;
    for group in &prepared.groups {
        let objects = &group.objects;
        let facts = || {
            group.facts.iter().map(|i| &prepared.facts[*i]).chain(
                group
                    .relational
                    .iter()
                    .filter(move |_| rca)
                    .map(|i| &prepared.relational[*i]),
            )
        };
        let _context = b.reserve(
            "analytic-concept-conversion",
            (group.facts.len() + if rca { group.relational.len() } else { 0 })
                .checked_mul(768)
                .ok_or_else(|| invalid("concept conversion overflow"))?,
        )?;
        let mut attributes = BTreeSet::new();
        let mut incidences = BTreeSet::new();
        for fact in facts() {
            attributes.insert(fact.attribute.id());
            incidences.insert((fact.entity, fact.attribute.id()));
        }
        let object_list = objects.iter().copied().collect::<Vec<_>>();
        let attribute_list = attributes.into_iter().collect::<Vec<_>>();
        let incidence_list = incidences.into_iter().collect::<Vec<_>>();
        let context =
            super::concepts::Context::new(&object_list, &attribute_list, &incidence_list, b)?;
        let lattice = context.analyse(
            super::policy::RETAINED.concept_support,
            super::policy::RETAINED.concept_bound,
        )?;
        let scope = ConceptScope {
            result: result.id(),
            access: group.access,
            namespace: group.namespace.clone(),
            examined: lattice.examined() as i64,
            partial: lattice.budget_reached(),
        };
        for selection in group.selections.iter().map(|i| &prepared.selections[*i]) {
            selection.emit(scope.id(), out)?;
        }
        for (entity, candidate) in &group.candidates {
            out.objects.insert(ConceptObject {
                scope: scope.id(),
                entity: *entity,
                candidate: *candidate,
            })?;
        }
        for fact in facts() {
            out.attributes.insert(fact.attribute.clone())?;
            out.incidence_sources.insert(fact.source.clone())?;
            out.incidences.insert(Incidence {
                scope: scope.id(),
                entity: fact.entity,
                attribute: fact.attribute.id(),
                source: fact.source.id(),
            })?;
        }
        for c in lattice.concepts() {
            let row = Concept {
                scope: scope.id(),
                extent: build::digest("analytic-extent", &c.extent),
                intent: build::digest("analytic-intent", &c.intent),
            };
            for entity in &c.extent {
                out.extents.insert(ConceptExtent {
                    concept: row.id(),
                    entity: *entity,
                })?;
            }
            for attribute in &c.intent {
                out.intents.insert(ConceptIntent {
                    concept: row.id(),
                    attribute: *attribute,
                })?;
            }
            out.concepts.insert(row)?;
        }
        for i in lattice.implications() {
            let row = Implication {
                scope: scope.id(),
                premise: build::digest("analytic-premise", &i.premise),
                conclusion: build::digest("analytic-conclusion", &i.conclusion),
                support: i.support as i64,
            };
            for attribute in &i.premise {
                out.implication_members.insert(ImplicationMember {
                    implication: row.id(),
                    premise: true,
                    attribute: *attribute,
                })?;
            }
            for attribute in &i.conclusion {
                out.implication_members.insert(ImplicationMember {
                    implication: row.id(),
                    premise: false,
                    attribute: *attribute,
                })?;
            }
            out.implications.insert(row)?;
        }
        result.examined += scope.examined;
        if scope.partial {
            result.status = analysis::AnalysisStatus::Partial;
            result.stop = Stop::EnumerationLimit;
        }
        out.scopes.insert(scope)?;
    }
    Ok(())
}

#[cfg(test)]
mod correspondence_tests {
    use super::*;
    fn id<T>(n: u8) -> Id<T> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    #[test]
    fn indexed_types_preserve_plural_contexts_and_lazy_borrower_reservations() {
        let b = ResourceBudget::fixed(16 << 20).unwrap();
        let mut d = Data::new(&b);
        let first = d
            .native
            .refs
            .insert(EntityRef::Class { class: id(1) })
            .unwrap();
        let second = d
            .native
            .refs
            .insert(EntityRef::Class { class: id(2) })
            .unwrap();
        let foreign = d
            .native
            .refs
            .insert(EntityRef::Class { class: id(3) })
            .unwrap();
        for (context, entity, policy) in [
            (id(10), first, b"first".as_slice()),
            (id(10), second, b"second".as_slice()),
            (id(11), foreign, b"foreign".as_slice()),
        ] {
            d.native
                .symbol_resolutions
                .insert(SymbolEntityResolution {
                    symbol: id(4),
                    context,
                    policy: ContentHash::of(policy),
                    status: ResolutionStatus::Resolved,
                    entity: Some(entity),
                    reason: EntityReason::DeclarationAgreement,
                })
                .unwrap();
        }
        let term = d
            .native
            .terms
            .insert(types::TypeTerm::ClassObject { class: id(4) })
            .unwrap();
        let baseline = b.reserved();
        let prep = Correspondence::new(&d, &b, true, false, false).unwrap();
        let current = classes(&d, &prep, term, id(10), &b).unwrap().unwrap();
        assert_eq!(current.values, BTreeSet::from([first, second]));
        let reserved = b.reserved();
        let again = classes(&d, &prep, term, id(10), &b).unwrap().unwrap();
        assert!(Rc::ptr_eq(&current, &again));
        assert_eq!(b.reserved(), reserved);
        assert_eq!(
            classes(&d, &prep, term, id(11), &b)
                .unwrap()
                .unwrap()
                .values,
            BTreeSet::from([foreign])
        );
        assert_eq!(
            classes(&d, &prep, term, id(12), &b)
                .unwrap()
                .unwrap()
                .values,
            BTreeSet::new()
        );
        // An unconsumed malformed term identity is not inspected while correspondence is built.
        assert!(classes(&d, &prep, id(99), id(10), &b).is_err());
        drop(again);
        drop(prep);
        assert!(b.reserved() > baseline); // actual class borrower retains its backing reservation
        drop(current);
        assert_eq!(b.reserved(), baseline);
        drop(d);
        assert_eq!(b.reserved(), 0);
    }
    fn scopes(b: &ResourceBudget) -> (Data, structural::StructuralFrame, Output, TechniqueResult) {
        let mut d = Data::new(b);
        let (parent, _) = analysis::structural::Invocation::new(id(1), id(2), id(3), None, []);
        d.structural_invocations.insert(parent.clone()).unwrap();
        let sf = structural::StructuralFrame {
            invocation: parent.id(),
            usage_invocation: parent.id(),
            handoff_invocation: parent.id(),
            control_invocation: parent.id(),
            controls_requested: false,
            configuration: id(4),
            invocation_graph: id(5),
            definition_graph: id(6),
        };
        d.structural.frames.insert(sf.clone()).unwrap();
        let f = AnalyticFrame {
            structural: sf.id(),
            configuration: id(4),
        };
        let mut out = Output::new(b);
        out.frames.insert(f.clone()).unwrap();
        let result = TechniqueResult {
            frame: f.id(),
            method: analysis::AnalysisMethod::Concepts,
            invocation: id(7),
            selected: true,
            status: analysis::AnalysisStatus::Completed,
            stop: Stop::Converged,
            iterations: 0,
            examined: 0,
            residual: None,
            input_partial: false,
        };
        (d, sf, out, result)
    }
    #[test]
    fn shared_native_facts_keep_each_namespace_object_universe_and_candidate_provenance() {
        let b = ResourceBudget::fixed(16 << 20).unwrap();
        let (mut d, sf, mut out, mut result) = scopes(&b);
        let q = assertion::AssertionQualification {
            context: id(2),
            scope: id(8),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        d.native.qualifications.insert(q.clone()).unwrap();
        let term = d.native.terms.insert(types::TypeTerm::None).unwrap();
        let mut entities = vec![];
        for n in [10, 11] {
            let callable = d
                .native
                .callables
                .insert(CallableEntity::Source {
                    declaration: id(n),
                    kind: CallableKind::Function,
                })
                .unwrap();
            let entity = d
                .native
                .refs
                .insert(EntityRef::Callable { callable })
                .unwrap();
            entities.push(entity);
            d.native
                .type_observations
                .insert(types::TypeObservation {
                    qualification: q.id(),
                    subject: id(n),
                    role: types::TypeRole::Return,
                    declared: true,
                    term,
                })
                .unwrap();
        }
        let mut expected = BTreeMap::new();
        for (ordinal, namespace, entity) in [
            (20, "one", entities[0]),
            (21, "one", entities[1]),
            (22, "two", entities[0]),
        ] {
            let member = d
                .members
                .insert(catalog::CatalogMember {
                    input: id(1),
                    access: id(9),
                    path: vec![namespace.into(), format!("f{ordinal}")],
                    name: format!("{namespace}.f{ordinal}"),
                })
                .unwrap();
            let candidate = structural::PublicCandidate {
                frame: sf.id(),
                callable: id(ordinal),
                member,
                entity,
                path: format!("{namespace}.f{ordinal}"),
                configured_ordinal: None,
                in_subsystem: true,
            };
            expected
                .entry(namespace.to_string())
                .or_insert_with(BTreeSet::new)
                .insert((entity, candidate.id()));
            d.structural.public.insert(candidate).unwrap();
        }
        // Irrelevant rows are indexed structurally; their absent premise is never relied on.
        for n in 100..150 {
            d.native
                .type_observations
                .insert(types::TypeObservation {
                    qualification: id(99),
                    subject: id(n),
                    role: types::TypeRole::Raised,
                    declared: true,
                    term: id(99),
                })
                .unwrap();
        }
        let prep = Correspondence::new(&d, &b, true, true, false).unwrap();
        let mut candidate_charge = charged::StateCharge::new(&b, "test-candidates");
        let relevant = prep
            .candidates(
                &prep.observations,
                [id(10), id(11)].into_iter(),
                &mut candidate_charge,
            )
            .unwrap();
        assert_eq!(relevant.len(), 2);
        assert_eq!(d.native.type_observations.len(), 52);
        let prepared = ConceptPreparation::new(
            &d,
            &prep,
            &sf,
            &entities.into_iter().collect(),
            &out,
            &b,
            policy::RETAINED.attributes,
            true,
        )
        .unwrap();
        assert_eq!(prepared.facts.len(), 2);
        assert_eq!(prepared.groups.len(), 2);
        assert_eq!(
            prepared.groups.iter().map(|g| g.facts.len()).sum::<usize>(),
            3
        );
        assert!(prepared.relational.is_empty());
        concepts_prepared(&prepared, &mut result, &mut out, &b).unwrap();
        let mut relational = Output::new(&b);
        let mut relational_result = result.clone();
        relational_result.method = analysis::AnalysisMethod::RelationalConcepts;
        relational_result.examined = 0;
        concepts_prepared(&prepared, &mut relational_result, &mut relational, &b).unwrap();
        assert_eq!(relational.incidences.len(), out.incidences.len());
        assert!(relational.incidence_sources.same(&out.incidence_sources));
        assert_eq!(relational.objects.len(), out.objects.len());
        assert_ne!(
            relational.scopes.iter().next().unwrap().result,
            out.scopes.iter().next().unwrap().result
        );
        assert_eq!(out.scopes.len(), 2);
        assert_eq!(out.objects.len(), 3);
        assert_eq!(out.incidences.len(), 3);
        for scope in out.scopes.iter() {
            let objects = out
                .objects
                .iter()
                .filter(|o| o.scope == scope.id())
                .map(|o| (o.entity, o.candidate))
                .collect::<BTreeSet<_>>();
            assert_eq!(objects, expected[&scope.namespace[0]]);
            let incident = out
                .incidences
                .iter()
                .filter(|i| i.scope == scope.id())
                .map(|i| i.entity)
                .collect::<BTreeSet<_>>();
            assert_eq!(
                incident,
                objects.iter().map(|(entity, _)| *entity).collect()
            );
        }
    }
    #[test]
    fn empty_rca_scope_does_not_inspect_unconsumed_handoffs() {
        let b = ResourceBudget::fixed(1 << 20).unwrap();
        let (mut d, sf, mut out, mut result) = scopes(&b);
        result.method = analysis::AnalysisMethod::RelationalConcepts;
        d.structural
            .handoffs
            .insert(structural::handoffs::Handoff {
                frame: sf.id(),
                consumer: id(80),
                producer: id(81),
                binding: id(82),
                value: id(83),
                role: id(84),
            })
            .unwrap();
        let prep = Correspondence::new(&d, &b, true, true, false).unwrap();
        concepts(
            &d,
            &prep,
            &sf,
            &BTreeSet::new(),
            &mut result,
            &mut out,
            &b,
            policy::RETAINED.attributes,
        )
        .unwrap();
        assert_eq!(result.stop, Stop::EmptyDomain);
        assert!(out.scopes.is_empty());
        assert!(out.incidences.is_empty());
    }
    #[test]
    fn type_groups_preflight_all_classes_before_any_evidence_is_published() {
        let b = ResourceBudget::fixed(16 << 20).unwrap();
        let (mut d, sf, _, _) = scopes(&b);
        let artifact =
            source::SourceArtifact::from_bytes(id(1), "api.py".into(), &[b' '; 128]).unwrap();
        d.native.artifacts.insert(artifact.clone()).unwrap();
        d.uses
            .insert(input::ArtifactUse {
                artifact: artifact.id(),
                input: id(1),
                role: input::SourceRole::Release,
            })
            .unwrap();
        let q = assertion::AssertionQualification {
            context: id(2),
            scope: id(8),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        d.native.qualifications.insert(q.clone()).unwrap();
        let mut terms = vec![];
        for n in [20, 21] {
            let declaration = d
                .native
                .occurrences
                .insert(source::Occurrence {
                    source: artifact.id(),
                    start: n as i64,
                    end: n as i64 + 1,
                    syntax_kind: source::SyntaxKind::StmtClassDef,
                    role: source::OccurrenceRole::Declaration,
                    structural_path: vec![n],
                })
                .unwrap();
            let class = d
                .native
                .entity_classes
                .insert(ClassEntity::Source { declaration })
                .unwrap();
            let entity = d.native.refs.insert(EntityRef::Class { class }).unwrap();
            d.native
                .symbol_resolutions
                .insert(SymbolEntityResolution {
                    symbol: id(n as u8),
                    context: id(2),
                    policy: ContentHash::of(b"class"),
                    status: ResolutionStatus::Resolved,
                    entity: Some(entity),
                    reason: EntityReason::DeclarationAgreement,
                })
                .unwrap();
            terms.push(
                d.native
                    .terms
                    .insert(types::TypeTerm::ClassObject { class: id(n as u8) })
                    .unwrap(),
            );
        }
        let mut objects = BTreeSet::new();
        for n in 10..14 {
            let callable = d
                .native
                .callables
                .insert(CallableEntity::Source {
                    declaration: id(n),
                    kind: CallableKind::Function,
                })
                .unwrap();
            objects.insert(
                d.native
                    .refs
                    .insert(EntityRef::Callable { callable })
                    .unwrap(),
            );
            let formal = d
                .native
                .occurrences
                .insert(source::Occurrence {
                    source: artifact.id(),
                    start: n as i64,
                    end: n as i64 + 1,
                    syntax_kind: source::SyntaxKind::Parameter,
                    role: source::OccurrenceRole::Parameter,
                    structural_path: vec![n as i32],
                })
                .unwrap();
            d.parameter_syntax
                .insert(syntax::ParameterSyntaxObservation {
                    qualification: q.id(),
                    function: id(n),
                    parameter: formal,
                    ordinal: 0,
                    kind: calls::ParameterKind::PositionalOrKeyword,
                    default: None,
                    default_literal: None,
                    annotation: None,
                })
                .unwrap();
            d.native
                .type_observations
                .insert(types::TypeObservation {
                    qualification: q.id(),
                    subject: formal,
                    role: types::TypeRole::Parameter,
                    declared: true,
                    term: terms[(n as usize - 10) / 2],
                })
                .unwrap();
        }
        let f = AnalyticFrame {
            structural: sf.id(),
            configuration: id(4),
        };
        let prep = Correspondence::new(&d, &b, true, false, false).unwrap();
        let mut exhausted = Output::new(&b);
        assert_eq!(
            type_layer(&d, &prep, &f, &objects, &mut exhausted, &b, 7).unwrap(),
            None
        );
        assert!(exhausted.contributions.is_empty());
        assert!(exhausted.pair_sources.is_empty());
        let mut complete = Output::new(&b);
        assert_eq!(
            type_layer(&d, &prep, &f, &objects, &mut complete, &b, 8).unwrap(),
            Some(8)
        );
        assert_eq!(complete.contributions.len(), 2);
        assert_eq!(complete.pair_sources.len(), 2);
    }
    #[test]
    fn corpus_mentions_keep_analytic_context_and_plural_candidates_with_aggregate_admission() {
        use normalized::links::{LinkReason, MentionEntityAssessment, MentionEntityCandidate};
        let b = ResourceBudget::fixed(8 << 20).unwrap();
        let (mut d, sf, _, _) = scopes(&b);
        let artifact =
            source::SourceArtifact::from_bytes(id(80), "guide.md".into(), &[b' '; 128]).unwrap();
        d.native.artifacts.insert(artifact.clone()).unwrap();
        let scope = source::CoverageScope::Artifact {
            artifact: artifact.id(),
        };
        d.native.scopes.insert(scope.clone()).unwrap();
        let q = assertion::AssertionQualification {
            context: id(81),
            scope: scope.id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        d.native.qualifications.insert(q.clone()).unwrap();
        d.corpus_libraries
            .insert(input::CorpusLibrary {
                corpus: id(80),
                library: id(1),
            })
            .unwrap();
        d.uses
            .insert(input::ArtifactUse {
                artifact: artifact.id(),
                input: id(80),
                role: input::SourceRole::Document,
            })
            .unwrap();
        for (ordinal, targets) in [[10u8, 11u8], [12u8, 13u8]].into_iter().enumerate() {
            let span = assertion::EvidenceSourceSpanId::of(&assertion::Evidence::SourceSpan {
                source: artifact.id(),
                start: ordinal as i64,
                end: ordinal as i64 + 1,
            })
            .unwrap();
            let passage = documents::DocumentNodePassageId::of(&documents::DocumentNode::Passage {
                span,
                ordinal: ordinal as i64,
            })
            .unwrap();
            let mention = documents::DocumentMentionObservation {
                qualification: q.id(),
                mention: documents::DocumentNodeMentionId::of(&documents::DocumentNode::Mention {
                    span,
                })
                .unwrap(),
                passage,
                class: documents::MentionClass::Exact,
                source: documents::MentionSource::InlineCode,
                form: "api".into(),
                access_path: Some("api".into()),
                qualified_name: None,
            };
            d.native.mentions.insert(mention.clone()).unwrap();
            let assessment = MentionEntityAssessment {
                observation: mention.id(),
                status: ResolutionStatus::Resolved,
                reason: LinkReason::ExplicitIdentity,
            };
            d.native
                .relation_mention_entity_assessments
                .insert(assessment.clone())
                .unwrap();
            let exposure = id(90 + ordinal as u8);
            d.native
                .relation_mention_entity_candidates
                .insert(MentionEntityCandidate {
                    assessment: assessment.id(),
                    exposure,
                })
                .unwrap();
            // Every matching analytic-context candidate survives; a corpus-context twin is excluded.
            for (target, ctx) in [(targets[0], id(2)), (targets[1], id(2)), (14, id(81))] {
                let resolution = SymbolEntityResolution {
                    symbol: id(target),
                    context: ctx,
                    policy: ContentHash::of(b"mention"),
                    status: ResolutionStatus::Resolved,
                    entity: Some(id(target)),
                    reason: EntityReason::DeclarationAgreement,
                };
                d.native
                    .symbol_resolutions
                    .insert(resolution.clone())
                    .unwrap();
                d.native
                    .entity_exposure_candidates
                    .insert(PublicExposureCandidate {
                        exposure,
                        resolution: resolution.id(),
                        support: id(target),
                    })
                    .unwrap();
            }
        }
        let f = AnalyticFrame {
            structural: sf.id(),
            configuration: id(4),
        };
        let objects = (10..15).map(id).collect();
        let prep = Correspondence::new(&d, &b, false, false, true).unwrap();
        let mut exhausted = Output::new(&b);
        assert_eq!(
            mention_layer_prepared(&d, &prep, &f, &objects, &mut exhausted, &b, 7).unwrap(),
            None
        );
        assert!(exhausted.contributions.is_empty());
        let mut complete = Output::new(&b);
        assert_eq!(
            mention_layer_prepared(&d, &prep, &f, &objects, &mut complete, &b, 8).unwrap(),
            Some(8)
        );
        assert_eq!(complete.contributions.len(), 2);
        let pairs = complete
            .contributions
            .iter()
            .map(|r| (r.left, r.right))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            pairs,
            BTreeSet::from([
                (
                    id::<EntityRef>(10).min(id(11)),
                    id::<EntityRef>(10).max(id(11))
                ),
                (
                    id::<EntityRef>(12).min(id(13)),
                    id::<EntityRef>(12).max(id(13))
                )
            ])
        );
    }

    #[test]
    fn relational_membership_scratch_releases_before_retained_facts_and_on_error() {
        let b = ResourceBudget::fixed(1 << 20).unwrap();
        let (mut d, sf, out, _) = scopes(&b);
        let module = d
            .native
            .refs
            .insert(EntityRef::Module { module: id(10) })
            .unwrap();
        for n in 20..100 {
            d.structural
                .scope
                .insert(structural::ScopeMember {
                    frame: sf.id(),
                    entity: id(n),
                    module: id(10),
                    declaration: id(n),
                    role: id(11),
                })
                .unwrap();
        }
        let prep = Correspondence::new(&d, &b, true, true, false).unwrap();
        let before = b.reserved();
        let (retained_facts, retained_selections, retained_charge) = facts(
            &d,
            &prep,
            &BTreeSet::from([module]),
            id(2),
            true,
            &out,
            &b,
            policy::RETAINED.attributes,
        )
        .unwrap();
        assert!(retained_facts.is_empty());
        assert!(retained_selections.is_empty());
        assert_eq!(b.reserved(), before);
        assert_eq!(retained_charge.reserved(), 0);
        drop(prep);
        d.structural
            .handoffs
            .insert(structural::handoffs::Handoff {
                frame: sf.id(),
                consumer: id(101),
                producer: id(102),
                binding: id(103),
                value: id(104),
                role: id(105),
            })
            .unwrap();
        let prep = Correspondence::new(&d, &b, true, true, false).unwrap();
        let before = b.reserved();
        assert!(
            facts(
                &d,
                &prep,
                &BTreeSet::from([module]),
                id(2),
                true,
                &out,
                &b,
                policy::RETAINED.attributes
            )
            .is_err()
        );
        assert_eq!(b.reserved(), before);
        drop((retained_facts, retained_selections, retained_charge));
    }

    #[test]
    fn unavailable_parameter_gates_remain_allocation_free_under_tight_remaining_budget() {
        let b = ResourceBudget::fixed(1 << 20).unwrap();
        let mut d = Data::new(&b);
        let q = assertion::AssertionQualification {
            context: id(2),
            scope: id(8),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        d.native.qualifications.insert(q.clone()).unwrap();
        let container = d
            .native
            .occurrences
            .insert(source::Occurrence {
                source: id(1),
                start: 0,
                end: 100,
                syntax_kind: source::SyntaxKind::ParameterWithDefault,
                role: source::OccurrenceRole::Parameter,
                structural_path: vec![0],
            })
            .unwrap();
        let unsupported = d
            .native
            .occurrences
            .insert(source::Occurrence {
                source: id(1),
                start: 0,
                end: 100,
                syntax_kind: source::SyntaxKind::StmtFunctionDef,
                role: source::OccurrenceRole::Declaration,
                structural_path: vec![1],
            })
            .unwrap();
        for n in 10..100 {
            d.native
                .placements
                .insert(syntax::SyntaxPlacement {
                    qualification: q.id(),
                    occurrence: id(n),
                    parent: Some(container),
                    field: lexical::SyntaxField::Child,
                    ordinal: n as i64,
                })
                .unwrap();
        }
        let parameter = syntax::ParameterSyntaxObservation {
            qualification: q.id(),
            function: id(3),
            parameter: container,
            ordinal: 0,
            kind: calls::ParameterKind::PositionalOrKeyword,
            default: None,
            default_literal: None,
            annotation: None,
        };
        let prep = Correspondence::new(&d, &b, true, true, false).unwrap();
        let hold = b
            .reserve("test-tight-remaining-budget", b.limit() - b.reserved() - 16)
            .unwrap();
        let before = b.reserved();
        assert!(prep.parameter(&d, &parameter, id(4), &b).unwrap().is_none());
        for parameter in [
            syntax::ParameterSyntaxObservation {
                parameter: id(101),
                ..parameter.clone()
            },
            syntax::ParameterSyntaxObservation {
                parameter: unsupported,
                ..parameter.clone()
            },
        ] {
            assert!(prep.parameter(&d, &parameter, id(2), &b).unwrap().is_none());
        }
        assert_eq!(b.reserved(), before);
        assert!(prep.parameter_cache.borrow().is_empty());
        drop(hold);
    }
}
