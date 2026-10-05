//! Exact source associations under a deliberately small plain initializer model.
//! None of these records proves allocation, reaching heap state, mutation freedom between
//! methods, normal invocation, or a receiver's runtime value. Callable metadata is the writer.
//! Admission requires exact standard decorator, complete source fields/defaults and native
//! initializer shape. Source associations remain distinct from temporal heap identity.
use crate::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    declarations::*,
    lexical::*,
    normalized::{
        Rows,
        callable_aspects::{AspectData, AspectOutput, FieldDefault},
    },
    obligation::ObligationKind,
    resources::ResourceBudget,
    source::*,
    symbols::*,
    syntax::*,
    types::*,
    *,
};
use crate::{Domain, DomainCode};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum SourceStorageKind {
    PlainInitializer = 0,
    GeneratedRecord = 1,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(
    name = "source_field_class_assessments",
    rule = "source_plain_record_shape"
)]
pub struct SourceFieldClass {
    #[model(key)]
    pub class: Id<Occurrence>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(premise)]
    pub traits: Id<ClassTraitObservation>,
    #[model(premise)]
    pub support: Id<ClassTraitSupport>,
    pub supported_record: bool,
    pub reason: Option<ObligationKind>,
    pub inventory: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "source_field_stores", rule = "source_receiver_field_store")]
pub struct SourceFieldStore {
    #[model(key)]
    pub class: Id<Occurrence>,
    #[model(key)]
    pub constructor: Id<Occurrence>,
    #[model(key)]
    pub target: Id<Occurrence>,
    pub name: String,
    pub value: Id<Occurrence>,
    #[model(premise)]
    pub formal: Id<ParameterDeclaration>,
    #[model(premise)]
    pub receiver: Id<ParameterSyntaxObservation>,
    #[model(premise)]
    pub placement: Id<SyntaxPlacement>,
    #[model(premise)]
    pub traits: Id<FunctionTraitObservation>,
    pub qualification: Id<AssertionQualification>,
    pub plain_initializer: bool,
    pub inventory: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name = "source_field_readers", rule = "source_receiver_field_reader")]
pub struct SourceFieldReader {
    #[model(key)]
    pub class: Id<Occurrence>,
    #[model(key)]
    pub reader: Id<Occurrence>,
    #[model(key)]
    pub access: Id<Occurrence>,
    pub name: String,
    #[model(premise)]
    pub receiver: Id<ParameterSyntaxObservation>,
    #[model(premise)]
    pub placement: Id<SyntaxPlacement>,
    #[model(premise)]
    pub traits: Id<FunctionTraitObservation>,
    pub qualification: Id<AssertionQualification>,
    pub inventory: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(
    name = "source_field_associations",
    rule = "source_field_initialization_association"
)]
pub struct SourceFieldAssociation {
    #[model(key, premise)]
    pub class: Id<SourceFieldClass>,
    #[model(key, premise)]
    pub field: Id<RecordFieldObservation>,
    #[model(key, premise)]
    pub parameter: Id<SignatureParameter>,
    #[model(premise)]
    pub store: Option<Id<SourceFieldStore>>,
    pub kind: SourceStorageKind,
    pub qualification: Id<AssertionQualification>,
    pub inventory: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(
    name = "source_field_reader_links",
    rule = "source_field_reader_association"
)]
pub struct SourceFieldReaderLink {
    #[model(key, premise)]
    pub association: Id<SourceFieldAssociation>,
    #[model(key, premise)]
    pub reader: Id<SourceFieldReader>,
}
fn exact(d: &AspectData, id: Id<AssertionQualification>, context: Id<AnalysisContext>) -> bool {
    d.qualifications.get(id).is_some_and(|q| {
        q.context == context
            && q.modality == Modality::Definite
            && q.approximation == Approximation::Exact
            && q.condition == conditions::Diagram::always().id()
    })
}
fn same(d: &AspectData, a: Id<Occurrence>, b: Id<Occurrence>) -> bool {
    d.occurrences
        .get(a)
        .zip(d.occurrences.get(b))
        .is_some_and(|(a, b)| (a.source, a.start, a.end) == (b.source, b.start, b.end))
}
trait SourceSupport: Support {
    fn source_fidelity(&self) -> bool;
}
macro_rules! source_supports {
    ($projection:expr; $($ty:ty),* $(,)?) => {$(
        impl SourceSupport for $ty {
            fn source_fidelity(&self) -> bool {
                (self.fidelity == Fidelity::NativeStructural
                    && self.mode == ExtractionMode::NativeTraversal
                    && matches!(self.origin,Origin::SourceObservation|Origin::AnalyzerAssertion))
                    || ($projection && self.fidelity == Fidelity::ReportProjection
                        && self.mode == ExtractionMode::NativeTraversal
                        && self.origin == Origin::AnalyzerAssertion)
            }
        }
    )*};
}
// The provider's typed symbol/signature projections retain their attribution. Admitting them
// for source metadata does not promote them into native flow or runtime execution proofs.
source_supports!(true; SignatureSupport, SignatureEnumerationSupport, FunctionTraitSupport, ClassTraitSupport,
    ClassAncestrySupport, SymbolDeclarationSupport, ParameterDeclarationSupport);
source_supports!(false; SyntaxSupport, SyntaxDetailSupport, SyntaxPlacementSupport,
    DeclarationSupport, DeclarationDecoratorSupport, ParameterSyntaxSupport,
    ClassFieldSyntaxSupport, ImportAliasSupport, CallTargetSupport, CallSyntaxSupport, RecordFieldSupport);
// These three raw lexical observations are produced by the source recognizer. Retain their
// normalized source-model fidelity rather than presenting it as native-provider execution.
macro_rules! lexical_supports { ($($ty:ty),* $(,)?) => {$(
    impl SourceSupport for $ty {
        fn source_fidelity(&self) -> bool {
            self.fidelity == Fidelity::NormalizedStructural
                && self.mode == ExtractionMode::Recognizer
                && self.origin == Origin::DerivedAnalysis
        }
    }
)*}; }
lexical_supports!(LexicalResolutionSupport, ReferenceSupport, BindingSupport);
fn source<R: Assertion, S: SourceSupport<Assertion = R>>(
    d: &AspectData,
    row: &R,
    supports: &Rows<S>,
    context: Id<AnalysisContext>,
) -> bool {
    supports
        .iter()
        .filter(|s| s.assertion() == row.id())
        .any(|s| {
            s.attribution().is_some_and(|a| {
                s.source_fidelity()
                    && d.symbolic_runs
                        .get(a.run)
                        .is_some_and(|r| r.context == context)
            })
        })
}
fn symbol_for(
    d: &AspectData,
    occurrence: Id<Occurrence>,
    context: Id<AnalysisContext>,
) -> Option<Id<ProviderSymbol>> {
    let mut rows = d.symbolic_symbol_declarations.iter().filter(|r| {
        same(d, r.declaration, occurrence)
            && exact(d, r.qualification, context)
            && source(d, *r, &d.symbolic_symbol_supports, context)
    });
    let first = rows.next()?;
    if rows.any(|r| r.symbol != first.symbol) {
        return None;
    }
    Some(first.symbol)
}
fn method(
    d: &AspectData,
    function: Id<Occurrence>,
    class: Id<ProviderSymbol>,
    context: Id<AnalysisContext>,
) -> Option<&FunctionTraitObservation> {
    if d.decorators
        .iter()
        .any(|r| same(d, r.declaration, function))
    {
        return None;
    }
    let symbol = symbol_for(d, function, context)?;
    let mut rows = d.traits.iter().filter(|r| {
        r.symbol == symbol
            && r.defining_class == Some(class)
            && !r.staticmethod
            && !r.classmethod
            && !r.property_getter
            && !r.property_setter
            && !r.stub
            && r.origin == FunctionOrigin::DefStatement
            && exact(d, r.qualification, context)
            && source(d, *r, &d.symbolic_trait_supports, context)
    });
    let row = rows.next()?;
    if rows.next().is_some() {
        return None;
    }
    Some(row)
}
fn receiver(
    d: &AspectData,
    function: Id<Occurrence>,
    context: Id<AnalysisContext>,
) -> Option<&ParameterSyntaxObservation> {
    let mut rows = d.symbolic_parameter_syntax.iter().filter(|r| {
        same(d, r.function, function)
            && r.ordinal == 0
            && exact(d, r.qualification, context)
            && source(d, *r, &d.symbolic_parameter_syntax_supports, context)
    });
    let first = rows.next()?;
    if rows.next().is_some() {
        return None;
    }
    Some(first)
}
fn formal_read(
    d: &AspectData,
    read: Id<Occurrence>,
    parameter: Id<Occurrence>,
    context: Id<AnalysisContext>,
) -> bool {
    let mut found = false;
    for r in d
        .lexical_resolutions
        .iter()
        .filter(|r| same(d, r.read, read) && exact(d, r.qualification, context))
    {
        if r.captured || !source(d, r, &d.symbolic_resolution_supports, context) {
            return false;
        }
        let Some(LexicalTarget::Binding { event }) = d.symbolic_lexical_targets.get(r.target)
        else {
            return false;
        };
        let Some(event) = d.binding_events.get(*event) else {
            return false;
        };
        let bindings = d
            .bindings
            .iter()
            .filter(|b| b.event == event.id())
            .collect::<Vec<_>>();
        if bindings.len() != 1
            || bindings[0].kind != BindingEventKind::Parameter
            || !exact(d, bindings[0].qualification, context)
            || !source(d, bindings[0], &d.symbolic_binding_supports, context)
        {
            return false;
        }
        if !same(d, event.site, parameter) {
            return false;
        }
        found = true
    }
    found
}
fn child(
    d: &AspectData,
    parent: Id<Occurrence>,
    field: SyntaxField,
    context: Id<AnalysisContext>,
) -> Option<&SyntaxPlacement> {
    let mut rows = d.placements.iter().filter(|p| {
        p.parent.is_some_and(|p| same(d, p, parent))
            && p.field == field
            && exact(d, p.qualification, context)
            && source(d, *p, &d.symbolic_placement_supports, context)
    });
    let row = rows.next()?;
    if rows.next().is_some() {
        return None;
    }
    Some(row)
}
fn attribute(
    d: &AspectData,
    site: Id<Occurrence>,
    receiver: &ParameterSyntaxObservation,
    context: Id<AnalysisContext>,
) -> Option<String> {
    if d.occurrences.get(site)?.syntax_kind != SyntaxKind::ExprAttribute {
        return None;
    }
    let base = child(d, site, SyntaxField::Value, context)?;
    if !formal_read(d, base.occurrence, receiver.parameter, context) {
        return None;
    }
    let mut name = None;
    for p in d.placements.iter().filter(|p| {
        p.parent.is_some_and(|p| same(d, p, site))
            && d.occurrences
                .get(p.occurrence)
                .is_some_and(|o| o.syntax_kind == SyntaxKind::Identifier)
    }) {
        if !exact(d, p.qualification, context)
            || !source(d, p, &d.symbolic_placement_supports, context)
        {
            return None;
        }
        for text in d
            .spellings
            .iter()
            .filter(|s| same(d, s.occurrence, p.occurrence) && exact(d, s.qualification, context))
        {
            if !source(d, text, &d.symbolic_syntax_supports, context) {
                return None;
            }
            if name
                .as_ref()
                .is_some_and(|old| old != text.spelling.as_str())
            {
                return None;
            }
            name = Some(text.spelling.as_str().to_owned());
        }
    }
    name
}
fn contains(d: &AspectData, outer: Id<Occurrence>, inner: Id<Occurrence>) -> bool {
    d.occurrences
        .get(outer)
        .zip(d.occurrences.get(inner))
        .is_some_and(|(a, b)| {
            a.source == b.source
                && a.start <= b.start
                && a.end >= b.end
                && b.structural_path.starts_with(&a.structural_path)
        })
}
fn covered(d: &AspectData, class: Id<Occurrence>, context: Id<AnalysisContext>) -> bool {
    let Some(o) = d.occurrences.get(class) else {
        return false;
    };
    let Some(a) = d.artifacts.get(o.source) else {
        return false;
    };
    !a.is_stub()
        && d.symbolic_coverage.iter().any(|c| {
            c.context == context
                && c.family == FactFamily::Syntax
                && c.status == CoverageStatus::CompleteUnderStatedModel
                && c.run.is_some_and(|r| {
                    d.symbolic_runs
                        .get(r)
                        .is_some_and(|r| r.input == a.input && r.context == context)
                })
                && match d.symbolic_scopes.get(c.scope) {
                    Some(CoverageScope::Artifact { artifact }) => *artifact == a.id(),
                    Some(CoverageScope::Input { input }) => *input == a.input,
                    Some(CoverageScope::Module { module }) => {
                        d.modules.get(*module).is_some_and(|m| m.source == a.id())
                    }
                    _ => false,
                }
        })
}
struct ClassInventory {
    hashes: crate::domain::identity::PreparedContentHashes,
    _reservation: Box<dyn crate::domain::resources::Reservation>,
}
impl ClassInventory {
    fn prepare(d: &AspectData, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut count = 0_usize;
        macro_rules! count_rows { ($($field:ident:$ty:ty,)*) => {$(
            count = count.checked_add(d.$field.len())
                .ok_or_else(|| ModelError::Invalid("class inventory size overflow".into()))?;
        )*}; }
        crate::callable_aspect_inputs!(count_rows);
        use crate::domain::identity::PreparedContentHashes;
        let size = PreparedContentHashes::encoded_size(count)
            .ok_or_else(|| ModelError::Invalid("class inventory size overflow".into()))?;
        // Reserve an allocation envelope before allocation, then retain the exact capacity.
        let capacity = size
            .checked_next_power_of_two()
            .ok_or_else(|| ModelError::Invalid("class inventory capacity overflow".into()))?;
        let metadata = size_of::<Self>();
        let allowance = capacity
            .checked_add(metadata)
            .ok_or_else(|| ModelError::Invalid("class inventory allowance overflow".into()))?;
        let mut reservation = budget.reserve("source-class-inventory", allowance)?;
        let mut hashes = PreparedContentHashes::try_new(count)?;
        if hashes.capacity() > capacity {
            return Err(ModelError::Invalid(
                "class inventory allocation exceeds reserved capacity".into(),
            ));
        }
        reservation.try_resize(hashes.capacity() + metadata)?;
        // Original macro order and original ID order, including otherwise unused siblings.
        macro_rules! rows { ($($field:ident:$ty:ty,)*) => {$(
            for row in d.$field.iter() { hashes.push(row.content_digest())?; }
        )*}; }
        crate::callable_aspect_inputs!(rows);
        Ok(Self {
            hashes,
            _reservation: reservation,
        })
    }
    fn for_class(
        &self,
        class: Id<Occurrence>,
        context: Id<AnalysisContext>,
        symbol: Id<ProviderSymbol>,
    ) -> ContentHash {
        let mut k = KeySink::new("source-field-class-full-inventory");
        class.encode(&mut k);
        context.encode(&mut k);
        self.hashes.encode(&mut k);
        symbol.encode(&mut k);
        k.finish()
    }
}

fn plain_init(
    d: &AspectData,
    function: Id<Occurrence>,
    symbol: Id<ProviderSymbol>,
    context: Id<AnalysisContext>,
    receiver: &ParameterSyntaxObservation,
) -> bool {
    if d.decorators
        .iter()
        .any(|r| same(d, r.declaration, function) && exact(d, r.qualification, context))
    {
        return false;
    }
    let mut written = std::collections::BTreeSet::new();
    for p in d.placements.iter().filter(|p| {
        p.parent.is_some_and(|p| same(d, p, function))
            && p.field == SyntaxField::Body
            && exact(d, p.qualification, context)
    }) {
        if !source(d, p, &d.symbolic_placement_supports, context) {
            return false;
        }
        let Some(node) = d.occurrences.get(p.occurrence) else {
            return false;
        };
        match node.syntax_kind {
            SyntaxKind::StmtPass => {}
            SyntaxKind::StmtExpr => {
                let Some(v) = child(d, node.id(), SyntaxField::Value, context) else {
                    return false;
                };
                if d.occurrences
                    .get(v.occurrence)
                    .is_none_or(|n| n.syntax_kind != SyntaxKind::ExprStringLiteral)
                {
                    return false;
                }
            }
            SyntaxKind::StmtAssign => {
                let Some(t) = child(d, node.id(), SyntaxField::Target, context) else {
                    return false;
                };
                let Some(name) = attribute(d, t.occurrence, receiver, context) else {
                    return false;
                };
                if !written.insert(name.clone()) {
                    return false;
                }
                if !d.symbolic_record_fields.iter().any(|f| {
                    f.class == symbol
                        && f.name.as_str() == name
                        && f.record == RecordKind::Dataclass
                }) {
                    return false;
                }
                let Some(v) = child(d, node.id(), SyntaxField::Value, context) else {
                    return false;
                };
                let Some(vn) = d.occurrences.get(v.occurrence) else {
                    return false;
                };
                if !matches!(
                    vn.syntax_kind,
                    SyntaxKind::ExprName
                        | SyntaxKind::ExprStringLiteral
                        | SyntaxKind::ExprNumberLiteral
                        | SyntaxKind::ExprBooleanLiteral
                        | SyntaxKind::ExprNoneLiteral
                ) {
                    return false;
                }
                if vn.syntax_kind == SyntaxKind::ExprName
                    && !d.symbolic_parameter_syntax.iter().any(|p| {
                        same(d, p.function, function)
                            && formal_read(d, v.occurrence, p.parameter, context)
                    })
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}
fn spelling(d: &AspectData, site: Id<Occurrence>, context: Id<AnalysisContext>) -> Option<&str> {
    let rows = d
        .spellings
        .iter()
        .filter(|r| same(d, r.occurrence, site))
        .collect::<Vec<_>>();
    let first = rows.first()?;
    if rows.iter().any(|r| {
        !exact(d, r.qualification, context)
            || !source(d, *r, &d.symbolic_syntax_supports, context)
            || r.spelling != first.spelling
    }) {
        return None;
    }
    Some(first.spelling.as_str())
}
fn literal(
    d: &AspectData,
    site: Id<Occurrence>,
    context: Id<AnalysisContext>,
) -> Option<&value::Literal> {
    let rows = d
        .details
        .iter()
        .filter(|r| same(d, r.occurrence, site))
        .collect::<Vec<_>>();
    let first = rows.first()?;
    if rows.iter().any(|r| {
        !exact(d, r.qualification, context)
            || !source(d, *r, &d.symbolic_detail_supports, context)
            || r.detail != first.detail
    }) {
        return None;
    }
    let SyntaxDetail::Literal { literal } = d.detail_values.get(first.detail)? else {
        return None;
    };
    d.symbolic_literals.get(*literal)
}
fn standard_import(
    d: &AspectData,
    site: Id<Occurrence>,
    name: &str,
    context: Id<AnalysisContext>,
) -> bool {
    let Some(node) = d.occurrences.get(site) else {
        return false;
    };
    let (read, qualified) = match node.syntax_kind {
        SyntaxKind::ExprName => (site, false),
        SyntaxKind::ExprAttribute => {
            let Some(base) = child(d, site, SyntaxField::Value, context) else {
                return false;
            };
            let names = d
                .placements
                .iter()
                .filter(|p| {
                    p.parent.is_some_and(|p| same(d, p, site))
                        && d.occurrences
                            .get(p.occurrence)
                            .is_some_and(|o| o.syntax_kind == SyntaxKind::Identifier)
                })
                .collect::<Vec<_>>();
            if names.len() != 1
                || !exact(d, names[0].qualification, context)
                || !source(d, names[0], &d.symbolic_placement_supports, context)
                || spelling(d, names[0].occurrence, context) != Some(name)
            {
                return false;
            }
            (base.occurrence, true)
        }
        _ => return false,
    };
    let references = d
        .references
        .iter()
        .filter(|r| same(d, r.read, read))
        .collect::<Vec<_>>();
    if references.len() != 1
        || !exact(d, references[0].qualification, context)
        || !source(d, references[0], &d.symbolic_reference_supports, context)
    {
        return false;
    }
    let resolutions = d
        .lexical_resolutions
        .iter()
        .filter(|r| same(d, r.read, read))
        .collect::<Vec<_>>();
    if resolutions.len() != 1
        || resolutions[0].captured
        || !exact(d, resolutions[0].qualification, context)
        || !source(d, resolutions[0], &d.symbolic_resolution_supports, context)
    {
        return false;
    }
    let Some(LexicalTarget::Binding { event }) =
        d.symbolic_lexical_targets.get(resolutions[0].target)
    else {
        return false;
    };
    let Some(event) = d.binding_events.get(*event) else {
        return false;
    };
    if event.name != references[0].name {
        return false;
    }
    let bindings = d
        .bindings
        .iter()
        .filter(|b| b.event == event.id())
        .collect::<Vec<_>>();
    if bindings.len() != 1
        || bindings[0].kind
            != if qualified {
                BindingEventKind::Import
            } else {
                BindingEventKind::FromImport
            }
        || !exact(d, bindings[0].qualification, context)
        || !source(d, bindings[0], &d.symbolic_binding_supports, context)
    {
        return false;
    }
    let imports = d
        .symbolic_imports
        .iter()
        .filter(|i| contains(d, i.alias, event.site))
        .collect::<Vec<_>>();
    if imports.len() != 1 {
        return false;
    }
    let mut identifiers = d
        .placements
        .iter()
        .filter(|p| {
            p.parent.is_some_and(|p| same(d, p, imports[0].alias))
                && d.occurrences
                    .get(p.occurrence)
                    .is_some_and(|o| o.syntax_kind == SyntaxKind::Identifier)
        })
        .collect::<Vec<_>>();
    identifiers.sort_by_key(|p| p.ordinal);
    if !(1..=2).contains(&identifiers.len())
        || identifiers.iter().any(|p| {
            !exact(d, p.qualification, context)
                || !source(d, *p, &d.symbolic_placement_supports, context)
        })
        || spelling(d, identifiers[0].occurrence, context)
            != Some(if qualified { "dataclasses" } else { name })
        || spelling(d, identifiers.last().unwrap().occurrence, context) != Some(event.name.as_str())
    {
        return false;
    }
    imports[0].level == 0
        && imports[0].resolved_module.as_deref() == Some("dataclasses")
        && exact(d, imports[0].qualification, context)
        && source(d, imports[0], &d.symbolic_import_supports, context)
        && d.occurrences.get(imports[0].statement).is_some_and(|o| {
            o.syntax_kind
                == if qualified {
                    SyntaxKind::StmtImport
                } else {
                    SyntaxKind::StmtImportFrom
                }
        })
}
fn standard_target(
    d: &AspectData,
    site: Id<Occurrence>,
    name: &str,
    context: Id<AnalysisContext>,
) -> bool {
    let Some(node) = d.occurrences.get(site) else {
        return false;
    };
    let callee = if node.syntax_kind == SyntaxKind::ExprCall {
        let Some(callee) = child(d, site, SyntaxField::Callee, context) else {
            return false;
        };
        callee.occurrence
    } else {
        site
    };
    if !standard_import(d, callee, name, context) {
        return false;
    }
    let targets = d
        .targets
        .iter()
        .filter(|t| same(d, t.site, site))
        .collect::<Vec<_>>();
    !targets.is_empty() && targets.iter().all(|t| {
        // Bare decorator application is Potential in the provider. The exact lexical import
        // establishes source target identity; this does not admit its runtime application.
        let Some(q)=d.qualifications.get(t.qualification) else {return false;};
        if q.context!=context || q.approximation!=Approximation::Exact
            || q.condition!=conditions::Diagram::always().id()
            || (node.syntax_kind==SyntaxKind::ExprCall && q.modality!=Modality::Definite)
            || !source(d, *t, &d.symbolic_target_supports, context) { return false; }
        let Some(CallDestination::Resolved { symbol }) = d.destinations.get(t.destination) else { return false; };
        let Some(s) = d.symbols.get(*symbol) else { return false; };
        s.context == context && s.kind == SymbolKind::Function && s.name == name
            && matches!(d.provider_modules.get(s.module), Some(ProviderModule::Bundled { provider, bundle: ModuleBundle::Typeshed, name }) if *provider == s.provider && name == "dataclasses")
    })
}
fn arguments(
    d: &AspectData,
    site: Id<Occurrence>,
    context: Id<AnalysisContext>,
) -> Option<Vec<&CallArgument>> {
    let mut calls = d.calls.iter().filter(|c| same(d, c.site, site));
    let call = calls.next()?;
    if calls.next().is_some()
        || !exact(d, call.qualification, context)
        || !source(d, call, &d.symbolic_call_supports, context)
    {
        return None;
    }
    let mut arguments = d
        .arguments
        .iter()
        .filter(|a| a.call == call.id())
        .collect::<Vec<_>>();
    arguments.sort_by_key(|a| a.ordinal);
    call.actuals(&arguments.iter().map(|a| (*a).clone()).collect::<Vec<_>>())
        .ok()?;
    Some(arguments)
}
fn decorator_options(
    d: &AspectData,
    decorator: &DeclarationDecorator,
    context: Id<AnalysisContext>,
) -> Option<(bool, bool)> {
    let mut expression = decorator.decorator;
    if d.occurrences.get(expression)?.syntax_kind == SyntaxKind::Decorator {
        expression = child(d, expression, SyntaxField::Value, context)
            .or_else(|| child(d, expression, SyntaxField::Child, context))?
            .occurrence;
    }
    if !standard_target(d, expression, "dataclass", context) {
        return None;
    }
    let mut init = true;
    let mut kw_only = false;
    if d.occurrences.get(expression)?.syntax_kind == SyntaxKind::ExprCall {
        let mut seen = std::collections::BTreeSet::new();
        for argument in arguments(d, expression, context)? {
            if argument.kind != ArgumentKind::Keyword {
                return None;
            }
            let key = argument.keyword.as_deref()?;
            if !seen.insert(key) {
                return None;
            }
            let value::Literal::Bool { value } = literal(d, argument.value, context)? else {
                return None;
            };
            match key {
                "init" => init = *value,
                "kw_only" => kw_only = *value,
                // Options that replace the class or change field access/allocation are outside
                // this narrow source-storage model. Other standard defaults are harmless.
                "repr" | "eq" | "match_args" if *value => {}
                "order" | "unsafe_hash" | "frozen" | "slots" | "weakref_slot" if !*value => {}
                _ => return None,
            }
        }
    }
    Some((init, kw_only))
}
// Synthesized initializer metadata supplies a source-field shape, never a source body.
// Its native completeness receipt replaces the source enumeration that does not exist for
// generated functions; exact slot replay still detects omission and reordered membership.
fn generated_initializer_parameters(
    d: &AspectData,
    symbol: Id<ProviderSymbol>,
    context: Id<AnalysisContext>,
) -> Option<Vec<&SignatureParameter>> {
    let mut signatures = d
        .symbolic_signatures
        .iter()
        .filter(|s| s.role == SignatureRole::Synthesized && s.symbol == symbol);
    let signature = signatures.next()?;
    if signatures.next().is_some()
        || signature.form != SignatureForm::List
        || !exact(d, signature.qualification, context)
        || !source(d, signature, &d.symbolic_signature_supports, context)
    {
        return None;
    }
    let mut parameters = d
        .symbolic_parameters
        .iter()
        .filter(|p| p.signature == signature.id())
        .collect::<Vec<_>>();
    parameters.sort_by_key(|p| p.ordinal);
    let shapes = parameters
        .iter()
        .map(|p| d.symbolic_parameter_shapes.get(p.shape).cloned())
        .collect::<Option<Vec<_>>>()?;
    let q = d.qualifications.get(signature.qualification)?;
    let native = d
        .symbolic_native_signatures
        .iter()
        .filter(|n| n.signature == signature.id())
        .collect::<Vec<_>>();
    if native.len() != 1
        || !native[0].complete
        || native[0].qualification != signature.qualification
        || native[0].scope != signature.scope
        || signature.native != Some(native[0].term)
        || !d.symbolic_native_signature_supports.iter().any(|n| {
            n.assertion == native[0].id()
                && n.origin == Origin::AnalyzerAssertion
                && n.mode == ExtractionMode::NativeTraversal
                && n.fidelity == Fidelity::NativeStructural
                && d.symbolic_runs.get(n.run).is_some_and(|r| {
                    r.context == context
                        && d.symbolic_surfaces.get(n.surface).is_some_and(|surface| {
                            surface.provider == r.provider && surface.family == FactFamily::Types
                        })
                })
                && d.symbolic_signature_supports.iter().any(|s| {
                    s.assertion == signature.id()
                        && s.run == n.run
                        && d.symbolic_surfaces.get(s.surface).is_some_and(|surface| {
                            surface.family == FactFamily::Signatures
                                && d.symbolic_runs
                                    .get(s.run)
                                    .is_some_and(|run| surface.provider == run.provider)
                        })
                        && s.origin == Origin::AnalyzerAssertion
                        && s.mode == ExtractionMode::NativeTraversal
                        && s.fidelity == Fidelity::NativeStructural
                })
        })
    {
        return None;
    }
    let (rebuilt, members) = Signature::new(
        q,
        signature.role,
        signature.native,
        symbol,
        signature.variant,
        signature.form,
        &shapes,
    )
    .ok()?;
    if rebuilt != *signature
        || members.len() != parameters.len()
        || members.iter().zip(&parameters).any(|(a, b)| a != *b)
    {
        return None;
    }
    Some(parameters)
}
fn initializer(
    d: &AspectData,
    class: Id<ProviderSymbol>,
    context: Id<AnalysisContext>,
) -> Option<&FunctionTraitObservation> {
    let mut rows = d.traits.iter().filter(|t| {
        t.defining_class == Some(class)
            && d.symbols
                .get(t.symbol)
                .is_some_and(|s| s.name == "__init__")
    });
    let first = rows.next()?;
    if rows.next().is_some()
        || !exact(d, first.qualification, context)
        || !source(d, first, &d.symbolic_trait_supports, context)
        || first.staticmethod
        || first.classmethod
        || first.property_getter
        || first.property_setter
        || first.stub
        || first.overload
    {
        return None;
    }
    Some(first)
}
fn field_default(
    d: &AspectData,
    syntax: &ClassFieldSyntaxObservation,
    field: &RecordFieldObservation,
    context: Id<AnalysisContext>,
    out: &AspectOutput,
) -> bool {
    let links = d
        .fields
        .iter()
        .filter(|l| l.declaration == syntax.id())
        .collect::<Vec<_>>();
    if links.len() != 1 {
        return false;
    }
    let Some(assessment) = out.fields.iter().find(|a| a.declaration == links[0].id()) else {
        return false;
    };
    let Some(default) = out.defaults.get(assessment.default) else {
        return false;
    };
    let expected_default = !matches!(default, FieldDefault::Absent {});
    if field.has_default != Some(expected_default) {
        return false;
    }
    (match default {
        FieldDefault::Absent {} => syntax.value.is_none(),
        FieldDefault::Literal { observation, literal: value } => d.details.get(*observation).is_some_and(|detail|
            exact(d, detail.qualification, context) && source(d, detail, &d.symbolic_detail_supports, context)
                && matches!(d.detail_values.get(detail.detail),Some(SyntaxDetail::Literal{literal}) if literal == value)),
        FieldDefault::Factory { argument, target, .. } => d.arguments.get(*argument).zip(d.targets.get(*target)).is_some_and(|(a,t)|
            a.keyword.as_deref() == Some("default_factory") && standard_target(d,t.site,"field",context)
                && d.occurrences.get(a.value).is_some_and(|o| matches!(o.syntax_kind,SyntaxKind::ExprName | SyntaxKind::ExprAttribute))),
        _ => false,
    }) && syntax.value.is_none_or(|site| {
        if d.occurrences.get(site).is_none_or(|o| o.syntax_kind != SyntaxKind::ExprCall) { return true; }
        if !standard_target(d, site, "field", context) { return false; }
        let Some(args) = arguments(d, site, context) else { return false; };
        let mut seen = std::collections::BTreeSet::new();
        args.iter().all(|a| {
            if a.kind != ArgumentKind::Keyword { return false; }
            let Some(key) = a.keyword.as_deref() else { return false; };
            if !seen.insert(key) { return false; }
            match key {
                "default" | "default_factory" => true,
                "init" => matches!(literal(d,a.value,context),Some(value::Literal::Bool{value}) if Some(*value)==field.init),
                "kw_only" => matches!(literal(d,a.value,context),Some(value::Literal::Bool{value}) if Some(*value)==field.kw_only),
                "repr" | "compare" => matches!(literal(d,a.value,context),Some(value::Literal::Bool{..})),
                _ => false,
            }
        })
    })
}
fn direct_member(
    d: &AspectData,
    class: Id<Occurrence>,
    node: Id<Occurrence>,
    context: Id<AnalysisContext>,
) -> bool {
    let rows = d
        .placements
        .iter()
        .filter(|p| same(d, p.occurrence, node))
        .collect::<Vec<_>>();
    rows.len() == 1
        && rows[0].parent.is_some_and(|p| same(d, p, class))
        && rows[0].field == SyntaxField::Body
        && exact(d, rows[0].qualification, context)
        && source(d, rows[0], &d.symbolic_placement_supports, context)
}
fn record_gate(
    d: &AspectData,
    class: Id<Occurrence>,
    symbol: Id<ProviderSymbol>,
    context: Id<AnalysisContext>,
    out: &AspectOutput,
) -> Option<ObligationKind> {
    if !covered(d, class, context) {
        return Some(ObligationKind::IncompleteCoverage);
    }
    let decorators = d
        .decorators
        .iter()
        .filter(|r| same(d, r.declaration, class))
        .collect::<Vec<_>>();
    if decorators.len() != 1
        || !exact(d, decorators[0].qualification, context)
        || !source(d, decorators[0], &d.symbolic_decorator_supports, context)
    {
        return Some(ObligationKind::OutsideProviderModel);
    }
    let Some((generate_init, kw_only)) = decorator_options(d, decorators[0], context) else {
        return Some(ObligationKind::OutsideProviderModel);
    };
    if d.placements
        .iter()
        .any(|p| p.parent.is_some_and(|p| same(d, p, class)) && p.field == SyntaxField::Argument)
    {
        return Some(ObligationKind::IncompleteDomain);
    }
    let mros = d
        .symbolic_ancestry
        .iter()
        .filter(|m| m.class == symbol && m.relation == AncestryRelation::Mro)
        .collect::<Vec<_>>();
    if mros.len() != 1
        || !exact(d, mros[0].qualification, context)
        || mros[0].linearization != Some(Linearization::Complete)
        || !source(d, mros[0], &d.symbolic_ancestry_supports, context)
        || d.symbolic_sequences
            .get(mros[0].ancestors)
            .is_none_or(|s| s.id() != SymbolSequence::empty())
    {
        return Some(ObligationKind::IncompleteDomain);
    }
    let mut fields = d
        .symbolic_record_fields
        .iter()
        .filter(|f| f.class == symbol)
        .collect::<Vec<_>>();
    fields.sort_by_key(|f| f.ordinal);
    let mut syntaxes = d
        .field_syntax
        .iter()
        .filter(|s| same(d, s.class, class))
        .collect::<Vec<_>>();
    syntaxes.sort_by_key(|s| d.occurrences.get(s.target).map(|o| o.start));
    if fields.len() != syntaxes.len() {
        return Some(ObligationKind::MissingEvidence);
    }
    for (ordinal, field) in fields.iter().enumerate() {
        if field.ordinal != ordinal as i64
            || field.record != RecordKind::Dataclass
            || !field.declared
            || field.alias.is_some()
            || !exact(d, field.qualification, context)
            || !source(d, *field, &d.symbolic_record_supports, context)
        {
            return Some(ObligationKind::MissingEvidence);
        }
        let matches = syntaxes
            .iter()
            .filter(|s| {
                field
                    .declaration
                    .is_some_and(|site| same(d, s.target, site))
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Some(ObligationKind::MissingEvidence);
        }
        let syntax = matches[0];
        if syntax.id() != syntaxes[ordinal].id() {
            return Some(ObligationKind::IncompleteDomain);
        }
        let targets = d
            .placements
            .iter()
            .filter(|p| same(d, p.occurrence, syntax.target) && p.field == SyntaxField::Target)
            .collect::<Vec<_>>();
        if targets.len() != 1
            || !exact(d, targets[0].qualification, context)
            || !source(d, targets[0], &d.symbolic_placement_supports, context)
            || targets[0]
                .parent
                .is_none_or(|node| !direct_member(d, class, node, context))
        {
            return Some(ObligationKind::MissingEvidence);
        }
        if syntax.annotation.is_none()
            || !exact(d, syntax.qualification, context)
            || !source(d, *syntax, &d.symbolic_field_supports, context)
            || spelling(d, syntax.target, context) != Some(field.name.as_str())
            || !field_default(d, syntax, field, context, out)
        {
            return Some(ObligationKind::MissingEvidence);
        }
    }
    // A complete direct-body inventory is checked independently of provider field rows.
    for p in d
        .placements
        .iter()
        .filter(|p| p.parent.is_some_and(|p| same(d, p, class)) && p.field == SyntaxField::Body)
    {
        if !exact(d, p.qualification, context)
            || !source(d, p, &d.symbolic_placement_supports, context)
        {
            return Some(ObligationKind::MissingEvidence);
        }
        let Some(node) = d.occurrences.get(p.occurrence) else {
            return Some(ObligationKind::MissingEvidence);
        };
        if matches!(
            node.syntax_kind,
            SyntaxKind::StmtAnnAssign | SyntaxKind::StmtAssign
        ) {
            let Some(target) = child(d, node.id(), SyntaxField::Target, context) else {
                return Some(ObligationKind::MissingEvidence);
            };
            if syntaxes
                .iter()
                .filter(|s| same(d, s.target, target.occurrence))
                .count()
                != 1
            {
                return Some(ObligationKind::MissingEvidence);
            }
        } else if node.syntax_kind == SyntaxKind::StmtExpr {
            let Some(value) = child(d, node.id(), SyntaxField::Value, context) else {
                return Some(ObligationKind::MissingEvidence);
            };
            if d.occurrences
                .get(value.occurrence)
                .is_none_or(|o| o.syntax_kind != SyntaxKind::ExprStringLiteral)
            {
                return Some(ObligationKind::IncompleteDomain);
            }
        } else if !matches!(
            node.syntax_kind,
            SyntaxKind::StmtFunctionDef | SyntaxKind::StmtPass
        ) {
            return Some(ObligationKind::IncompleteDomain);
        }
    }
    for method in d
        .declarations
        .iter()
        .filter(|r| r.parent.is_some_and(|p| same(d, p, class)))
    {
        if !direct_member(d, class, method.declaration, context)
            || !exact(d, method.qualification, context)
            || !source(d, method, &d.symbolic_declaration_supports, context)
        {
            return Some(ObligationKind::MissingEvidence);
        }
        let Some(name) = spelling(d, method.name, context) else {
            return Some(ObligationKind::MissingEvidence);
        };
        if matches!(
            name,
            "__new__" | "__post_init__" | "__setattr__" | "__getattribute__" | "__getattr__"
        ) || fields.iter().any(|f| f.name.as_str() == name)
        {
            return Some(ObligationKind::IncompleteDomain);
        }
    }
    let Some(init) = initializer(d, symbol, context) else {
        return Some(ObligationKind::MissingEvidence);
    };
    if init.origin == FunctionOrigin::DefStatement {
        let declarations = d
            .declarations
            .iter()
            .filter(|m| {
                m.parent.is_some_and(|p| same(d, p, class))
                    && spelling(d, m.name, context) == Some("__init__")
            })
            .collect::<Vec<_>>();
        if declarations.len() != 1 {
            return Some(ObligationKind::MissingEvidence);
        }
        let function = declarations[0].declaration;
        if method(d, function, symbol, context).is_none() {
            return Some(ObligationKind::IncompleteDomain);
        }
        let Some(receiver) = receiver(d, function, context) else {
            return Some(ObligationKind::MissingEvidence);
        };
        if !plain_init(d, function, symbol, context, receiver) {
            return Some(ObligationKind::IncompleteDomain);
        }
    } else if init.origin == FunctionOrigin::Synthesized && generate_init {
        let Some(parameters) = generated_initializer_parameters(d, init.symbol, context) else {
            return Some(ObligationKind::MissingEvidence);
        };
        let active = fields
            .iter()
            .filter(|f| f.init == Some(true))
            .collect::<Vec<_>>();
        if parameters.len() != active.len() + 1 {
            return Some(ObligationKind::IncompleteDomain);
        }
        let Some(receiver) = d.symbolic_parameter_shapes.get(parameters[0].shape) else {
            return Some(ObligationKind::MissingEvidence);
        };
        if receiver.name.as_ref().is_none_or(|n| n.as_str() != "self")
            || !matches!(
                receiver.kind,
                ParameterKind::PositionalOnly | ParameterKind::PositionalOrKeyword
            )
            || !receiver.required
        {
            return Some(ObligationKind::IncompleteDomain);
        }
        // Native dataclass signatures group positional fields before keyword-only fields.
        let mut active = active;
        active.sort_by_key(|f| (f.kw_only.unwrap_or(kw_only), f.ordinal));
        for (parameter, field) in parameters.iter().skip(1).zip(active) {
            let Some(shape) = d.symbolic_parameter_shapes.get(parameter.shape) else {
                return Some(ObligationKind::MissingEvidence);
            };
            if shape
                .name
                .as_ref()
                .is_none_or(|n| n.as_str() != field.name.as_str())
                || shape.kind
                    != if field.kw_only.unwrap_or(kw_only) {
                        ParameterKind::KeywordOnly
                    } else {
                        ParameterKind::PositionalOrKeyword
                    }
                || shape.required != (field.has_default == Some(false))
            {
                return Some(ObligationKind::IncompleteDomain);
            }
        }
    } else {
        return Some(ObligationKind::IncompleteDomain);
    }
    None
}
pub(super) fn normalize(
    d: &AspectData,
    out: &mut AspectOutput,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let allowance = d
        .occurrences
        .len()
        .saturating_add(d.placements.len())
        .saturating_add(d.symbolic_record_fields.len())
        .saturating_mul(2048);
    let _scratch = b.reserve(
        "source-symbolic-field-check",
        allowance.saturating_add(1024),
    )?;
    let mut full_inventory = None;
    for declaration in d
        .declarations
        .iter()
        .filter(|r| r.kind == DeclarationKind::Class)
    {
        let Some(q) = d.qualifications.get(declaration.qualification) else {
            continue;
        };
        if !exact(d, q.id(), q.context)
            || !source(d, declaration, &d.symbolic_declaration_supports, q.context)
        {
            continue;
        }
        let class = declaration.declaration;
        let Some(symbol) = symbol_for(d, class, q.context) else {
            continue;
        };
        let mut traits = d.symbolic_class_traits.iter().filter(|t| {
            t.symbol == symbol
                && exact(d, t.qualification, q.context)
                && source(d, *t, &d.symbolic_class_supports, q.context)
        });
        let Some(class_traits) = traits.next() else {
            continue;
        };
        if traits.next().is_some() {
            continue;
        }
        let traits = class_traits;
        let Some(support) = d.symbolic_class_supports.iter().find(|s| {
            s.assertion == traits.id()
                && s.source_fidelity()
                && d.symbolic_runs
                    .get(s.run)
                    .is_some_and(|r| r.context == q.context)
        }) else {
            continue;
        };
        if full_inventory.is_none() {
            full_inventory = Some(ClassInventory::prepare(d, b)?);
        }
        let inventory = full_inventory
            .as_ref()
            .expect("prepared inventory")
            .for_class(class, q.context, symbol);
        let reason = if traits.dataclass && !traits.synthesized {
            record_gate(d, class, symbol, q.context, out)
        } else {
            Some(ObligationKind::IncompleteDomain)
        };
        let assessment = SourceFieldClass {
            class,
            qualification: q.id(),
            traits: traits.id(),
            support: support.id(),
            supported_record: reason.is_none(),
            reason,
            inventory,
        };
        out.symbolic_classes.insert(assessment.clone())?;
        for function in d.declarations.iter().filter(|r| {
            r.parent.is_some_and(|p| same(d, p, class))
                && r.kind == DeclarationKind::Function
                && exact(d, r.qualification, q.context)
        }) {
            let Some(traits) = method(d, function.declaration, symbol, q.context) else {
                continue;
            };
            let Some(receiver) = receiver(d, function.declaration, q.context) else {
                continue;
            };
            let is_init = d
                .spellings
                .iter()
                .any(|s| same(d, s.occurrence, function.name) && s.spelling.as_str() == "__init__");
            let plain = is_init && plain_init(d, function.declaration, symbol, q.context, receiver);
            for p in d.placements.iter().filter(|p| {
                contains(d, function.declaration, p.occurrence)
                    && exact(d, p.qualification, q.context)
            }) {
                let Some(name) = attribute(d, p.occurrence, receiver, q.context) else {
                    continue;
                };
                if p.field == SyntaxField::Target && is_init {
                    let Some(statement) = p.parent else { continue };
                    if d.occurrences
                        .get(statement)
                        .is_none_or(|s| s.syntax_kind != SyntaxKind::StmtAssign)
                    {
                        continue;
                    }
                    let Some(rhs) = child(d, statement, SyntaxField::Value, q.context) else {
                        continue;
                    };
                    let Some(formal) = d.symbolic_parameter_declarations.iter().find(|a| {
                        contains(d, function.declaration, a.declaration)
                            && !same(d, a.declaration, receiver.parameter)
                            && formal_read(d, rhs.occurrence, a.declaration, q.context)
                            && exact(d, a.qualification, q.context)
                            && source(d, *a, &d.symbolic_parameter_supports, q.context)
                    }) else {
                        continue;
                    };
                    out.symbolic_stores.insert(SourceFieldStore {
                        class,
                        constructor: function.declaration,
                        target: p.occurrence,
                        name,
                        value: rhs.occurrence,
                        formal: formal.id(),
                        receiver: receiver.id(),
                        placement: p.id(),
                        traits: traits.id(),
                        qualification: q.id(),
                        plain_initializer: plain,
                        inventory,
                    })?;
                } else if p.field != SyntaxField::Target && !is_init {
                    out.symbolic_readers.insert(SourceFieldReader {
                        class,
                        reader: function.declaration,
                        access: p.occurrence,
                        name,
                        receiver: receiver.id(),
                        placement: p.id(),
                        traits: traits.id(),
                        qualification: q.id(),
                        inventory,
                    })?;
                }
            }
        }
        if !assessment.supported_record {
            continue;
        }
        for f in d
            .symbolic_record_fields
            .iter()
            .filter(|f| f.class == symbol && exact(d, f.qualification, q.context))
        {
            for store in out.symbolic_stores.iter().filter(|s| {
                same(d, s.class, class) && s.name == f.name.as_str() && s.plain_initializer
            }) {
                let Some(formal) = d.symbolic_parameter_declarations.get(store.formal) else {
                    continue;
                };
                out.symbolic_associations.insert(SourceFieldAssociation {
                    class: assessment.id(),
                    field: f.id(),
                    parameter: formal.parameter,
                    store: Some(store.id()),
                    kind: SourceStorageKind::PlainInitializer,
                    qualification: q.id(),
                    inventory,
                })?;
            }
            // Native generated initializer parameters have no source assignment or source formal.
            if !d.declarations.iter().any(|m| {
                m.parent.is_some_and(|p| same(d, p, class))
                    && d.spellings
                        .iter()
                        .any(|s| same(d, s.occurrence, m.name) && s.spelling.as_str() == "__init__")
            }) {
                for signature in d.symbolic_signatures.iter().filter(|s| {
                    initializer(d, symbol, q.context).is_some_and(|i| {
                        i.origin == FunctionOrigin::Synthesized && i.symbol == s.symbol
                    }) && exact(d, s.qualification, q.context)
                        && source(d, *s, &d.symbolic_signature_supports, q.context)
                }) {
                    for p in d
                        .symbolic_parameters
                        .iter()
                        .filter(|p| p.signature == signature.id())
                    {
                        if f.init == Some(true)
                            && d.symbolic_parameter_shapes.get(p.shape).is_some_and(|s| {
                                s.name
                                    .as_ref()
                                    .is_some_and(|n| n.as_str() == f.name.as_str())
                            })
                        {
                            out.symbolic_associations.insert(SourceFieldAssociation {
                                class: assessment.id(),
                                field: f.id(),
                                parameter: p.id(),
                                store: None,
                                kind: SourceStorageKind::GeneratedRecord,
                                qualification: q.id(),
                                inventory,
                            })?;
                        }
                    }
                }
            }
        }
    }
    for association in out.symbolic_associations.iter() {
        let Some(class) = out.symbolic_classes.get(association.class) else {
            continue;
        };
        let Some(field) = d.symbolic_record_fields.get(association.field) else {
            continue;
        };
        for reader in out.symbolic_readers.iter().filter(|r| {
            same(d, r.class, class.class)
                && r.name == field.name.as_str()
                && r.qualification == association.qualification
        }) {
            out.symbolic_links.insert(SourceFieldReaderLink {
                association: association.id(),
                reader: reader.id(),
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod inventory_controls {
    use super::*;
    fn nominal<T>(value: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    #[test]
    fn full_inventory_matches_replay_and_unused_sibling_changes_invalidate() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut data = AspectData::new(&budget);
        let original_usage = budget.reserved();
        let class = nominal(1);
        let context = nominal(2);
        let symbol = nominal(3);
        let original = ClassInventory::prepare(&data, &budget).unwrap();
        let before = original.for_class(class, context, symbol);
        drop(original);
        assert_eq!(budget.reserved(), original_usage);
        // This occurrence is not a class or a member. It remains an inventory premise.
        data.occurrences
            .insert(Occurrence {
                source: nominal(4),
                start: 7,
                end: 9,
                syntax_kind: SyntaxKind::ExprName,
                structural_path: vec![7],
                role: OccurrenceRole::Syntax,
            })
            .unwrap();
        let rows_usage = budget.reserved();
        let prepared = ClassInventory::prepare(&data, &budget).unwrap();
        let after = prepared.for_class(class, context, symbol);
        assert_ne!(before, after);
        let mut old = KeySink::new("source-field-class-full-inventory");
        class.encode(&mut old);
        context.encode(&mut old);
        macro_rules! rows { ($($field:ident:$ty:ty,)*) => {$(
            for row in data.$field.iter() { row.content_digest().encode(&mut old); }
        )*}; }
        crate::callable_aspect_inputs!(rows);
        symbol.encode(&mut old);
        assert_eq!(after, old.finish());
        drop(prepared);
        assert_eq!(budget.reserved(), rows_usage);
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            ClassInventory::prepare(&data, &tiny),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(tiny.reserved(), 0);
    }
}
