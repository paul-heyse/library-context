//! Declared source-class reachability for dynamic access. This inspection operation grants no
//! exact runtime class, allocation, mutable-state stability or complete member universe.
use super::{
    evaluation::EvaluationData,
    read_channels::{Work, declared_class_inspection, native, selected},
};
use crate::domain::{
    analysis::{
        base_evaluation as publication,
        native::NativeAssertionPremise,
        policy::{self, EvidenceStatus},
    },
    assertion::*,
    attribution::*,
    calls::*,
    conditions::entry::EntryData,
    flow::*,
    lexical::*,
    normalized::{Rows, entities::*},
    source::*,
    syntax::*,
    value::*,
    *,
};
use crate::{Domain, DomainCode};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DynamicKind {
    GetAttr = 0,
    Dictionary = 1,
    HasAttr = 2,
    SetAttr = 3,
    DelAttr = 4,
    Vars = 5,
    Exec = 6,
    Eval = 7,
    DunderImport = 8,
    ImportModule = 9,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ClassInspection {
    DeclaredReachableClass = 0,
    Unknown = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DeclaredClassOrigin {
    ReceiverDeclaration = 0,
    UniqueGlobalInitializer = 1,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_dynamic_access_observations")]
pub struct DynamicAccessObservation {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub site: Id<Occurrence>,
    #[model(key)]
    pub kind: DynamicKind,
    pub owner: Id<EntityRef>,
    pub receiver: Option<Id<Occurrence>>,
    pub qualification: Id<AssertionQualification>,
    pub status: EvidenceStatus,
    pub inspection: ClassInspection,
    pub declared_class: Option<Id<ClassEntity>>,
    pub origin: Option<DeclaredClassOrigin>,
    pub reason: Option<obligation::ObligationKind>,
    pub sources: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_dynamic_access_premises")]
pub struct DynamicAccessPremise {
    #[model(key)]
    pub access: Id<DynamicAccessObservation>,
    #[model(key)]
    pub premise: Id<NativeAssertionPremise>,
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<DynamicAccessObservation>(),
        Relation::of::<DynamicAccessPremise>(),
    ]
}
pub fn validation_inputs() -> Vec<ValidationInput> {
    vec![
        ValidationInput::of::<DynamicAccessObservation>(&["id"]),
        ValidationInput::of::<DynamicAccessPremise>(&["id"]),
    ]
}
const DEPTH_LIMIT: usize = 64;
struct Inspect<'a> {
    data: &'a EvaluationData,
    entry: &'a EntryData,
    invocation: &'a publication::AnalysisInvocation,
    work: &'a mut Work,
    budget: &'a resources::ResourceBudget,
    premises: Rows<NativeAssertionPremise>,
    actual:Option<super::read_channels::ReadEntries<'a>>,
}
struct DynamicSite {
    site: Id<Occurrence>,
    kind: DynamicKind,
    owner: Id<EntityRef>,
    receiver: Option<Id<Occurrence>>,
}

impl Inspect<'_> {
    fn observe<S: Support>(
        &mut self,
        supports: &Rows<S>,
        assertion: Id<S::Assertion>,
        q: Id<AssertionQualification>,
        site: Id<Occurrence>,
    ) -> Result<bool, ModelError> {
        let selected = declared_class_inspection(
            super::read_channels::NativeContext {
                data: self.data,
                entry: self.entry,
                invocation: self.invocation,
            },
            supports,
            assertion,
            q,
            site,
            self.work,
        )?;
        if let Some((id, _, _)) = selected {
            self.premises.insert(
                self.data
                    .premises
                    .get(id)
                    .ok_or_else(|| ModelError::Invalid("dynamic native pair missing".into()))?
                    .clone(),
            )?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn always(&self, q: Id<AssertionQualification>) -> bool {
        self.entry.qualifications.get(q).is_some_and(|q| {
            q.context == self.invocation.context
                && q.modality == Modality::Definite
                && q.approximation == Approximation::Exact
                && q.condition == conditions::Diagram::always().id()
        })
    }
    fn class_symbol(
        &mut self,
        symbol: Id<ProviderSymbol>,
    ) -> Result<Option<Id<ClassEntity>>, ModelError> {
        let mut declarations = self
            .entry
            .symbol_declarations
            .iter()
            .filter(|d| d.symbol == symbol && self.always(d.qualification));
        let Some(d) = declarations.next() else {
            return Ok(None);
        };
        if declarations.next().is_some() {
            return Ok(None);
        };
        let class = ClassEntity::Source {
            declaration: d.declaration,
        };
        if self.data.classes.get(class.id()) != Some(&class)
            || !self.observe(
                &self.entry.symbol_declaration_supports,
                d.id(),
                d.qualification,
                d.declaration,
            )?
        {
            return Ok(None);
        };
        Ok(Some(class.id()))
    }
    fn receiver_parameter(
        &mut self,
        declaration: Id<Occurrence>,
        scope: Id<LexicalScope>,
        site: Id<Occurrence>,
    ) -> Result<Option<Id<ClassEntity>>, ModelError> {
        let Some(function) = self.entry.lexical_scopes.get(scope).map(|s| s.owner) else {
            return Ok(None);
        };
        let Some(owner) = self.work.owner(site) else {
            return Ok(None);
        };
        let mut selected = None;
        for formal in self.entry.formals.iter() {
            self.work.tick()?;
            let ParameterEntity::Source {
                declaration: parameter,
            } = formal
            else {
                continue;
            };
            let matches = *parameter == declaration
                || self.entry.placements.iter().any(|p| {
                    p.occurrence == declaration
                        && p.parent == Some(*parameter)
                        && p.field == SyntaxField::Child
                        && p.ordinal == 0
                });
            if !matches {
                continue;
            }
            let mut run = None;
            for observation in self.entry.use_observations.iter().filter(|o| {
                self.entry
                    .uses
                    .get(o.use_)
                    .is_some_and(|u| u.occurrence == site)
            }) {
                if let Some((_, candidate, _)) = declared_class_inspection(
                    super::read_channels::NativeContext {
                        data: self.data,
                        entry: self.entry,
                        invocation: self.invocation,
                    },
                    &self.entry.use_supports,
                    observation.id(),
                    observation.qualification,
                    site,
                    self.work,
                )? {
                    if run.is_some() {
                        return Ok(None);
                    }
                    run = Some(candidate);
                }
            }
            let Some(run) = run else { continue };
            let request = conditions::entry::EntryRequest {
                owner,
                formal: formal.id(),
                access: site,
                context: self.invocation.context,
                run,
            };
            let proof=if let Some(actual)=self.actual {
                let mut candidates=actual.witnesses.iter().filter(|w|w.owner==request.owner && w.formal==request.formal && w.access==request.access && w.context==request.context && w.run==request.run && matches!(actual.sources.get(w.access_source),Some(conditions::entry::EntryAccessSource::Use {..})));
                let Some(witness)=candidates.next() else {continue;};
                if candidates.next().is_some(){return Ok(None);}
                let source=actual.sources.get(witness.access_source).ok_or_else(||ModelError::Invalid("actual dynamic entry source missing".into()))?;
                actual.actual.entry(witness,source,self.budget)?
            }else {
                // Finite whole-input owner oracle. Production always supplies actual Local.
                let Ok(proof)=conditions::entry::EntryValueWitness::derive(self.entry,request,self.budget)? else {continue;};proof
            };
            if selected.is_some() {
                return Ok(None);
            };
            let witness = proof.witness();
            let link = self
                .entry
                .links
                .get(witness.link)
                .ok_or_else(|| ModelError::Invalid("dynamic entry link missing".into()))?;
            let parameter = self
                .entry
                .parameters
                .get(link.parameter)
                .ok_or_else(|| ModelError::Invalid("dynamic entry parameter missing".into()))?;
            let signature = self
                .entry
                .signatures
                .get(parameter.signature)
                .ok_or_else(|| ModelError::Invalid("dynamic signature missing".into()))?;
            let declared = self
                .entry
                .declarations
                .get(link.declaration.ok_or_else(|| {
                    ModelError::Invalid("dynamic parameter declaration missing".into())
                })?)
                .ok_or_else(|| {
                    ModelError::Invalid("dynamic parameter declaration missing".into())
                })?;
            if !self.observe(
                &self.entry.declaration_supports,
                declared.id(),
                declared.qualification,
                site,
            )? {
                return Ok(None);
            };
            if let Some(placement) = witness.parameter_placement {
                let p = self.entry.placements.get(placement).unwrap();
                if !self.observe(
                    &self.entry.placement_supports,
                    p.id(),
                    p.qualification,
                    p.occurrence,
                )? {
                    return Ok(None);
                }
            }
            selected = Some((parameter, signature));
        }
        let Some((parameter, signature)) = selected else {
            return Ok(None);
        };
        // Native instance receiver shape, rather than a spelling such as self or an annotation.
        if self
            .entry
            .parameters
            .iter()
            .filter(|p| p.signature == signature.id())
            .any(|p| p.ordinal < parameter.ordinal)
        {
            return Ok(None);
        };
        let mut owners = self.entry.symbol_declarations.iter().filter(|d| {
            d.symbol == signature.symbol
                && d.declaration == function
                && self.always(d.qualification)
        });
        let Some(owner) = owners.next() else {
            return Ok(None);
        };
        if owners.next().is_some()
            || !self.observe(
                &self.entry.symbol_declaration_supports,
                owner.id(),
                owner.qualification,
                function,
            )?
        {
            return Ok(None);
        };
        let mut trait_rows = self
            .data
            .function_traits
            .iter()
            .filter(|t| t.symbol == signature.symbol && self.always(t.qualification));
        let Some(traits) = trait_rows.next() else {
            return Ok(None);
        };
        if trait_rows.next().is_some()
            || traits.staticmethod
            || traits.classmethod
            || traits.property_getter
            || traits.property_setter
            || !self.observe(
                &self.data.function_trait_supports,
                traits.id(),
                traits.qualification,
                function,
            )?
        {
            return Ok(None);
        };
        let Some(class) = traits.defining_class else {
            return Ok(None);
        };
        self.class_symbol(class)
    }
    fn lexical_target(
        &mut self,
        site: Id<Occurrence>,
    ) -> Result<Option<Id<LexicalTarget>>, ModelError> {
        let mut selected = None;
        self.work.scan(self.data.lexical_resolutions.len())?;
        for row in self.data.lexical_resolutions.iter() {
            if row.read != site || !self.always(row.qualification) {
                continue;
            }
            if !self.observe(
                &self.data.lexical_resolution_supports,
                row.id(),
                row.qualification,
                site,
            )? {
                continue;
            }
            if selected.is_some() {
                return Ok(None);
            }
            selected = Some(row.target);
        }
        Ok(selected)
    }
    fn builtin(&mut self, site: Id<Occurrence>, name: &str) -> Result<bool, ModelError> {
        let Some(target) = self.lexical_target(site)? else {
            return Ok(false);
        };
        Ok(
            matches!(self.data.lexical_targets.get(target),Some(LexicalTarget::Builtin{name:actual,..})if actual==name),
        )
    }
    /// A supported, sole local lambda assignment is a source function, not a builtin selected
    /// by its variable's spelling. Parameters, imported aliases and rebinding remain uncertain.
    fn local_lambda(&mut self, site: Id<Occurrence>) -> Result<bool, ModelError> {
        let Some(target) = self.lexical_target(site)? else {
            return Ok(false);
        };
        let Some(LexicalTarget::Binding { event }) = self.data.lexical_targets.get(target) else {
            return Ok(false);
        };
        let Some(event) = self.data.binding_events.get(*event) else {
            return Ok(false);
        };
        let Some((binding, proof)) = super::read_channels::native_binding(
            self.data,
            self.entry,
            self.invocation,
            event.id(),
            self.work,
        )?
        else {
            return Ok(false);
        };
        let Some(scope) = binding.scope else {
            return Ok(false);
        };
        if !self.always(binding.qualification)
            || binding.kind != ruff::RuffBindingKind::Assignment
            || binding.typing
            || binding.lazy
            || binding.deleted
            || binding.external
            || binding.global
            || binding.nonlocal
            || self
                .entry
                .lexical_scopes
                .get(scope)
                .is_none_or(|s| s.kind != LexicalScopeKind::Function)
        {
            return Ok(false);
        }
        // Retained source characterization may conservatively refuse a conditional
        // assignment; it never supplies the positive binding or lambda-value proof.
        self.work.scan(self.data.bindings.len())?;
        if self.data.bindings.iter().any(|b| {
            b.event == event.id()
                && b.static_branch.is_some()
                && self
                    .entry
                    .qualifications
                    .get(b.qualification)
                    .is_some_and(|q| q.context == self.invocation.context)
        }) {
            return Ok(false);
        }
        self.work.scan(self.data.ruff_bindings.len())?;
        if self
            .data
            .ruff_bindings
            .iter()
            .filter(|b| {
                b.scope == Some(scope)
                    && b.native_name == binding.native_name
                    && self
                        .entry
                        .qualifications
                        .get(b.qualification)
                        .is_some_and(|q| q.context == self.invocation.context)
            })
            .count()
            != 1
        {
            return Ok(false);
        }
        // The native binding supplies identity; the original native syntax supplies its
        // lambda value. A recognizer's BindingObservation.value is not this proof.
        self.work.scan(self.data.placements.len())?;
        let mut targets = self
            .data
            .placements
            .iter()
            .filter(|p| p.occurrence == event.site && self.always(p.qualification));
        let Some(target) = targets.next() else {
            return Ok(false);
        };
        if targets.next().is_some()
            || !self.observe(
                &self.entry.placement_supports,
                target.id(),
                target.qualification,
                event.site,
            )?
        {
            return Ok(false);
        }
        let Some(parent) = target.parent else {
            return Ok(false);
        };
        if self
            .entry
            .occurrences
            .get(parent)
            .is_none_or(|o| o.syntax_kind != SyntaxKind::StmtAssign)
        {
            return Ok(false);
        }
        self.work
            .scan(self.data.placements.len() + self.data.callables.len())?;
        let mut values = self.data.placements.iter().filter(|p| {
            p.parent == Some(parent)
                && p.field == SyntaxField::Value
                && self.always(p.qualification)
        });
        let Some(value) = values.next() else {
            return Ok(false);
        };
        if values.next().is_some()
            || self
                .entry
                .occurrences
                .get(value.occurrence)
                .is_none_or(|o| o.syntax_kind != SyntaxKind::ExprLambda)
            || !self.data.callables.iter().any(|c| {
                matches!(c, CallableEntity::Source {
                declaration, kind: CallableKind::Lambda
            } if *declaration == value.occurrence)
            })
            || !self.observe(
                &self.entry.placement_supports,
                value.id(),
                value.qualification,
                value.occurrence,
            )?
        {
            return Ok(false);
        }
        self.premises.insert(
            self.data
                .premises
                .get(proof.0)
                .ok_or_else(|| ModelError::Invalid("lambda native binding premise missing".into()))?
                .clone(),
        )?;
        Ok(true)
    }

    /// Inspect a sole native module assignment and its original syntax value. Retained
    /// recognizer rows can refuse conditional source shapes, but never supply this proof.
    fn global_value(&mut self, event: &BindingEvent) -> Result<Option<Id<Occurrence>>, ModelError> {
        let Some((binding, proof)) = super::read_channels::native_binding(
            self.data,
            self.entry,
            self.invocation,
            event.id(),
            self.work,
        )?
        else {
            return Ok(None);
        };
        let Some(scope) = binding.scope else {
            return Ok(None);
        };
        if !self.always(binding.qualification)
            || binding.kind != ruff::RuffBindingKind::Assignment
            || binding.typing
            || binding.lazy
            || binding.deleted
            || binding.external
            || binding.global
            || binding.nonlocal
            || self
                .entry
                .lexical_scopes
                .get(scope)
                .is_none_or(|s| s.kind != LexicalScopeKind::Module)
        {
            return Ok(None);
        }
        self.work
            .scan(self.data.bindings.len() + self.data.ruff_bindings.len())?;
        if self.data.bindings.iter().any(|b| {
            b.event == event.id()
                && b.static_branch.is_some()
                && self
                    .entry
                    .qualifications
                    .get(b.qualification)
                    .is_some_and(|q| q.context == self.invocation.context)
        }) || self
            .data
            .ruff_bindings
            .iter()
            .filter(|b| {
                b.scope == Some(scope)
                    && b.native_name == binding.native_name
                    && self
                        .entry
                        .qualifications
                        .get(b.qualification)
                        .is_some_and(|q| q.context == self.invocation.context)
            })
            .count()
            != 1
        {
            return Ok(None);
        }
        self.work.scan(self.data.placements.len())?;
        let mut targets = self
            .data
            .placements
            .iter()
            .filter(|p| p.occurrence == event.site && self.always(p.qualification));
        let Some(target) = targets.next() else {
            return Ok(None);
        };
        if targets.next().is_some()
            || !self.observe(
                &self.entry.placement_supports,
                target.id(),
                target.qualification,
                event.site,
            )?
        {
            return Ok(None);
        }
        let Some(parent) = target.parent else {
            return Ok(None);
        };
        if self
            .entry
            .occurrences
            .get(parent)
            .is_none_or(|o| o.syntax_kind != SyntaxKind::StmtAssign)
        {
            return Ok(None);
        }
        self.work.scan(self.data.placements.len())?;
        let mut values = self.data.placements.iter().filter(|p| {
            p.parent == Some(parent)
                && p.field == SyntaxField::Value
                && self.always(p.qualification)
        });
        let Some(value) = values.next() else {
            return Ok(None);
        };
        if values.next().is_some()
            || !self.observe(
                &self.entry.placement_supports,
                value.id(),
                value.qualification,
                value.occurrence,
            )?
        {
            return Ok(None);
        }
        self.premises.insert(
            self.data
                .premises
                .get(proof.0)
                .ok_or_else(|| ModelError::Invalid("global native binding premise missing".into()))?
                .clone(),
        )?;
        Ok(Some(value.occurrence))
    }
    /// A located finalized native reference can identify a declared module binding across
    /// lexical scopes. This source inspection supplies no closure capture or runtime value.
    fn module_reference(
        &mut self,
        site: Id<Occurrence>,
    ) -> Result<Option<Id<BindingEvent>>, ModelError> {
        let Some(source) = self.entry.occurrences.get(site).map(|o| o.source) else {
            return Ok(None);
        };
        self.work.scan(self.data.ruff_contexts.len())?;
        let mut selected = None;
        for row in self.data.ruff_contexts.iter().filter(|r| r.subject == site) {
            if row.phase != ruff::ContextPhase::FinalReference
                || row.reference_load != Some(true)
                || row.typing != Some(false)
                || row.typing_only_annotation != Some(false)
                || row.runtime_annotation != Some(false)
                || row.string_annotation != Some(false)
                || row.type_checking != Some(false)
                || row.final_binding_location != Some(ruff::AttachmentStatus::Located)
                || !self.always(row.qualification)
            {
                continue;
            }
            let Some(event) = row.final_binding else {
                continue;
            };
            let Some(reference) = declared_class_inspection(
                super::read_channels::NativeContext {
                    data: self.data,
                    entry: self.entry,
                    invocation: self.invocation,
                },
                &self.data.ruff_context_supports,
                row.id(),
                row.qualification,
                site,
                self.work,
            )?
            else {
                continue;
            };
            let Some((binding, proof)) = super::read_channels::native_binding(
                self.data,
                self.entry,
                self.invocation,
                event,
                self.work,
            )?
            else {
                continue;
            };
            if reference.1 != proof.1
                || binding
                    .scope
                    .and_then(|s| self.entry.lexical_scopes.get(s))
                    .is_none_or(|scope| {
                        scope.kind != LexicalScopeKind::Module
                            || self
                                .entry
                                .occurrences
                                .get(scope.owner)
                                .is_none_or(|o| o.source != source)
                    })
            {
                continue;
            }
            if selected.is_some() {
                return Ok(None);
            }
            self.premises.insert(
                self.data
                    .premises
                    .get(reference.0)
                    .ok_or_else(|| {
                        ModelError::Invalid("module reference native premise missing".into())
                    })?
                    .clone(),
            )?;
            selected = Some(event);
        }
        Ok(selected)
    }
    fn global(
        &mut self,
        event: Id<BindingEvent>,
        depth: usize,
    ) -> Result<Option<(Id<ClassEntity>, DeclaredClassOrigin)>, ModelError> {
        let Some(event) = self.data.binding_events.get(event) else {
            return Ok(None);
        };
        let Some(value) = self.global_value(event)? else {
            return Ok(None);
        };
        self.constructed_class(value, depth)
    }
    fn constructed_class(
        &mut self,
        site: Id<Occurrence>,
        depth: usize,
    ) -> Result<Option<(Id<ClassEntity>, DeclaredClassOrigin)>, ModelError> {
        if depth >= DEPTH_LIMIT {
            return Ok(None);
        };
        let mut calls = self
            .data
            .call_syntax
            .iter()
            .filter(|c| c.site == site && self.always(c.qualification));
        let Some(call) = calls.next() else {
            return Ok(None);
        };
        if calls.next().is_some()
            || !self.observe(
                &self.data.call_syntax_supports,
                call.id(),
                call.qualification,
                site,
            )?
        {
            return Ok(None);
        };
        let Some(target) = self.lexical_target(call.callee)? else {
            return Ok(None);
        };
        let Some(LexicalTarget::Binding { event }) = self.data.lexical_targets.get(target) else {
            return Ok(None);
        };
        let Some(event) = self.data.binding_events.get(*event) else {
            return Ok(None);
        };
        let Some((binding, proof)) = super::read_channels::native_binding(
            self.data,
            self.entry,
            self.invocation,
            event.id(),
            self.work,
        )?
        else {
            return Ok(None);
        };
        let Some(scope) = binding.scope else {
            return Ok(None);
        };
        if binding.kind != ruff::RuffBindingKind::ClassDefinition
            || !self.always(binding.qualification)
            || binding.typing
            || binding.lazy
            || binding.deleted
            || binding.external
            || binding.global
            || binding.nonlocal
        {
            return Ok(None);
        }
        self.work.scan(self.data.ruff_bindings.len())?;
        if self
            .data
            .ruff_bindings
            .iter()
            .filter(|b| {
                b.scope == Some(scope)
                    && b.native_name == binding.native_name
                    && self
                        .entry
                        .qualifications
                        .get(b.qualification)
                        .is_some_and(|q| q.context == self.invocation.context)
            })
            .count()
            != 1
        {
            return Ok(None);
        }
        self.premises.insert(
            self.data
                .premises
                .get(proof.0)
                .ok_or_else(|| ModelError::Invalid("class native binding premise missing".into()))?
                .clone(),
        )?;
        let class = ClassEntity::Source {
            declaration: event.site,
        };
        if self.data.classes.get(class.id()) != Some(&class) {
            return Ok(None);
        };
        Ok(Some((
            class.id(),
            DeclaredClassOrigin::UniqueGlobalInitializer,
        )))
    }
    fn trace(
        &mut self,
        site: Id<Occurrence>,
        scope: Id<LexicalScope>,
        depth: usize,
    ) -> Result<Option<(Id<ClassEntity>, DeclaredClassOrigin)>, ModelError> {
        self.work.tick()?;
        if depth >= DEPTH_LIMIT
            || self
                .entry
                .occurrences
                .get(site)
                .is_none_or(|o| o.syntax_kind != SyntaxKind::ExprName)
        {
            return Ok(None);
        };
        let mut uses = self.entry.uses.iter().filter(|u| u.occurrence == site);
        let Some(use_) = uses.next() else {
            return Ok(None);
        };
        if uses.next().is_some() {
            return Ok(None);
        };
        let mut observations =
            self.entry.use_observations.iter().filter(|o| {
                o.use_ == use_.id() && o.scope == scope && self.always(o.qualification)
            });
        let Some(observation) = observations.next() else {
            return Ok(None);
        };
        if observations.next().is_some()
            || observation.annotation
            || !self.observe(
                &self.entry.use_supports,
                observation.id(),
                observation.qualification,
                site,
            )?
        {
            return Ok(None);
        };
        if let Some(event) = self.module_reference(site)? {
            return self.global(event, depth + 1);
        }
        let mut reach_rows = self.entry.reaching.iter().filter(|r| {
            r.use_ == use_.id()
                && self
                    .entry
                    .qualifications
                    .get(r.qualification)
                    .is_some_and(|q| q.context == self.invocation.context)
        });
        let Some(reach) = reach_rows.next() else {
            return Ok(None);
        };
        if reach_rows.next().is_some()
            || reach.loop_carried
            || !self.always(reach.qualification)
            || !self.observe(
                &self.entry.reaching_supports,
                reach.id(),
                reach.qualification,
                site,
            )?
        {
            return Ok(None);
        };
        let Some(ReachingDefinition::Bound { definition }) = self.entry.targets.get(reach.target)
        else {
            return Ok(None);
        };
        let Some(definition) = self.entry.definitions.get(*definition) else {
            return Ok(None);
        };
        let mut definitions = self.entry.definition_observations.iter().filter(|d| {
            d.definition == definition.id() && d.scope == scope && self.always(d.qualification)
        });
        let Some(observed) = definitions.next() else {
            return Ok(None);
        };
        if definitions.next().is_some()
            || !self.observe(
                &self.entry.definition_supports,
                observed.id(),
                observed.qualification,
                definition.occurrence,
            )?
        {
            return Ok(None);
        };
        if observed.kind == BindingEventKind::Parameter {
            return Ok(self
                .receiver_parameter(definition.occurrence, scope, site)?
                .map(|c| (c, DeclaredClassOrigin::ReceiverDeclaration)));
        };
        if observed.kind != BindingEventKind::Assignment {
            return Ok(None);
        };
        let Some(value) = observed.value else {
            return Ok(None);
        };
        self.trace(value, scope, depth + 1)
    }
    fn emit(
        &mut self,
        records: &mut super::read_channels::ReadRecords,
        dynamic_site: DynamicSite,
        q: Id<AssertionQualification>,
        scope: Option<Id<LexicalScope>>,
        admitted: bool,
    ) -> Result<(), ModelError> {
        let DynamicSite {
            site,
            kind,
            owner,
            receiver,
        } = dynamic_site;
        let class = if admitted {
            if let (Some(receiver), Some(scope)) = (receiver, scope) {
                self.trace(receiver, scope, 0)?
            } else {
                None
            }
        } else {
            None
        };
        let mut digest = KeySink::new("dynamic-declared-class-native-premises");
        let mut status = EvidenceStatus::StructurallyObserved;
        for premise in self.premises.iter() {
            premise.id().encode(&mut digest);
            let native = self
                .work
                .qualification(self.data, premise.id())
                .ok_or_else(|| {
                    ModelError::Invalid("dynamic native qualification missing".into())
                })?;
            status = policy::derive_status(&[
                (policy::SupportRole::Support, status),
                (policy::SupportRole::Support, native.status),
            ]);
        }
        if self.premises.is_empty() {
            status = EvidenceStatus::Unresolved;
        }
        let row = DynamicAccessObservation {
            invocation: self.invocation.id(),
            site,
            kind,
            owner,
            receiver,
            qualification: q,
            status,
            inspection: if class.is_some() {
                ClassInspection::DeclaredReachableClass
            } else {
                ClassInspection::Unknown
            },
            declared_class: class.map(|c| c.0),
            origin: class.map(|c| c.1),
            reason: class
                .is_none()
                .then_some(obligation::ObligationKind::DynamicAccess),
            sources: digest.finish(),
        };
        let id = records.dynamic.insert(row)?;
        for premise in self.premises.iter() {
            records.dynamic_premises.insert(DynamicAccessPremise {
                access: id,
                premise: premise.id(),
            })?;
        }
        Ok(())
    }
}
type NativeCallTarget = (&'static str, Id<CallTarget>, bool);

/// Native resolved builtin/module target inspection catches qualified and imported aliases.
/// Mixed target inventories retain a broad unknown access; no runtime uniqueness is granted.
pub(super) fn native_name(
    data: &EvaluationData,
    entry: &EntryData,
    inv: &publication::AnalysisInvocation,
    site: Id<Occurrence>,
    work: &mut Work,
) -> Result<Option<NativeCallTarget>, ModelError> {
    let count = work.dynamic_targets.get(&site).map_or(0, Vec::len);
    let mut selected = None;
    let mut mixed = false;
    for ordinal in 0..count {
        work.tick()?;
        let id = work.dynamic_targets.get(&site).unwrap()[ordinal];
        let target = data
            .call_targets
            .get(id)
            .ok_or_else(|| ModelError::Invalid("dynamic call target missing".into()))?;
        if target.phase != CallPhase::Call {
            continue;
        }
        let name = (|| {
            let CallDestination::Resolved { symbol } =
                data.call_destinations.get(target.destination)?
            else {
                return None;
            };
            let symbol = entry.symbols.get(*symbol)?;
            if symbol.context != inv.context {
                return None;
            }
            let module = match data.provider_modules.get(symbol.module) {
                Some(ProviderModule::Bundled { name, .. }) => Some(name.as_str()),
                Some(ProviderModule::Acquired { module }) => entry
                    .modules
                    .get(*module)
                    .map(|m| m.qualified_name.as_str()),
                _ => None,
            };
            let name = symbol.name.rsplit('.').next().unwrap_or(&symbol.name);
            match (module, name) {
                (Some("builtins"), "getattr") => Some("getattr"),
                (Some("builtins"), "hasattr") => Some("hasattr"),
                (Some("builtins"), "setattr") => Some("setattr"),
                (Some("builtins"), "delattr") => Some("delattr"),
                (Some("builtins"), "vars") => Some("vars"),
                (Some("builtins"), "exec") => Some("exec"),
                (Some("builtins"), "eval") => Some("eval"),
                (Some("builtins"), "__import__") => Some("__import__"),
                (Some("importlib"), "import_module") => Some("import_module"),
                _ => None,
            }
        })();
        let Some(name) = name else {
            mixed = true;
            continue;
        };
        if native(
            super::read_channels::NativeContext {
                data,
                entry,
                invocation: inv,
            },
            &data.call_target_supports,
            target.id(),
            target.qualification,
            site,
            work,
        )?
        .is_none()
        {
            mixed = true;
            continue;
        }
        if let Some((prior, _)) = selected {
            if prior != name {
                mixed = true;
                if matches!(name, "exec" | "eval") {
                    selected = Some((name, target.id()));
                }
            }
        } else {
            selected = Some((name, target.id()));
        }
    }
    Ok(selected.map(|(name, id)| (name, id, !mixed)))
}

#[derive(Clone,Copy)]
pub(super) enum SelectedRoot {Call(Id<calls::CallSyntax>),Attribute(Id<flow::FlowAttributeLoadObservation>)}
pub(super) fn produce_selected(
    data: &EvaluationData,
    entry: &EntryData,
    invocation: &publication::AnalysisInvocation,
    roots: &std::collections::BTreeSet<Id<SourceArtifact>>,
    records: &mut super::read_channels::ReadRecords,
    budget: &resources::ResourceBudget,
    work: &mut Work,
    selected_id:Option<SelectedRoot>,
    actual:Option<super::read_channels::ReadEntries<'_>>,
) -> Result<(), ModelError> {
    for call in data.call_syntax.iter().filter(|row|selected_id.is_none_or(|root|matches!(root,SelectedRoot::Call(id)if row.id()==id))) {
        work.tick()?;
        if !selected(entry, invocation, call.site, roots)
            || entry
                .qualifications
                .get(call.qualification)
                .is_none_or(|q| q.context != invocation.context)
        {
            continue;
        }
        let native_target = native_name(data, entry, invocation, call.site, work)?;
        let named = data.references.iter().find(|r| {
            r.read == call.callee
                && entry
                    .qualifications
                    .get(r.qualification)
                    .is_some_and(|q| q.context == invocation.context)
        });
        let name = native_target
            .map(|t| t.0)
            .or_else(|| named.map(|r| r.name.as_str()))
            .or_else(|| {
                data.spellings
                    .iter()
                    .find(|s| {
                        s.occurrence == call.callee
                            && s.spelling == "importlib.import_module"
                            && entry
                                .qualifications
                                .get(s.qualification)
                                .is_some_and(|q| q.context == invocation.context)
                    })
                    .map(|_| "import_module")
            });
        let Some(name) = name else { continue };
        let kind = match name {
            "getattr" => DynamicKind::GetAttr,
            "hasattr" => DynamicKind::HasAttr,
            "setattr" => DynamicKind::SetAttr,
            "delattr" => DynamicKind::DelAttr,
            "vars" => DynamicKind::Vars,
            "exec" => DynamicKind::Exec,
            "eval" => DynamicKind::Eval,
            "__import__" => DynamicKind::DunderImport,
            "import_module" => DynamicKind::ImportModule,
            _ => continue,
        };
        let Some(owner) = work.owner(call.site) else {
            continue;
        };
        let mut inspect = Inspect {
            data,
            entry,
            invocation,
            work,
            budget,
            premises: Rows::new(budget),
            actual,
        };
        let native = inspect.observe(
            &data.call_syntax_supports,
            call.id(),
            call.qualification,
            call.site,
        )?;
        let lexical_builtin = inspect.builtin(call.callee, name)?;
        if native_target.is_none() && !lexical_builtin && inspect.local_lambda(call.callee)? {
            continue;
        }
        let builtin = lexical_builtin || native_target.is_some_and(|t| t.2);
        if let Some((_, id, _)) = native_target {
            let t = data.call_targets.get(id).unwrap();
            inspect.observe(&data.call_target_supports, t.id(), t.qualification, t.site)?;
        }
        if kind == DynamicKind::ImportModule
            && let Some(spelling) = data
                .spellings
                .iter()
                .find(|s| s.occurrence == call.callee && inspect.always(s.qualification))
        {
            inspect.observe(
                &data.spelling_supports,
                spelling.id(),
                spelling.qualification,
                call.callee,
            )?;
        }
        // Literal getter/hasattr names are exact source-name reads, not unconstrained dynamic access.
        let literal_name = data
            .call_arguments
            .iter()
            .find(|a| a.call == call.id() && a.ordinal == 1 && a.kind == ArgumentKind::Positional)
            .and_then(|arg| {
                data.details
                    .iter()
                    .find(|d| d.occurrence == arg.value && inspect.always(d.qualification))
            })
            .and_then(|d| data.detail_values.get(d.detail))
            .and_then(|d| match d {
                SyntaxDetail::Literal { literal } => data.literals.get(*literal),
                _ => None,
            })
            .is_some_and(|l| matches!(l, Literal::String { .. }));
        if native
            && builtin
            && literal_name
            && matches!(
                kind,
                DynamicKind::GetAttr
                    | DynamicKind::HasAttr
                    | DynamicKind::SetAttr
                    | DynamicKind::DelAttr
            )
        {
            continue;
        }
        let receiver = data
            .call_arguments
            .iter()
            .find(|a| a.call == call.id() && a.ordinal == 0 && a.kind == ArgumentKind::Positional)
            .map(|a| a.value);
        let scope = receiver.and_then(|receiver| {
            entry
                .use_observations
                .iter()
                .find(|o| {
                    entry
                        .uses
                        .get(o.use_)
                        .is_some_and(|u| u.occurrence == receiver)
                        && entry
                            .qualifications
                            .get(o.qualification)
                            .is_some_and(|q| q.context == invocation.context)
                })
                .map(|o| o.scope)
        });
        let receiver_kind = matches!(
            kind,
            DynamicKind::GetAttr
                | DynamicKind::HasAttr
                | DynamicKind::SetAttr
                | DynamicKind::DelAttr
                | DynamicKind::Vars
        );
        inspect.emit(
            records,
            DynamicSite {
                site: call.site,
                kind,
                owner,
                receiver,
            },
            call.qualification,
            scope,
            native && builtin && receiver_kind,
        )?;
    }
    for attribute in data.attribute_loads.iter().filter(|row|selected_id.is_none_or(|root|matches!(root,SelectedRoot::Attribute(id)if row.id()==id))) {
        work.tick()?;
        if attribute.name != "__dict__"
            || !selected(entry, invocation, attribute.occurrence, roots)
            || entry
                .qualifications
                .get(attribute.qualification)
                .is_none_or(|q| q.context != invocation.context)
        {
            continue;
        }
        let Some(owner) = work.owner(attribute.occurrence) else {
            continue;
        };
        let mut inspect = Inspect {
            data,
            entry,
            invocation,
            work,
            budget,
            premises: Rows::new(budget),
            actual,
        };
        let native = inspect.observe(
            &data.attribute_load_supports,
            attribute.id(),
            attribute.qualification,
            attribute.occurrence,
        )?;
        let mut children = data.placements.iter().filter(|p| {
            p.parent == Some(attribute.occurrence)
                && p.field == SyntaxField::Value
                && inspect.always(p.qualification)
        });
        let receiver = children
            .next()
            .filter(|_| children.next().is_none())
            .map(|p| p.occurrence);
        let scope = receiver.and_then(|receiver| {
            entry
                .use_observations
                .iter()
                .find(|o| {
                    entry
                        .uses
                        .get(o.use_)
                        .is_some_and(|u| u.occurrence == receiver)
                        && inspect.always(o.qualification)
                })
                .map(|o| o.scope)
        });
        inspect.emit(
            records,
            DynamicSite {
                site: attribute.occurrence,
                kind: DynamicKind::Dictionary,
                owner,
                receiver,
            },
            attribute.qualification,
            scope,
            native,
        )?;
    }
    Ok(())
}
/// Source-only unique module initializer inspection, independent of any emitted dynamic row.
/// A candidate is never an allocation, singleton runtime, or mutable-state certificate.
pub(super) fn global_candidate(
    data: &EvaluationData,
    entry: &EntryData,
    invocation: &publication::AnalysisInvocation,
    binding: &BindingObservation,
    budget: &resources::ResourceBudget,
    work: &mut Work,
    actual:Option<super::read_channels::ReadEntries<'_>>,
) -> Result<Option<(Id<ClassEntity>, ContentHash, EvidenceStatus)>, ModelError> {
    let Some(event) = data.binding_events.get(binding.event) else {
        return Ok(None);
    };
    let mut inspect = Inspect {
        data,
        entry,
        invocation,
        work,
        budget,
        premises: Rows::new(budget),
        actual,
    };
    let Some(value) = inspect.global_value(event)? else {
        return Ok(None);
    };
    let Some((class, _)) = inspect.constructed_class(value, 0)? else {
        return Ok(None);
    };
    let mut digest = KeySink::new("global-source-class-premises");
    let mut status = EvidenceStatus::StructurallyObserved;
    for premise in inspect.premises.iter() {
        premise.id().encode(&mut digest);
        let native = inspect
            .work
            .qualification(data, premise.id())
            .ok_or_else(|| {
                ModelError::Invalid("global class native qualification missing".into())
            })?;
        status = policy::derive_status(&[
            (policy::SupportRole::Support, status),
            (policy::SupportRole::Support, native.status),
        ]);
    }
    Ok(Some((class, digest.finish(), status)))
}
