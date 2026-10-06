//! Source-model field/global read screening, independent of heap state and Summary reach.
//! The old fixedpoint budget guard protected its derived inventory. Here the full admitted
//! native inventory is independently replayed and bounded: incomplete inputs refuse negatives.
//! A same-name load screens every class; unresolved dynamic receivers screen all classes.
use super::{
    evaluation::EvaluationData,
    read_channels::{
        ReadAssessment, ReadRecords, Work, declared_class_inspection, input_scope, native,
        selected, source_class_hierarchy,
    },
};
use crate::domain::{
    analysis::{
        base_evaluation as publication, native::NativeAssertionPremise, policy::EvidenceStatus,
    },
    assertion::*,
    attribution::*,
    conditions::entry::EntryData,
    lexical::*,
    normalized::{Rows, entities::*},
    source::*,
    symbols::*,
    syntax::*,
    value::*,
    *,
};
use crate::{Domain, DomainCode};
#[derive(Debug, Clone, Copy, PartialEq, Eq, DomainCode)]
#[repr(i16)]
pub enum FieldLocationKind {
    ReceiverStore = 0,
    ClassDeclaration = 1,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_field_locations")]
pub struct FieldLocationObservation {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub class: Id<ClassEntity>,
    #[model(key)]
    pub name: String,
    #[model(key)]
    pub site: Id<Occurrence>,
    pub kind: FieldLocationKind,
    pub premise: Option<Id<NativeAssertionPremise>>,
    pub qualification: Id<AssertionQualification>,
    pub sources: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_field_read_assessments")]
pub struct FieldReadAssessment {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub class: Id<ClassEntity>,
    #[model(key)]
    pub name: String,
    pub status: ReadAssessment,
    pub reason: Option<obligation::ObligationKind>,
    pub universe: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_global_class_inspections")]
pub struct GlobalClassInspection {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub binding: Id<BindingEvent>,
    pub class: Id<ClassEntity>,
    pub status: EvidenceStatus,
    pub sources: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_global_field_read_assessments")]
pub struct GlobalFieldReadAssessment {
    #[model(key)]
    pub global: Id<GlobalClassInspection>,
    #[model(key)]
    pub name: String,
    pub status: ReadAssessment,
    pub reason: Option<obligation::ObligationKind>,
    pub universe: ContentHash,
}
pub struct FieldRecords {
    pub locations: Rows<FieldLocationObservation>,
    pub assessments: Rows<FieldReadAssessment>,
    pub globals: Rows<GlobalClassInspection>,
    pub global_assessments: Rows<GlobalFieldReadAssessment>,
}
macro_rules! types{($m:ident)=>{$m!{locations:FieldLocationObservation,assessments:FieldReadAssessment,globals:GlobalClassInspection,global_assessments:GlobalFieldReadAssessment,}};}
impl FieldRecords {
    pub fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            locations: Rows::new(budget),
            assessments: Rows::new(budget),
            globals: Rows::new(budget),
            global_assessments: Rows::new(budget),
        }
    }
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        macro_rules! read{($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*};}
        types!(read);
        Ok(())
    }
    pub fn append(&mut self, other: &Self) -> Result<(), ModelError> {
        macro_rules! copy{($($field:ident:$ty:ty,)*)=>{$(for row in other.$field.iter(){self.$field.insert(row.clone())?;})*};}
        types!(copy);
        Ok(())
    }
    pub fn same(&self, other: &Self) -> bool {
        self.locations.same(&other.locations)
            && self.assessments.same(&other.assessments)
            && self.globals.same(&other.globals)
            && self.global_assessments.same(&other.global_assessments)
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<FieldLocationObservation>(),
        Relation::of::<FieldReadAssessment>(),
        Relation::of::<GlobalClassInspection>(),
        Relation::of::<GlobalFieldReadAssessment>(),
    ]
}
pub fn validation_inputs() -> Vec<ValidationInput> {
    vec![
        ValidationInput::of::<FieldLocationObservation>(&["id"]),
        ValidationInput::of::<FieldReadAssessment>(&["id"]),
        ValidationInput::of::<GlobalClassInspection>(&["id"]),
        ValidationInput::of::<GlobalFieldReadAssessment>(&["id"]),
    ]
}
fn same_context(
    entry: &EntryData,
    q: Id<AssertionQualification>,
    context: Id<AnalysisContext>,
) -> bool {
    entry
        .qualifications
        .get(q)
        .is_some_and(|q| q.context == context)
}
fn class_of_scope(
    data: &EvaluationData,
    entry: &EntryData,
    scope: Id<LexicalScope>,
    context: Id<AnalysisContext>,
) -> Option<(Id<ClassEntity>, Id<Occurrence>)> {
    let function = entry.lexical_scopes.get(scope)?.owner;
    let mut owners = entry
        .symbol_declarations
        .iter()
        .filter(|d| d.declaration == function && same_context(entry, d.qualification, context));
    let owner = owners.next()?;
    if owners.next().is_some() {
        return None;
    }
    let mut traits = data
        .function_traits
        .iter()
        .filter(|t| t.symbol == owner.symbol && same_context(entry, t.qualification, context));
    let first = traits.next()?;
    if traits.next().is_some() {
        return None;
    }
    let traits = first;
    if traits.staticmethod || traits.classmethod || traits.property_getter || traits.property_setter
    {
        return None;
    }
    let symbol = traits.defining_class?;
    let mut classes = entry
        .symbol_declarations
        .iter()
        .filter(|d| d.symbol == symbol && same_context(entry, d.qualification, context));
    let class = classes.next()?;
    if classes.next().is_some() {
        return None;
    }
    let id = ClassEntity::Source {
        declaration: class.declaration,
    }
    .id();
    data.classes.get(id)?;
    Some((id, function))
}
/// Exact complete source MRO plus canonical sequence membership, retaining report provenance.
/// Missing class shape cannot exclude a dynamic class from another class's source read screen.
type ChargedHierarchy = (
    Id<calls::ProviderSymbol>,
    charged::ChargedSet<Id<calls::ProviderSymbol>>,
    charged::StateCharge,
);

fn hierarchy(
    data: &EvaluationData,
    entry: &EntryData,
    class: Id<ClassEntity>,
    inv: &publication::AnalysisInvocation,
    budget: &resources::ResourceBudget,
    work: &mut Work,
) -> Result<Option<ChargedHierarchy>, ModelError> {
    work.scan(entry.symbol_declarations.len() + data.ancestry.len() + data.sequence_members.len())?;
    let Some(ClassEntity::Source { declaration }) = data.classes.get(class) else {
        return Ok(None);
    };
    let mut symbols = entry.symbol_declarations.iter().filter(|d| {
        d.declaration == *declaration && same_context(entry, d.qualification, inv.context)
    });
    let Some(symbol) = symbols.next() else {
        return Ok(None);
    };
    if symbols.next().is_some() {
        return Ok(None);
    };
    if declared_class_inspection(
        super::read_channels::NativeContext {
            data,
            entry,
            invocation: inv,
        },
        &entry.symbol_declaration_supports,
        symbol.id(),
        symbol.qualification,
        *declaration,
        work,
    )?
    .is_none()
    {
        return Ok(None);
    };
    let mut rows = data.ancestry.iter().filter(|a| {
        a.class == symbol.symbol
            && a.relation == AncestryRelation::Mro
            && same_context(entry, a.qualification, inv.context)
    });
    let Some(mro) = rows.next() else {
        return Ok(None);
    };
    if rows.next().is_some()
        || mro.linearization != Some(Linearization::Complete)
        || source_class_hierarchy(
            super::read_channels::NativeContext {
                data,
                entry,
                invocation: inv,
            },
            &data.ancestry_supports,
            mro.id(),
            mro.qualification,
            *declaration,
            work,
        )?
        .is_none()
    {
        return Ok(None);
    };
    let count = data
        .sequence_members
        .iter()
        .filter(|m| m.sequence == mro.ancestors)
        .count();
    if count > MAX_SEQUENCE_SYMBOLS {
        return Ok(None);
    };
    let _reservation = budget.reserve(
        "field-read-mro-sequence",
        count
            .saturating_mul(
                size_of::<SymbolSequenceMember>() * 2 + size_of::<Id<calls::ProviderSymbol>>() * 3,
            )
            .saturating_add(1024),
    )?;
    let mut members = data
        .sequence_members
        .iter()
        .filter(|m| m.sequence == mro.ancestors)
        .collect::<Vec<_>>();
    members.sort_by_key(|m| m.ordinal);
    if members
        .iter()
        .enumerate()
        .any(|(ordinal, m)| m.ordinal != ordinal as i64)
    {
        return Ok(None);
    };
    let ids = members.iter().map(|m| m.symbol).collect::<Vec<_>>();
    let (expected, _) = SymbolSequence::new(&ids)?;
    if data.sequences.get(mro.ancestors) != Some(&expected) {
        return Ok(None);
    };
    let mut charge = charged::StateCharge::new(budget, "field-read-class-closure");
    let mut ancestors = charged::ChargedSet::default();
    for id in ids {
        work.tick()?;
        let Some(s) = entry.symbols.get(id) else {
            return Ok(None);
        };
        if s.context != inv.context || s.kind != calls::SymbolKind::Class {
            return Ok(None);
        };
        if !ancestors.insert(&mut charge, id)? {
            return Ok(None);
        }
    }
    Ok(Some((symbol.symbol, ancestors, charge)))
}
fn receiver_sources(
    data: &EvaluationData,
    entry: &EntryData,
    function: Id<Occurrence>,
    class: Id<ClassEntity>,
    inv: &publication::AnalysisInvocation,
    work: &mut Work,
) -> Result<Option<ContentHash>, ModelError> {
    let mut digest = KeySink::new("source-field-receiver-declarations");
    work.scan(entry.symbol_declarations.len() + data.function_traits.len())?;
    let Some(owner) = entry
        .symbol_declarations
        .iter()
        .find(|d| d.declaration == function && same_context(entry, d.qualification, inv.context))
    else {
        return Ok(None);
    };
    let Some(traits) = data
        .function_traits
        .iter()
        .find(|t| t.symbol == owner.symbol && same_context(entry, t.qualification, inv.context))
    else {
        return Ok(None);
    };
    let Some(ClassEntity::Source { declaration }) = data.classes.get(class) else {
        return Ok(None);
    };
    let Some(class) = entry.symbol_declarations.iter().find(|d| {
        d.declaration == *declaration
            && Some(d.symbol) == traits.defining_class
            && same_context(entry, d.qualification, inv.context)
    }) else {
        return Ok(None);
    };
    for premise in [
        declared_class_inspection(
            super::read_channels::NativeContext {
                data,
                entry,
                invocation: inv,
            },
            &entry.symbol_declaration_supports,
            owner.id(),
            owner.qualification,
            function,
            work,
        )?,
        declared_class_inspection(
            super::read_channels::NativeContext {
                data,
                entry,
                invocation: inv,
            },
            &data.function_trait_supports,
            traits.id(),
            traits.qualification,
            function,
            work,
        )?,
        declared_class_inspection(
            super::read_channels::NativeContext {
                data,
                entry,
                invocation: inv,
            },
            &entry.symbol_declaration_supports,
            class.id(),
            class.qualification,
            *declaration,
            work,
        )?,
    ] {
        let Some(p) = premise else { return Ok(None) };
        p.0.encode(&mut digest);
    }
    Ok(Some(digest.finish()))
}
/// Compact complete-negative state for one actual Base invocation. Rich observations and
/// native reports stay with their selected kernel; only names, class/name keys and fixed-value
/// dynamic/global summaries survive a kernel boundary. Every retained value is charged.
pub struct FieldUniverse {
    charge: charged::StateCharge,
    names: charged::ChargedSet<String>,
    fields: charged::ChargedSet<(Id<ClassEntity>, String)>,
    attribute_ids: charged::ChargedSet<Id<super::read_channels::AttributeRead>>,
    digest: Option<KeySink>,
    universe: Option<ContentHash>,
    complete: bool,
    globals: Rows<GlobalClassInspection>,
    dynamic: Rows<super::read_dynamic::DynamicAccessObservation>,
}
impl FieldUniverse {
    pub fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            charge: charged::StateCharge::new(budget, "field-read-native-universe"),
            names: Default::default(),
            fields: Default::default(),
            attribute_ids: Default::default(),
            digest: Some(KeySink::new("complete-native-field-read-universe")),
            universe: None,
            complete: true,
            globals: Rows::new(budget),
            dynamic: Rows::new(budget),
        }
    }
    pub(super) fn remember_attribute(
        &mut self,
        row: &super::read_channels::AttributeRead,
        name: &str,
    ) -> Result<(), ModelError> {
        self.names.insert(&mut self.charge, name.to_owned())?;
        self.attribute_ids.insert(&mut self.charge, row.id())?;
        if row.premise.is_none() {
            self.complete = false;
        }
        Ok(())
    }
    pub(super) fn remember_dynamic(
        &mut self,
        row: &super::read_dynamic::DynamicAccessObservation,
    ) -> Result<(), ModelError> {
        self.dynamic.insert(row.clone())?;
        Ok(())
    }
    pub(super) fn dynamic_scope(&self) -> bool {
        self.dynamic.iter().any(|row| {
            matches!(
                row.kind,
                super::read_dynamic::DynamicKind::Exec | super::read_dynamic::DynamicKind::Eval
            )
        })
    }
    pub fn keys(&self) -> impl Iterator<Item = &(Id<ClassEntity>, String)> {
        self.fields.iter()
    }
    pub fn next_key(
        &self,
        after: Option<&(Id<ClassEntity>, String)>,
    ) -> Option<&(Id<ClassEntity>, String)> {
        match after {
            Some(after) => self
                .fields
                .range((std::ops::Bound::Excluded(after), std::ops::Bound::Unbounded))
                .next(),
            None => self.fields.iter().next(),
        }
    }
    pub fn dynamic_classes(&self) -> impl Iterator<Item = Id<ClassEntity>> + '_ {
        self.dynamic.iter().filter_map(|row| row.declared_class)
    }
    pub(super) fn roots(
        &mut self,
        _data: &EvaluationData,
        entry: &EntryData,
        inv: &publication::AnalysisInvocation,
        roots: &std::collections::BTreeSet<Id<SourceArtifact>>,
        work: &mut Work,
    ) -> Result<(), ModelError> {
        // Admission roots, not observed reads, define the complete source universe.
        for artifact in roots.iter().filter(|a| {
            entry.artifacts.get(**a).is_some_and(|a| {
                a.input == inv.input
                    && admission::ArtifactClass::of(&a.path)
                        == Some(admission::ArtifactClass::PythonSource)
            })
        }) {
            work.tick()?;
            artifact.encode(
                self.digest
                    .as_mut()
                    .ok_or(ModelError::Conflict("field universe already sealed"))?,
            );
            let coverages = entry.coverage.iter().filter(|c| {
                c.family == FactFamily::Flow
                    && c.context == inv.context
                    && input_scope(entry, c.scope, inv.input, *artifact)
            });
            if !super::read_channels::calls_available(entry, inv, *artifact) {
                self.complete = false
            }
            for c in entry.coverage.iter().filter(|c| {
                c.family == FactFamily::Calls
                    && c.context == inv.context
                    && input_scope(entry, c.scope, inv.input, *artifact)
            }) {
                c.id().encode(
                    self.digest
                        .as_mut()
                        .ok_or(ModelError::Conflict("field universe already sealed"))?,
                );
            }
            let mut found = false;
            for c in coverages {
                work.tick()?;
                c.id().encode(
                    self.digest
                        .as_mut()
                        .ok_or(ModelError::Conflict("field universe already sealed"))?,
                );
                found = true;
                if c.status != CoverageStatus::CompleteUnderStatedModel
                    || c.run.is_none_or(|run| {
                        entry
                            .runs
                            .get(run)
                            .is_none_or(|r| r.input != inv.input || r.context != inv.context)
                    })
                {
                    self.complete = false
                }
            }
            if !found {
                self.complete = false
            }
        }
        Ok(())
    }
    pub(super) fn attributes(&mut self, work: &mut Work) -> Result<(), ModelError> {
        let digest = self
            .digest
            .as_mut()
            .ok_or(ModelError::Conflict("field universe already sealed"))?;
        for id in self.attribute_ids.iter() {
            work.tick()?;
            id.encode(digest);
        }
        Ok(())
    }
    pub(super) fn calls(
        &mut self,
        data: &EvaluationData,
        entry: &EntryData,
        inv: &publication::AnalysisInvocation,
        roots: &std::collections::BTreeSet<Id<SourceArtifact>>,
        _out: &mut ReadRecords,
        _budget: &resources::ResourceBudget,
        work: &mut Work,
        selected_id: Option<Id<calls::CallSyntax>>,
    ) -> Result<(), ModelError> {
        for call in data
            .call_syntax
            .iter()
            .filter(|row| selected_id.is_none_or(|id| row.id() == id))
        {
            work.tick()?;
            if !selected(entry, inv, call.site, roots)
                || !same_context(entry, call.qualification, inv.context)
            {
                continue;
            }
            let mut builtin = None;
            let mut hinted = false;
            let mut conflicting = false;
            for resolution in data.lexical_resolutions.iter().filter(|r| {
                r.read == call.callee && same_context(entry, r.qualification, inv.context)
            }) {
                work.tick()?;
                let name = match data.lexical_targets.get(resolution.target) {
                    Some(LexicalTarget::Builtin {
                        name,
                        variable: false,
                    }) if name == "getattr" || name == "hasattr" => Some(name.as_str()),
                    _ => None,
                };
                hinted |= name.is_some();
                if native(
                    super::read_channels::NativeContext {
                        data,
                        entry,
                        invocation: inv,
                    },
                    &data.lexical_resolution_supports,
                    resolution.id(),
                    resolution.qualification,
                    call.callee,
                    work,
                )?
                .is_none()
                {
                    continue;
                }
                if let Some(name) = name {
                    if builtin.is_some_and(|prior| prior != name) {
                        conflicting = true;
                    }
                    builtin = Some(name);
                } else {
                    conflicting = true;
                }
            }
            let target = super::read_dynamic::native_name(data, entry, inv, call.site, work)?;
            let named_target = target.filter(|t| t.0 == "getattr" || t.0 == "hasattr");
            if !hinted && named_target.is_none() {
                continue;
            }
            if let (Some(builtin), Some(target)) = (builtin, named_target)
                && builtin != target.0
            {
                conflicting = true;
            }
            // Canonical lexical resolution and the native call target are independent
            // source-model premises. A recognizer builtin hint does not require a native
            // lexical twin when the supported call inventory already resolves the builtin.
            // A mixed inventory still refuses closure, even with a lexical builtin answer.
            let native_builtin = named_target.is_some_and(|target| target.2);
            conflicting |= named_target.is_some_and(|target| !target.2);
            if let Some(target) = named_target {
                target.1.encode(
                    self.digest
                        .as_mut()
                        .ok_or(ModelError::Conflict("field universe already sealed"))?,
                );
            }
            let arg = data.call_arguments.iter().find(|a| {
                a.call == call.id() && a.ordinal == 1 && a.kind == calls::ArgumentKind::Positional
            });
            let Some(detail) = arg.and_then(|a| {
                data.details.iter().find(|d| {
                    d.occurrence == a.value && same_context(entry, d.qualification, inv.context)
                })
            }) else {
                continue;
            };
            let Some(SyntaxDetail::Literal { literal }) = data.detail_values.get(detail.detail)
            else {
                continue;
            };
            let Some(Literal::String { value }) = data.literals.get(*literal) else {
                continue;
            };
            self.names
                .insert(&mut self.charge, value.as_str().to_owned())?;
            call.id().encode(
                self.digest
                    .as_mut()
                    .ok_or(ModelError::Conflict("field universe already sealed"))?,
            );
            detail.id().encode(
                self.digest
                    .as_mut()
                    .ok_or(ModelError::Conflict("field universe already sealed"))?,
            );
            if native(
                super::read_channels::NativeContext {
                    data,
                    entry,
                    invocation: inv,
                },
                &data.call_syntax_supports,
                call.id(),
                call.qualification,
                call.site,
                work,
            )?
            .is_none()
                || conflicting
                || (builtin.is_none() && !native_builtin)
            {
                self.complete = false
            }
        }
        Ok(())
    }
    pub(super) fn classes(
        &mut self,
        data: &EvaluationData,
        entry: &EntryData,
        inv: &publication::AnalysisInvocation,
        roots: &std::collections::BTreeSet<Id<SourceArtifact>>,
        out: &mut ReadRecords,
        _budget: &resources::ResourceBudget,
        work: &mut Work,
        selected_id: Option<Id<ClassFieldSyntaxObservation>>,
    ) -> Result<(), ModelError> {
        for row in data
            .class_fields
            .iter()
            .filter(|row| selected_id.is_none_or(|id| row.id() == id))
        {
            work.tick()?;
            if !same_context(entry, row.qualification, inv.context)
                || !selected(entry, inv, row.target, roots)
            {
                continue;
            }
            let class = ClassEntity::Source {
                declaration: row.class,
            }
            .id();
            if data.classes.get(class).is_none() {
                self.complete = false;
                continue;
            }
            let binding = data.binding_events.iter().find(|e| e.site == row.target);
            let Some(binding) = binding else {
                self.complete = false;
                continue;
            };
            let premise = native(
                super::read_channels::NativeContext {
                    data,
                    entry,
                    invocation: inv,
                },
                &data.class_field_supports,
                row.id(),
                row.qualification,
                row.target,
                work,
            )?
            .map(|p| p.0);
            if premise.is_none() {
                self.complete = false
            }
            let location = FieldLocationObservation {
                invocation: inv.id(),
                class,
                name: binding.name.clone(),
                site: row.target,
                kind: FieldLocationKind::ClassDeclaration,
                premise,
                qualification: row.qualification,
                sources: ContentHash::of(b"native-class-field-declaration"),
            };
            self.fields
                .insert(&mut self.charge, (class, binding.name.clone()))?;
            location.id().encode(
                self.digest
                    .as_mut()
                    .ok_or(ModelError::Conflict("field universe already sealed"))?,
            );
            out.fields.locations.insert(location)?;
        }
        Ok(())
    }
    pub(super) fn stores(
        &mut self,
        data: &EvaluationData,
        entry: &EntryData,
        inv: &publication::AnalysisInvocation,
        roots: &std::collections::BTreeSet<Id<SourceArtifact>>,
        out: &mut ReadRecords,
        _budget: &resources::ResourceBudget,
        work: &mut Work,
        selected_id: Option<Id<flow::FlowDefinitionObservation>>,
    ) -> Result<(), ModelError> {
        for row in entry
            .definition_observations
            .iter()
            .filter(|row| selected_id.is_none_or(|id| row.id() == id))
        {
            work.tick()?;
            if !same_context(entry, row.qualification, inv.context) {
                continue;
            }
            let Some(def) = entry.definitions.get(row.definition) else {
                self.complete = false;
                continue;
            };
            if !selected(entry, inv, def.occurrence, roots) {
                continue;
            }
            let Some(place) = entry.places.get(def.place) else {
                self.complete = false;
                continue;
            };
            let Some(path) = entry.paths.get(place.path) else {
                self.complete = false;
                continue;
            };
            let Some(PathSegment::Attribute { name }) =
                path.first.and_then(|s| data.segments.get(s))
            else {
                continue;
            };
            if path.second.is_some() || path.unknown_suffix {
                continue;
            }
            work.scan(entry.symbol_declarations.len() + data.function_traits.len())?;
            let Some((class, function)) = class_of_scope(data, entry, row.scope, inv.context)
            else {
                continue;
            };
            let Some(root) = entry.roots.get(place.root) else {
                self.complete = false;
                continue;
            };
            let receiver=match root{PlaceRoot::Receiver{callable}=>*callable==function,PlaceRoot::Formal{declaration}=>entry.links.iter().any(|l|matches!(entry.formals.get(l.entity),Some(ParameterEntity::Source{declaration:p})if p==declaration)&&entry.parameters.get(l.parameter).is_some_and(|p|p.ordinal==0&&entry.signatures.get(p.signature).is_some_and(|s|s.role.runtime_source()))),PlaceRoot::Local{scope,name}if *scope==function=>entry.signatures.iter().filter(|s|s.role.runtime_source()&&entry.symbol_declarations.iter().any(|d|d.symbol==s.symbol&&d.declaration==function)).any(|s|entry.parameters.iter().any(|p|p.signature==s.id()&&p.ordinal==0&&data.parameter_shapes.get(p.shape).is_some_and(|shape|shape.name.as_ref().is_some_and(|n|n.as_str()==name)))),_=>false};
            if !receiver {
                continue;
            }
            let sources = receiver_sources(data, entry, function, class, inv, work)?;
            if sources.is_none() {
                self.complete = false
            }
            let premise = native(
                super::read_channels::NativeContext {
                    data,
                    entry,
                    invocation: inv,
                },
                &entry.definition_supports,
                row.id(),
                row.qualification,
                def.occurrence,
                work,
            )?
            .map(|p| p.0);
            if premise.is_none() {
                self.complete = false
            }
            let location = FieldLocationObservation {
                invocation: inv.id(),
                class,
                name: name.clone(),
                site: def.occurrence,
                kind: FieldLocationKind::ReceiverStore,
                premise,
                qualification: row.qualification,
                sources: sources
                    .unwrap_or_else(|| ContentHash::of(b"unavailable-source-field-receiver")),
            };
            self.fields
                .insert(&mut self.charge, (class, name.clone()))?;
            location.id().encode(
                self.digest
                    .as_mut()
                    .ok_or(ModelError::Conflict("field universe already sealed"))?,
            );
            out.fields.locations.insert(location)?;
        }
        Ok(())
    }
    pub(super) fn globals(
        &mut self,
        data: &EvaluationData,
        entry: &EntryData,
        inv: &publication::AnalysisInvocation,
        roots: &std::collections::BTreeSet<Id<SourceArtifact>>,
        out: &mut ReadRecords,
        budget: &resources::ResourceBudget,
        work: &mut Work,
        selected_id: Option<Id<BindingObservation>>,
        actual: Option<super::read_channels::ReadEntries<'_>>,
    ) -> Result<(), ModelError> {
        for binding in data
            .bindings
            .iter()
            .filter(|row| selected_id.is_none_or(|id| row.id() == id))
        {
            work.tick()?;
            if !same_context(entry, binding.qualification, inv.context) {
                continue;
            }
            let Some(event) = data.binding_events.get(binding.event) else {
                continue;
            };
            if !selected(entry, inv, event.site, roots) {
                continue;
            }
            if let Some((class, sources, status)) = super::read_dynamic::global_candidate(
                data, entry, inv, binding, budget, work, actual,
            )? {
                let row = GlobalClassInspection {
                    invocation: inv.id(),
                    binding: event.id(),
                    class,
                    status,
                    sources,
                };
                row.id().encode(
                    self.digest
                        .as_mut()
                        .ok_or(ModelError::Conflict("field universe already sealed"))?,
                );
                self.globals.insert(row.clone())?;
                out.fields.globals.insert(row)?;
            }
        }
        Ok(())
    }
    pub(super) fn seal(&mut self, work: &mut Work) -> Result<(), ModelError> {
        let mut digest = self
            .digest
            .take()
            .ok_or(ModelError::Conflict("field universe already sealed"))?;
        for row in self.dynamic.iter() {
            work.tick()?;
            row.id().encode(&mut digest);
        }
        self.universe = Some(digest.finish());
        Ok(())
    }
    pub(super) fn assess(
        &self,
        data: &EvaluationData,
        entry: &EntryData,
        inv: &publication::AnalysisInvocation,
        class: &Id<ClassEntity>,
        name: &String,
        out: &mut ReadRecords,
        budget: &resources::ResourceBudget,
        work: &mut Work,
    ) -> Result<(), ModelError> {
        let _key = budget.reserve("field-assessment-key", name.len())?;
        if !self.fields.contains(&(*class, name.clone())) {
            return Err(ModelError::Conflict("field key was not produced"));
        }
        let universe = self
            .universe
            .ok_or(ModelError::Conflict("field universe is not sealed"))?;
        {
            work.tick()?;
            let (status, reason) = if self.names.contains(name) {
                (ReadAssessment::ObservedRead, None)
            } else if !self.complete {
                (
                    ReadAssessment::Unknown,
                    Some(obligation::ObligationKind::IncompleteCoverage),
                )
            } else {
                let mut dynamic = false;
                let mut missing = false;
                for d in self.dynamic.iter() {
                    work.tick()?;
                    if matches!(
                        d.kind,
                        super::read_dynamic::DynamicKind::DunderImport
                            | super::read_dynamic::DynamicKind::ImportModule
                    ) {
                        continue;
                    }
                    if let Some(other) = d.declared_class {
                        if other == *class {
                            dynamic = true;
                            continue;
                        }
                        match (
                            hierarchy(data, entry, *class, inv, budget, work)?,
                            hierarchy(data, entry, other, inv, budget, work)?,
                        ) {
                            (Some((a, aa, _)), Some((b, bb, _))) => {
                                if aa.contains(&b) || bb.contains(&a) {
                                    dynamic = true
                                }
                            }
                            _ => missing = true,
                        }
                    } else {
                        dynamic = true
                    }
                }
                if dynamic {
                    (
                        ReadAssessment::Unknown,
                        Some(obligation::ObligationKind::DynamicAccess),
                    )
                } else if missing {
                    (
                        ReadAssessment::Unknown,
                        Some(obligation::ObligationKind::MissingEvidence),
                    )
                } else {
                    (ReadAssessment::CompleteNoReadUnderModel, None)
                }
            };
            let row = FieldReadAssessment {
                invocation: inv.id(),
                class: *class,
                name: name.clone(),
                status,
                reason,
                universe,
            };
            out.fields.assessments.insert(row)?;
            for global in self.globals.iter().filter(|g| g.class == *class) {
                out.fields
                    .global_assessments
                    .insert(GlobalFieldReadAssessment {
                        global: global.id(),
                        name: name.clone(),
                        status,
                        reason,
                        universe,
                    })?;
            }
        }
        Ok(())
    }
}
pub(super) fn produce(
    data: &EvaluationData,
    entry: &EntryData,
    inv: &publication::AnalysisInvocation,
    roots: &std::collections::BTreeSet<Id<SourceArtifact>>,
    out: &mut ReadRecords,
    budget: &resources::ResourceBudget,
    work: &mut Work,
    actual: Option<super::read_channels::ReadEntries<'_>>,
) -> Result<(), ModelError> {
    let mut universe = FieldUniverse::new(budget);
    for row in out.attributes.iter() {
        let native = data
            .attribute_loads
            .get(row.observation)
            .ok_or_else(|| ModelError::Invalid("field read observation missing".into()))?;
        universe.remember_attribute(row, &native.name)?;
    }
    for row in out.dynamic.iter() {
        universe.remember_dynamic(row)?;
    }
    universe.roots(data, entry, inv, roots, work)?;
    universe.attributes(work)?;
    universe.calls(data, entry, inv, roots, out, budget, work, None)?;
    universe.classes(data, entry, inv, roots, out, budget, work, None)?;
    universe.stores(data, entry, inv, roots, out, budget, work, None)?;
    universe.globals(data, entry, inv, roots, out, budget, work, None, actual)?;
    universe.seal(work)?;
    // Iterate compact field keys without retaining any rich kernel input or output.
    for (class, name) in universe.fields.iter() {
        universe.assess(data, entry, inv, class, name, out, budget, work)?;
    }
    Ok(())
}
