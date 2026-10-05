//! N2 keeps every originating observation, including empty and ambiguous correspondences.
use super::{
    Rows,
    entities::*,
    entity_normalization::{EntityData, EntityOutput},
    links::*,
    policy_revision,
};
use crate::domain::{
    attribution::*,
    calls::*,
    charged::{ChargedMap, ChargedSet, StateCharge},
    lexical::*,
    resources::ResourceBudget,
    source::*,
    symbols::*,
    types::*,
    value::*,
    *,
};
fn entity_stage_inputs() -> Vec<stages::RelationUse> {
    macro_rules! declare { ($($name:ident: $ty:ty,)*) => { vec![$(stages::RelationUse::stored::<$ty>()),*] }; }
    crate::normalized_entity_outputs!(declare)
}
macro_rules! inputs {
    ($($field:ident: $ty:ty => $family:ident,)*) => {
        pub struct RelationData { pub facts: EntityData, pub entities: EntityOutput, $(pub $field: Rows<$ty>,)* }
        impl RelationData {
            pub fn new(budget: &ResourceBudget) -> Self { Self { facts: EntityData::new(budget), entities: EntityOutput::new(budget), $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                let facts = self.facts.visit(relation, batch)?;
                let entities = self.entities.visit(relation, batch)?;
                let mut matched = facts || entities;
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; matched = true; })*
                Ok(matched)
            }
            pub fn validation_inputs() -> Vec<ValidationInput> {
                let mut inputs = EntityData::validation_inputs(); inputs.extend(EntityOutput::validation_inputs());
                inputs.extend([$(ValidationInput::of::<$ty>(&["id"]),)*]); super::facts_inputs(inputs)
            }
            pub fn stage_inputs() -> Vec<stages::RelationUse> {
                let mut inputs = EntityData::stage_inputs();
                inputs.extend(entity_stage_inputs());
                inputs.extend([$(stages::RelationUse::stored::<$ty>().availability(FactFamily::$family, stages::AvailabilityPolicy::ObserveAvailability),)*]); inputs
            }
        }
    }
}
crate::normalized_relation_inputs!(inputs);
macro_rules! outputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct RelationOutput { $(pub $field: Rows<$ty>,)* }
        impl RelationOutput {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if relation == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn matches(&self, expected: &Self) -> Result<(), ModelError> {
                $(if !self.$field.same(&expected.$field) { return Err(invalid(format!("normalized relationship closure differs: {}", <$ty>::NAME))); })* Ok(())
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
        }
    }
}
crate::normalized_relation_outputs!(outputs);
fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("normalized relation requires {}", R::NAME)))
}
fn context(
    data: &RelationData,
    qualification: Id<assertion::AssertionQualification>,
) -> Result<Id<AnalysisContext>, ModelError> {
    Ok(need(&data.facts.qualifications, qualification)?.context)
}
fn input(
    data: &RelationData,
    qualification: Id<assertion::AssertionQualification>,
) -> Result<Id<input::InputRevision>, ModelError> {
    match need(
        &data.scopes,
        need(&data.facts.qualifications, qualification)?.scope,
    )? {
        CoverageScope::Input { input } => Ok(*input),
        CoverageScope::Artifact { artifact } => Ok(need(&data.artifacts, *artifact)?.input),
        CoverageScope::Module { module } => {
            Ok(need(&data.artifacts, need(&data.facts.modules, *module)?.source)?.input)
        }
        CoverageScope::Release { .. } => Err(invalid(
            "normalized observation requires captured input scope",
        )),
    }
}
fn decision(count: usize, incomplete: bool) -> (ResolutionStatus, LinkReason) {
    if incomplete || count == 0 {
        (
            ResolutionStatus::Unresolved,
            LinkReason::MissingCorrespondence,
        )
    } else if count == 1 {
        (ResolutionStatus::Resolved, LinkReason::ExplicitIdentity)
    } else {
        (
            ResolutionStatus::Ambiguous,
            LinkReason::ConflictingCandidates,
        )
    }
}
/// Prefer a declared entity at this exact occurrence; otherwise retain the occurrence itself.
/// No ancestor/name inference substitutes a value for the binding event.
fn source_entity(
    data: &RelationData,
    occurrence: Id<Occurrence>,
) -> Result<Id<EntityRef>, ModelError> {
    let row = need(&data.facts.occurrences, occurrence)?;
    let candidates = [
        super::entities::source_callable(row).map(|c| EntityRef::Callable { callable: c.id() }),
        Some(EntityRef::Class {
            class: ClassEntity::Source {
                declaration: occurrence,
            }
            .id(),
        }),
        Some(EntityRef::Parameter {
            parameter: ParameterEntity::Source {
                declaration: occurrence,
            }
            .id(),
        }),
    ];
    for candidate in candidates.into_iter().flatten() {
        if data.entities.refs.get(candidate.id()).is_some() {
            return Ok(candidate.id());
        }
    }
    let entity = EntityRef::Occurrence { occurrence }.id();
    need(&data.entities.refs, entity)?;
    Ok(entity)
}
struct Index<'a> {
    resolutions: ChargedMap<Id<ProviderSymbol>, &'a SymbolEntityResolution>,
    modules: ChargedMap<Id<ProviderModule>, String>,
    _charge: StateCharge,
}
impl<'a> Index<'a> {
    fn new(data: &'a RelationData, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut charge = StateCharge::new(budget, "normalized-relationship-index");
        let mut resolutions = ChargedMap::default();
        for row in data.entities.resolutions.iter() {
            resolutions.insert(&mut charge, row.symbol, row)?;
        }
        let mut modules = ChargedMap::default();
        for row in data.facts.provider_modules.iter() {
            let name = match row {
                ProviderModule::Acquired { module } => {
                    need(&data.facts.modules, *module)?.qualified_name.clone()
                }
                ProviderModule::Bundled { name, .. }
                | ProviderModule::Namespace { name, .. }
                | ProviderModule::Unresolved { name, .. } => name.clone(),
            };
            modules.insert(&mut charge, row.id(), name)?;
        }
        Ok(Self {
            resolutions,
            modules,
            _charge: charge,
        })
    }
    fn resolution(
        &self,
        symbol: Id<ProviderSymbol>,
    ) -> Result<&'a SymbolEntityResolution, ModelError> {
        self.resolutions
            .get(&symbol)
            .copied()
            .ok_or_else(|| invalid("missing total symbol resolution"))
    }
}
pub fn normalize(
    data: &RelationData,
    budget: &ResourceBudget,
) -> Result<RelationOutput, ModelError> {
    let mut output = RelationOutput::new(budget);
    let index = Index::new(data, budget)?;
    references(data, &mut output, budget)?;
    super::native_lexical::characterize(data, &mut output, budget)?;
    imports(data, &index, &mut output, budget)?;
    ancestry(data, &index, &mut output, budget)?;
    mentions(data, &index, &mut output, budget)?;
    types(data, &index, &mut output)?;
    binders(data, &mut output, budget)?;
    places(data, &mut output)?;
    test_operands(data, &mut output, budget)?;
    Ok(output)
}
fn references(
    data: &RelationData,
    output: &mut RelationOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "reference-index");
    let mut resolutions: ChargedMap<
        (Id<AnalysisContext>, Id<Occurrence>),
        Vec<&LexicalResolution>,
    > = Default::default();
    for row in data.lexical_resolutions.iter() {
        resolutions.update(
            &mut charge,
            (context(data, row.qualification)?, row.read),
            |rows| rows.push(row),
        )?;
    }
    for reference in data.references.iter() {
        let rows = resolutions.get(&(context(data, reference.qualification)?, reference.read));
        let mut candidates = Vec::new();
        let mut held = StateCharge::new(budget, "reference-candidates");
        let mut distinct: ChargedSet<Id<ReferenceEntityTarget>> = Default::default();
        let mut unresolved = false;
        for row in rows.into_iter().flatten() {
            let target = match need(&data.lexical_targets, row.target)? {
                LexicalTarget::Binding { event } => ReferenceEntityTarget::Binding {
                    event: *event,
                    entity: source_entity(data, need(&data.facts.bindings, *event)?.site)?,
                },
                LexicalTarget::Builtin { .. } => {
                    ReferenceEntityTarget::Builtin { target: row.target }
                }
                LexicalTarget::Unresolved { .. } => {
                    unresolved = true;
                    ReferenceEntityTarget::Unresolved { target: row.target }
                }
            };
            let target = output.reference_targets.insert(target)?;
            distinct.insert(&mut held, target)?;
            held.admit(&(row.id(), target))?;
            candidates.push((row.id(), target));
        }
        let (status, mut reason) = decision(distinct.len(), unresolved);
        if rows.is_none() {
            reason = LinkReason::MissingResolution;
        }
        let assessment = output
            .reference_entity_assessments
            .insert(ReferenceEntityAssessment {
                reference: reference.id(),
                status,
                reason,
            })?;
        for (resolution, target) in candidates {
            output
                .reference_entity_candidates
                .insert(ReferenceEntityCandidate {
                    assessment,
                    resolution,
                    target,
                })?;
        }
    }
    Ok(())
}
type QualifiedPath = (Id<AnalysisContext>, Id<input::InputRevision>, String);
type QualifiedImportAlias = (
    Id<AnalysisContext>,
    Id<input::InputRevision>,
    Id<Occurrence>,
);
fn imports(
    data: &RelationData,
    _index: &Index<'_>,
    output: &mut RelationOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "import-resolution-index");
    let mut resolutions: ChargedMap<QualifiedImportAlias, Vec<&ModuleResolutionObservation>> =
        Default::default();
    for row in data.module_resolutions.iter() {
        need(&data.facts.provider_modules, row.module)?;
        if let Some(alias) = row.alias {
            let alias_source = need(&data.facts.occurrences, alias)?;
            if alias_source.syntax_kind != SyntaxKind::Alias {
                return Err(invalid(
                    "module resolution alias is not an import alias occurrence",
                ));
            }
            resolutions.update(
                &mut charge,
                (
                    context(data, row.qualification)?,
                    input(data, row.qualification)?,
                    alias,
                ),
                |rows| rows.push(row),
            )?;
        }
    }
    for import in data.imports.iter() {
        let mut unique: ChargedSet<Id<ProviderModule>> = Default::default();
        let mut held = StateCharge::new(budget, "import-candidates");
        let context = context(data, import.qualification)?;
        let input = input(data, import.qualification)?;
        let matches = resolutions.get(&(context, input, import.alias));
        let mut unresolved = false;
        for candidate in matches.into_iter().flatten() {
            unique.insert(&mut held, candidate.module)?;
            if matches!(
                need(&data.facts.provider_modules, candidate.module)?,
                ProviderModule::Unresolved { .. }
            ) {
                unresolved = true;
            }
        }
        let (status, reason) = decision(unique.len(), unresolved);
        let assessment = output
            .import_module_assessments
            .insert(ImportModuleAssessment {
                observation: import.id(),
                status,
                reason,
            })?;
        for observation in matches.into_iter().flatten() {
            output
                .import_module_candidates
                .insert(ImportModuleCandidate {
                    assessment,
                    observation: observation.id(),
                    module: observation.module,
                })?;
        }
    }
    Ok(())
}
fn ancestry(
    data: &RelationData,
    index: &Index<'_>,
    output: &mut RelationOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "ancestry-index");
    let mut sequences: ChargedMap<Id<SymbolSequence>, Vec<&SymbolSequenceMember>> =
        Default::default();
    for row in data.sequence_members.iter() {
        sequences.update(&mut charge, row.sequence, |rows| rows.push(row))?;
    }
    for observation in data.ancestry.iter() {
        let class = index.resolution(observation.class)?;
        let members = sequences.get(&observation.ancestors);
        let mut status = class.status;
        for member in members.into_iter().flatten() {
            match index.resolution(member.symbol)?.status {
                ResolutionStatus::Unresolved => status = ResolutionStatus::Unresolved,
                ResolutionStatus::Ambiguous if status != ResolutionStatus::Unresolved => {
                    status = ResolutionStatus::Ambiguous
                }
                _ => {}
            }
        }
        // Structural correspondence does not promote Prefix/Cyclic into complete ancestry.
        let reason = if observation
            .linearization
            .is_some_and(|l| l != Linearization::Complete)
        {
            LinkReason::IncompleteInput
        } else {
            match status {
                ResolutionStatus::Resolved => LinkReason::ExplicitIdentity,
                ResolutionStatus::Ambiguous => LinkReason::ConflictingCandidates,
                ResolutionStatus::Unresolved => LinkReason::MissingCorrespondence,
            }
        };
        let assessment = output
            .ancestry_entity_assessments
            .insert(AncestryEntityAssessment {
                observation: observation.id(),
                class: class.id(),
                status,
                reason,
            })?;
        for member in members.into_iter().flatten() {
            output
                .ancestry_entity_members
                .insert(AncestryEntityMember {
                    assessment,
                    member: member.id(),
                    resolution: index.resolution(member.symbol)?.id(),
                })?;
        }
    }
    Ok(())
}
fn mentions(
    data: &RelationData,
    index: &Index<'_>,
    output: &mut RelationOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "mention-exposure-index");
    let mut exposures: ChargedMap<QualifiedPath, Vec<&PublicExposure>> = Default::default();
    for exposure in data.entities.exposures.iter() {
        let module = need(&data.facts.modules, exposure.access)?;
        let name = need(&data.facts.public_names, exposure.observation)?;
        exposures.update(
            &mut charge,
            (
                exposure.context,
                need(&data.artifacts, module.source)?.input,
                format!("{}.{}", module.qualified_name, name.name),
            ),
            |rows| rows.push(exposure),
        )?;
    }
    let mut exposure_members: ChargedMap<Id<PublicExposure>, Vec<Id<SymbolEntityResolution>>> =
        Default::default();
    let mut resolved_members: ChargedMap<Id<SymbolEntityResolution>, Vec<Id<EntityRef>>> =
        Default::default();
    for row in data.entities.exposure_candidates.iter() {
        exposure_members.update(&mut charge, row.exposure, |rows| rows.push(row.resolution))?;
    }
    for row in data.entities.candidates.iter() {
        resolved_members.update(&mut charge, row.resolution, |rows| rows.push(row.entity))?;
    }
    let mut observations: ChargedMap<Id<ProviderSymbol>, Vec<&SymbolObservation>> =
        Default::default();
    for row in data.symbol_observations.iter() {
        observations.update(&mut charge, row.symbol, |rows| rows.push(row))?;
    }
    let mut qualified: ChargedMap<QualifiedPath, Vec<&SymbolObservation>> = Default::default();
    for row in data.symbol_observations.iter() {
        let symbol = need(&data.facts.symbols, row.symbol)?;
        let mut held = StateCharge::new(budget, "mention-qualified-path");
        let mut names = Vec::new();
        let mut current = Some(row);
        let mut uncertain = false;
        while let Some(next) = current {
            let symbol = need(&data.facts.symbols, next.symbol)?;
            held.admit(&symbol.name)?;
            names.push(symbol.name.clone());
            if names.len() > crate::domain::symbols::MAX_SYMBOL_NESTING {
                return Err(invalid("qualified mention path exceeds validated nesting"));
            }
            current = match next.parent {
                Some(parent) => match observations.get(&parent) {
                    Some(rows) if rows.len() == 1 => Some(rows[0]),
                    _ => {
                        uncertain = true;
                        None
                    }
                },
                None => None,
            };
        }
        if uncertain {
            continue;
        }
        names.reverse();
        let module = index
            .modules
            .get(&symbol.module)
            .ok_or_else(|| invalid("qualified symbol module missing"))?;
        qualified.update(
            &mut charge,
            (
                symbol.context,
                input(data, row.qualification)?,
                format!("{module}.{}", names.join(".")),
            ),
            |rows| rows.push(row),
        )?;
    }
    for mention in data.mentions.iter() {
        let mut held = StateCharge::new(budget, "mention-candidates");
        let mut candidates: ChargedMap<Id<PublicExposure>, &PublicExposure> = Default::default();
        let context = context(data, mention.qualification)?;
        let input = input(data, mention.qualification)?;
        for spelling in [&mention.access_path, &mention.qualified_name]
            .into_iter()
            .flatten()
        {
            for exposure in exposures
                .get(&(context, input, spelling.clone()))
                .into_iter()
                .flatten()
            {
                candidates.insert(&mut held, exposure.id(), exposure)?;
            }
        }
        let mut entities: ChargedSet<Id<EntityRef>> = Default::default();
        let mut unresolved = false;
        for exposure in candidates.values() {
            if exposure.status == ResolutionStatus::Unresolved {
                unresolved = true;
            }
            for resolution in exposure_members.get(&exposure.id()).into_iter().flatten() {
                for entity in resolved_members.get(resolution).into_iter().flatten() {
                    entities.insert(&mut held, *entity)?;
                }
            }
        }
        let mut symbol_candidates: ChargedMap<Id<SymbolObservation>, &SymbolObservation> =
            Default::default();
        if let Some(name) = &mention.qualified_name {
            for row in qualified
                .get(&(context, input, name.clone()))
                .into_iter()
                .flatten()
            {
                symbol_candidates.insert(&mut held, row.id(), row)?;
                let resolution = index.resolution(row.symbol)?;
                if resolution.status == ResolutionStatus::Unresolved {
                    unresolved = true;
                }
                for entity in resolved_members.get(&resolution.id()).into_iter().flatten() {
                    entities.insert(&mut held, *entity)?;
                }
            }
        }
        let (status, mut reason) = decision(entities.len(), unresolved);
        if mention.access_path.is_none() && mention.qualified_name.is_none() {
            reason = LinkReason::UntracedName;
        }
        let assessment = output
            .mention_entity_assessments
            .insert(MentionEntityAssessment {
                observation: mention.id(),
                status,
                reason,
            })?;
        for exposure in candidates.keys() {
            output
                .mention_entity_candidates
                .insert(MentionEntityCandidate {
                    assessment,
                    exposure: *exposure,
                })?;
        }
        for observation in symbol_candidates.values() {
            output
                .mention_symbol_candidates
                .insert(MentionSymbolCandidate {
                    assessment,
                    observation: observation.id(),
                    resolution: index.resolution(observation.symbol)?.id(),
                })?;
        }
    }
    Ok(())
}
fn types(
    data: &RelationData,
    index: &Index<'_>,
    output: &mut RelationOutput,
) -> Result<(), ModelError> {
    for term in data.facts.terms.iter() {
        let symbol = match term {
            TypeTerm::ClassInstance { class, .. }
            | TypeTerm::ClassObject { class }
            | TypeTerm::TypedDict { class, .. }
            | TypeTerm::SelfType { class, .. }
            | TypeTerm::EnumLiteral { class, .. } => Some(*class),
            TypeTerm::Callable { function, .. } => *function,
            TypeTerm::Overload { function, .. } => Some(*function),
            _ => None,
        };
        if let Some(symbol) = symbol {
            let resolution = index.resolution(symbol)?;
            output.type_entity_links.insert(TypeEntityLink {
                term: term.id(),
                resolution: Some(resolution.id()),
                module: None,
                entity: resolution.entity,
                status: resolution.status,
                reason: match resolution.status {
                    ResolutionStatus::Resolved => LinkReason::ExplicitIdentity,
                    ResolutionStatus::Ambiguous => LinkReason::ConflictingCandidates,
                    ResolutionStatus::Unresolved => LinkReason::MissingCorrespondence,
                },
            })?;
        } else if let TypeTerm::Module { module } | TypeTerm::TypeAliasReference { module, .. } =
            term
        {
            let entity = if matches!(term, TypeTerm::Module { .. }) {
                match need(&data.facts.provider_modules, *module)? {
                    ProviderModule::Acquired { module } => {
                        Some(EntityRef::Module { module: *module }.id())
                    }
                    _ => None,
                }
            } else {
                None
            };
            let (status, reason) = decision(usize::from(entity.is_some()), false);
            output.type_entity_links.insert(TypeEntityLink {
                term: term.id(),
                resolution: None,
                module: Some(*module),
                entity,
                status,
                reason,
            })?;
        }
    }
    Ok(())
}
fn binders(
    data: &RelationData,
    output: &mut RelationOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "type-binder-index");
    let mut by_source: ChargedMap<Id<SourceArtifact>, Vec<&Occurrence>> = Default::default();
    let mut by_path: ChargedMap<(Id<SourceArtifact>, Vec<i32>), &Occurrence> = Default::default();
    for row in data.facts.occurrences.iter() {
        by_source.update(&mut charge, row.source, |rows| rows.push(row))?;
        by_path.insert(&mut charge, (row.source, row.structural_path.clone()), row)?;
    }
    let mut supported: ChargedMap<(Id<AnalysisContext>, Id<Occurrence>), Vec<TypeBinderPremise>> =
        Default::default();
    for row in data.declaration_syntax.iter() {
        let occurrence = need(&data.facts.occurrences, row.declaration)?;
        if matches!(
            occurrence.syntax_kind,
            SyntaxKind::StmtFunctionDef | SyntaxKind::StmtClassDef | SyntaxKind::StmtTypeAlias
        ) {
            supported.update(
                &mut charge,
                (context(data, row.qualification)?, occurrence.id()),
                |rows| {
                    rows.push(TypeBinderPremise::Declaration {
                        observation: row.id(),
                    })
                },
            )?;
        }
    }
    for row in data.binding_observations.iter() {
        if !matches!(
            row.kind,
            BindingEventKind::Assignment
                | BindingEventKind::AnnotationOnly
                | BindingEventKind::TypeAlias
                | BindingEventKind::TypeParam
        ) {
            continue;
        }
        let site = need(
            &data.facts.occurrences,
            need(&data.facts.bindings, row.event)?.site,
        )?;
        let mut held = StateCharge::new(budget, "binder-ancestor-path");
        held.admit(&site.structural_path)?;
        let mut path = site.structural_path.clone();
        while !path.is_empty() {
            let occurrence = by_path
                .get(&(site.source, path.clone()))
                .ok_or_else(|| invalid("binder ancestor missing"))?;
            if matches!(
                occurrence.syntax_kind,
                SyntaxKind::StmtAssign
                    | SyntaxKind::StmtAnnAssign
                    | SyntaxKind::StmtTypeAlias
                    | SyntaxKind::StmtFunctionDef
                    | SyntaxKind::StmtClassDef
            ) {
                supported.update(
                    &mut charge,
                    (context(data, row.qualification)?, occurrence.id()),
                    |rows| {
                        rows.push(TypeBinderPremise::Binding {
                            observation: row.id(),
                        })
                    },
                )?;
                break;
            }
            path.pop();
        }
    }
    for variable in data.variables.iter() {
        let mut held = StateCharge::new(budget, "type-binder-candidates");
        let mut candidates: ChargedMap<Id<Occurrence>, &Vec<TypeBinderPremise>> =
            Default::default();
        let mut reason = LinkReason::MissingCorrespondence;
        let mut best_depth = None;
        if !matches!(
            variable.origin,
            TypeVariableOrigin::ScopedLegacy | TypeVariableOrigin::Pep695
        ) {
            reason = LinkReason::UnsupportedNativeOrigin;
        } else if let ProviderModule::Acquired { module } =
            need(&data.facts.provider_modules, variable.module)?
        {
            let source = need(&data.facts.modules, *module)?.source;
            let artifact = need(&data.artifacts, source)?;
            if variable.anchor_end > artifact.byte_len {
                reason = LinkReason::OutsideCapturedScope;
            } else {
                for occurrence in by_source.get(&source).into_iter().flatten() {
                    if occurrence.start > variable.anchor_start
                        || occurrence.end < variable.anchor_end
                    {
                        continue;
                    }
                    let Some(premises) = supported.get(&(variable.context, occurrence.id())) else {
                        continue;
                    };
                    let depth = occurrence.structural_path.len();
                    if best_depth.is_none_or(|best| depth > best) {
                        candidates = Default::default();
                        held = StateCharge::new(budget, "type-binder-candidates");
                        best_depth = Some(depth);
                    }
                    if best_depth == Some(depth) {
                        candidates.insert(&mut held, occurrence.id(), premises)?;
                    }
                }
            }
        } else {
            reason = LinkReason::OutsideCapturedScope;
        }
        let (status, selected_reason) = decision(candidates.len(), false);
        if !candidates.is_empty() {
            reason = selected_reason;
        }
        let assessment = output
            .type_binder_assessments
            .insert(TypeBinderAssessment {
                variable: variable.id(),
                status,
                reason,
            })?;
        for (declaration, premises) in candidates.iter() {
            for premise in *premises {
                let premise = output.type_binder_premises.insert(premise.clone())?;
                output.type_binder_candidates.insert(TypeBinderCandidate {
                    assessment,
                    declaration: *declaration,
                    premise,
                })?;
            }
        }
    }
    Ok(())
}
fn places(data: &RelationData, output: &mut RelationOutput) -> Result<(), ModelError> {
    for place in data.facts.places.iter() {
        let entity = match need(&data.roots, place.root)? {
            PlaceRoot::Formal { declaration } | PlaceRoot::Entry { declaration } => {
                let entity = EntityRef::Parameter {
                    parameter: ParameterEntity::Source {
                        declaration: *declaration,
                    }
                    .id(),
                };
                data.entities.refs.get(entity.id()).map(Record::id)
            }
            PlaceRoot::Receiver { callable }
            | PlaceRoot::Return { callable }
            | PlaceRoot::Yield { callable }
            | PlaceRoot::Raise { callable } => {
                let occurrence = need(&data.facts.occurrences, *callable)?;
                super::entities::source_callable(occurrence)
                    .map(|c| EntityRef::Callable { callable: c.id() }.id())
                    .filter(|id| data.entities.refs.get(*id).is_some())
            }
            PlaceRoot::Field { class, name } => {
                let field = FieldEntity {
                    class: ClassEntity::Source {
                        declaration: *class,
                    }
                    .id(),
                    name: name.as_str().into(),
                };
                let entity = EntityRef::Field { field: field.id() }.id();
                data.entities.refs.get(entity).map(Record::id)
            }
            PlaceRoot::Global { module, .. } => Some(EntityRef::Module { module: *module }.id()),
            PlaceRoot::Local { scope, .. } => Some(source_entity(data, *scope)?),
            PlaceRoot::Occurrence { occurrence } => Some(
                EntityRef::Occurrence {
                    occurrence: *occurrence,
                }
                .id(),
            ),
            // A runtime class value is not the identity of its source expression.
            PlaceRoot::ClassOf { .. } => None,
        };
        if let Some(id) = entity {
            need(&data.entities.refs, id)?;
        }
        let (status, reason) = decision(usize::from(entity.is_some()), false);
        output.place_entity_links.insert(PlaceEntityLink {
            place: place.id(),
            entity,
            status,
            reason,
        })?;
    }
    Ok(())
}
fn test_operands(
    data: &RelationData,
    output: &mut RelationOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut charge = StateCharge::new(budget, "test-operand-index");
    let mut observations: ChargedMap<(Id<AnalysisContext>, Id<Occurrence>), Vec<&TypeObservation>> =
        Default::default();
    for row in data
        .type_observations
        .iter()
        .filter(|o| o.role == TypeRole::TestOperand)
    {
        observations.update(
            &mut charge,
            (context(data, row.qualification)?, row.subject),
            |rows| rows.push(row),
        )?;
    }
    let mut coverages: ChargedMap<
        (Id<AnalysisContext>, Id<SourceArtifact>),
        Vec<&ProviderCoverage>,
    > = Default::default();
    for row in data
        .coverage
        .iter()
        .filter(|c| c.family == FactFamily::Types)
    {
        if let CoverageScope::Artifact { artifact } = need(&data.scopes, row.scope)? {
            coverages.update(&mut charge, (row.context, *artifact), |rows| rows.push(row))?;
        }
    }
    for leaf in data.leaves.iter() {
        let context = context(data, leaf.qualification)?;
        let matches = leaf
            .operand
            .and_then(|operand| observations.get(&(context, operand)));
        let source = need(&data.facts.occurrences, leaf.test)?.source;
        let coverage = coverages.get(&(context, source));
        let mut held = StateCharge::new(budget, "test-operand-candidates");
        let mut terms: ChargedSet<Id<TypeTerm>> = Default::default();
        for row in matches.into_iter().flatten() {
            terms.insert(&mut held, row.term)?;
        }
        let (status, mut reason) = decision(terms.len(), false);
        if terms.is_empty() {
            reason = if leaf.operand.is_none() {
                LinkReason::MissingOperand
            } else if coverage.is_some_and(|rows| {
                !rows.is_empty()
                    && rows
                        .iter()
                        .all(|r| r.status == CoverageStatus::NotRequested)
            }) {
                LinkReason::NotRequested
            } else if coverage.is_some_and(|rows| {
                !rows.is_empty() && rows.iter().all(|r| r.status == CoverageStatus::Unavailable)
            }) {
                LinkReason::Unavailable
            } else if coverage.is_none_or(|rows| {
                rows.iter()
                    .any(|r| r.status != CoverageStatus::CompleteUnderStatedModel)
            }) {
                LinkReason::IncompleteInput
            } else {
                LinkReason::NoTypeObservation
            };
        }
        let assessment =
            output
                .test_operand_type_assessments
                .insert(TestOperandTypeAssessment {
                    leaf: leaf.id(),
                    status,
                    reason,
                })?;
        for row in matches.into_iter().flatten() {
            output.test_operand_type_links.insert(TestOperandTypeLink {
                assessment,
                observation: row.id(),
            })?;
        }
        for row in coverage.into_iter().flatten() {
            output.test_operand_coverage.insert(TestOperandCoverage {
                assessment,
                coverage: row.id(),
            })?;
        }
    }
    Ok(())
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = RelationData::validation_inputs();
    inputs.extend(RelationOutput::validation_inputs());
    vec![Invariant {
        revision: 1,
        name: "normalized_relationship_closure",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(RelationCheck {
                data: RelationData::new(budget),
                output: RelationOutput::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct RelationCheck {
    data: RelationData,
    output: RelationOutput,
    budget: ResourceBudget,
}
impl InvariantCheck for RelationCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if !self.data.visit(relation, batch)? && !self.output.visit(relation, batch)? {
            return Err(invalid("undeclared normalized relationship input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.output.matches(&normalize(&self.data, &self.budget)?)
    }
}
pub fn stage(profile: stages::Profile) -> stages::Stage {
    // Catalog does not request a flow producer. Its frozen facts checkpoint validates those
    // relations as unrequested; there is no fabricated writer or completed-source capability.
    let mut inputs = RelationData::stage_inputs();
    if profile == stages::Profile::Catalog {
        inputs.retain(|r| r.name() != flow::FlowTestLeafObservation::NAME);
    }
    stages::Stage {
        name: "normalize_relations",
        inputs: super::facts_stage_inputs(inputs),
        outputs: super::links::relations()
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: stages::Effect::Pure,
        code: policy_revision(),
        configuration: ContentHash::of(b"relations/v1"),
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> { vec!["normalized_relationship_closure"] }
