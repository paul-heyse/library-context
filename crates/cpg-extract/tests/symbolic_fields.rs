#[path = "fixtures/transfer_composition.rs"]
mod fixture;
use lctx_model::domain::{
    flow::*,
    normalized::{Rows, callable_aspects::*, symbolic_fields::*},
    source::*,
    symbols::*,
    *,
};
fn data(f: &fixture::NativeFixture) -> AspectData {
    let mut data = AspectData::new(&f.budget);
    macro_rules! raw { ($($field:ident:$ty:ty,)*) => {$(for row in f.rows::<$ty>() {data.$field.insert(row).unwrap();})*}; }
    lctx_model::callable_aspect_inputs!(raw);
    macro_rules! normalized { ($($field:ident:$ty:ty,)*) => {$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*}; }
    lctx_model::normalized_binding_inputs!(normalized);
    let mut entities = normalized::entity_normalization::EntityData::new(&f.budget);
    macro_rules! facts { ($($field:ident:$ty:ty => $family:ident,)*) => {$(for row in f.rows::<$ty>() { entities.$field.insert(row).unwrap(); })*}; }
    lctx_model::normalized_entity_inputs!(facts);
    let entities =
        normalized::entity_normalization::normalize(entities.inputs(), &f.budget).unwrap();
    macro_rules! entities { ($($field:ident:$ty:ty,)*) => {$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*}; }
    lctx_model::normalized_entity_outputs!(entities);
    data
}
fn text(f: &fixture::NativeFixture, id: Id<Occurrence>) -> String {
    let o = f.data.occurrences.get(id).unwrap();
    String::from_utf8(f.source_bytes(o.source)[o.start as usize..o.end as usize].to_vec()).unwrap()
}
fn class<'a>(
    f: &fixture::NativeFixture,
    out: &'a AspectOutput,
    name: &str,
) -> &'a SourceFieldClass {
    out.symbolic_classes
        .iter()
        .find(|c| {
            let t = f
                .rows::<ClassTraitObservation>()
                .into_iter()
                .find(|t| t.id() == c.traits)
                .unwrap();
            f.data.symbols.get(t.symbol).unwrap().name == name
        })
        .unwrap()
}
fn admitted(f: &fixture::NativeFixture, out: &AspectOutput, name: &str) {
    let c = class(f, out, name);
    if !c.supported_record {
        diagnose_record_premises(f, out, c);
    }
    assert!(c.supported_record, "{name}: {c:?}");
    assert!(
        out.symbolic_associations.iter().any(|a| a.class == c.id()),
        "{name} has no associations"
    );
}
// Bounded failure-only diagnostics: report existing premises, never recreate a validator.
fn diagnose_record_premises(f: &fixture::NativeFixture, out: &AspectOutput, c: &SourceFieldClass) {
    let d = data(f);
    let symbol = d.symbolic_class_traits.get(c.traits).unwrap().symbol;
    eprintln!(
        "RECORD_GATE class={:?} symbol={symbol:?} qualification={:?}",
        c.class,
        d.qualifications.get(c.qualification)
    );
    for field in d
        .symbolic_record_fields
        .iter()
        .filter(|r| r.class == symbol)
        .take(16)
    {
        eprintln!(
            "RECORD_FIELD {field:?} qualification={:?}",
            d.qualifications.get(field.qualification)
        );
        for support in d
            .symbolic_record_supports
            .iter()
            .filter(|s| s.assertion == field.id())
            .take(4)
        {
            eprintln!("RECORD_FIELD_SUPPORT {support:?}");
        }
    }
    for syntax in d
        .field_syntax
        .iter()
        .filter(|s| s.class == c.class)
        .take(16)
    {
        eprintln!("RECORD_SYNTAX {syntax:?}");
        for placement in d
            .placements
            .iter()
            .filter(|p| p.occurrence == syntax.target)
            .take(4)
        {
            eprintln!("RECORD_TARGET_PLACEMENT {placement:?}");
        }
        for link in d
            .fields
            .iter()
            .filter(|l| l.declaration == syntax.id())
            .take(4)
        {
            let assessment = out.fields.iter().find(|a| a.declaration == link.id());
            eprintln!(
                "RECORD_DEFAULT_LINK {link:?} assessment={assessment:?} default={:?}",
                assessment.and_then(|a| out.defaults.get(a.default))
            );
        }
    }
    for init in d
        .traits
        .iter()
        .filter(|t| {
            t.defining_class == Some(symbol)
                && d.symbols
                    .get(t.symbol)
                    .is_some_and(|s| s.name == "__init__")
        })
        .take(4)
    {
        eprintln!("RECORD_INITIALIZER {init:?}");
        for signature in d
            .symbolic_signatures
            .iter()
            .filter(|s| s.symbol == init.symbol)
            .take(8)
        {
            eprintln!("RECORD_SIGNATURE {signature:?}");
            for parameter in d
                .symbolic_parameters
                .iter()
                .filter(|p| p.signature == signature.id())
                .take(16)
            {
                eprintln!(
                    "RECORD_PARAMETER {parameter:?} shape={:?}",
                    d.symbolic_parameter_shapes.get(parameter.shape)
                );
            }
        }
        for enumeration in d
            .symbolic_signature_enumerations
            .iter()
            .filter(|e| e.symbol == init.symbol)
            .take(4)
        {
            eprintln!("RECORD_ENUMERATION {enumeration:?}");
            for member in d
                .symbolic_signature_members
                .iter()
                .filter(|m| m.enumeration == enumeration.id())
                .take(8)
            {
                eprintln!("RECORD_ENUMERATION_MEMBER {member:?}");
            }
        }
    }
}
fn refused(f: &fixture::NativeFixture, out: &AspectOutput, name: &str) {
    let c = class(f, out, name);
    assert!(!c.supported_record, "{name}: {c:?}");
    assert!(
        !out.symbolic_associations.iter().any(|a| a.class == c.id()),
        "{name} leaked association"
    );
}
#[tokio::test]
async fn native_exact_record_links_and_plain_stores() {
    let f = fixture::native_from("transfer_alternatives").await;
    let d = data(&f);
    let out = normalize(&d, &f.budget).unwrap();
    admitted(&f, &out, "RecordHolder");
    refused(&f, &out, "Holder");
    refused(&f, &out, "ConditionalHolder");
    let record = class(&f, &out, "RecordHolder");
    let associations = out
        .symbolic_associations
        .iter()
        .filter(|a| a.class == record.id())
        .collect::<Vec<_>>();
    assert_eq!(associations.len(), 1);
    assert_eq!(associations[0].kind, SourceStorageKind::PlainInitializer);
    let links = out
        .symbolic_links
        .iter()
        .filter(|l| l.association == associations[0].id())
        .collect::<Vec<_>>();
    assert_eq!(
        links.len(),
        3,
        "each syntactically distinct conditional reader survives"
    );
    for link in links {
        let reader = out.symbolic_readers.get(link.reader).unwrap();
        assert_eq!(text(&f, reader.access), "self.value");
        let uses = f.rows::<FlowUse>();
        let native = uses
            .iter()
            .filter(|u| u.occurrence == reader.access)
            .collect::<Vec<_>>();
        assert_eq!(native.len(), 1, "native reader retains ExprAttribute site");
        assert!(
            f.rows::<FlowValueObservation>()
                .iter()
                .any(|v| v.use_ == native[0].id())
        );
        eprintln!(
            "SOURCE READER {:?} NATIVE USE {:?}",
            reader.access, native[0]
        );
    }
    for name in ["RecordHolder", "Holder", "ConditionalHolder"] {
        let c = class(&f, &out, name);
        assert_eq!(
            out.symbolic_stores
                .iter()
                .filter(|s| s.class == c.class)
                .count(),
            1,
            "{name} source store remains observable"
        );
    }
    let store = out
        .symbolic_stores
        .iter()
        .find(|s| s.class == record.class)
        .unwrap();
    let values = f.rows::<FlowValueObservation>();
    let uses = f.rows::<FlowUse>();
    let value = values
        .iter()
        .find(|v| {
            v.kind == FlowSinkKind::Definition
                && v.transfer == transfer::TransferKind::Identity
                && !v.through_call
                && v.sink == store.value
                && uses
                    .iter()
                    .any(|u| u.id() == v.use_ && u.occurrence == store.value)
        })
        .expect("actual native unchanged RHS receipt");
    let usage = uses.iter().find(|u| u.id() == value.use_).unwrap();
    let places = f.rows::<value::Place>();
    let roots = f.rows::<value::PlaceRoot>();
    let place = places.iter().find(|p| p.id() == usage.place).unwrap();
    assert!(
        matches!(roots.iter().find(|r|r.id()==place.root),Some(value::PlaceRoot::Formal{declaration}) if *declaration==d.symbolic_parameter_declarations.get(store.formal).unwrap().declaration)
    );
    eprintln!("UNCHANGED STORE {store:?} VALUE {value:?} USE {usage:?}");
    for definition in f.rows::<FlowDefinition>() {
        if definition.occurrence == store.target || definition.occurrence == store.value {
            eprintln!("STORE DEFINITION {definition:?}");
        }
    }
}
#[tokio::test]
async fn native_generated_and_direct_controls_do_not_leak_fields() {
    let f = fixture::native_from("catalog").await;
    let d = data(&f);
    let out = normalize(&d, &f.budget).unwrap();
    for name in ["Options", "DirectOptions"] {
        admitted(&f, &out, name);
    }
    for name in [
        "CustomAllocation",
        "CustomMeta",
        "ReplacedRecord",
        "DescriptorField",
        "SetterMutation",
        "OperatorMutation",
        "StaticConstructor",
    ] {
        refused(&f, &out, name);
    }
    let c = class(&f, &out, "Options");
    let associations = out
        .symbolic_associations
        .iter()
        .filter(|a| a.class == c.id())
        .collect::<Vec<_>>();
    assert_eq!(associations.len(), 3);
    let left = associations
        .iter()
        .find(|a| d.symbolic_record_fields.get(a.field).unwrap().name == "left")
        .unwrap();
    assert_eq!(left.kind, SourceStorageKind::GeneratedRecord);
    assert!(left.store.is_none());
    let links = out
        .symbolic_links
        .iter()
        .filter(|l| l.association == left.id())
        .collect::<Vec<_>>();
    assert_eq!(links.len(), 1);
    let reader = out.symbolic_readers.get(links[0].reader).unwrap();
    assert!(text(&f, reader.reader).starts_with("def read_left("));
    assert_eq!(text(&f, reader.access), "self.left");
}
fn retain<R: Record>(
    rows: &mut Rows<R>,
    budget: &resources::ResourceBudget,
    keep: impl Fn(&R) -> bool,
) {
    let retained = rows.iter().filter(|r| keep(r)).cloned().collect::<Vec<_>>();
    *rows = Rows::new(budget);
    for row in retained {
        rows.insert(row).unwrap();
    }
}
fn replay(
    d: &AspectData,
    out: &AspectOutput,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let invariant = normalized::callable_aspects::invariants().remove(0);
    let mut check = (invariant.create)(budget);
    macro_rules! input { ($($field:ident:$ty:ty,)*) => {$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.$field.iter().cloned().collect::<Vec<_>>())?)?;)*}; }
    lctx_model::callable_aspect_inputs!(input);
    macro_rules! output { ($($field:ident:$ty:ty,)*) => {$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&out.$field.iter().cloned().collect::<Vec<_>>())?)?;)*}; }
    lctx_model::callable_aspect_outputs!(output);
    check.finish()
}
#[tokio::test]
async fn native_premise_removal_and_coupled_membership_corruption_refuse() {
    let f = fixture::native_from("catalog").await;
    let original = data(&f);
    let out = normalize(&original, &f.budget).unwrap();
    admitted(&f, &out, "Options");
    replay(&original, &out, &f.budget).unwrap();
    let c = class(&f, &out, "Options");
    let class_symbol = original.symbolic_class_traits.get(c.traits).unwrap().symbol;
    let left = original
        .symbolic_record_fields
        .iter()
        .find(|r| r.class == class_symbol && r.name == "left")
        .unwrap();
    let initializer = original
        .traits
        .iter()
        .find(|t| {
            t.defining_class == Some(class_symbol)
                && original
                    .symbols
                    .get(t.symbol)
                    .is_some_and(|s| s.name == "__init__")
        })
        .unwrap()
        .symbol;
    let signature = original
        .symbolic_signatures
        .iter()
        .find(|s| s.symbol == initializer && s.role == calls::SignatureRole::Synthesized)
        .unwrap()
        .clone();
    assert!(
        !original
            .symbolic_signature_enumerations
            .iter()
            .any(|e| e.symbol == initializer)
    );
    let native = original
        .symbolic_native_signatures
        .iter()
        .find(|n| n.signature == signature.id())
        .unwrap()
        .clone();
    assert!(native.complete);
    for mutation in 0..22 {
        let mut d = data(&f);
        match mutation {
            0 => retain(&mut d.symbolic_record_supports, &f.budget, |s| {
                s.assertion != left.id()
            }),
            1 => retain(&mut d.symbolic_record_fields, &f.budget, |r| {
                r.id() != left.id()
            }),
            2 => {
                // Removing both provider and source field inventories still leaves its actual
                // source-body placement, so a vacuous subset is never admitted as complete.
                retain(&mut d.symbolic_record_fields, &f.budget, |r| {
                    r.class != class_symbol
                });
                retain(&mut d.field_syntax, &f.budget, |r| r.class != c.class);
                let ids = d.field_syntax.iter().map(Record::id).collect::<Vec<_>>();
                retain(&mut d.fields, &f.budget, |r| ids.contains(&r.declaration));
            }
            3 => d.symbolic_target_supports = Rows::new(&f.budget),
            4 => d.symbolic_import_supports = Rows::new(&f.budget),
            5 => d.symbolic_detail_supports = Rows::new(&f.budget),
            6 => {
                let init = d
                    .traits
                    .iter()
                    .find(|t| {
                        t.defining_class == Some(class_symbol)
                            && d.symbols
                                .get(t.symbol)
                                .is_some_and(|s| s.name == "__init__")
                    })
                    .unwrap()
                    .symbol;
                let signature = d
                    .symbolic_signatures
                    .iter()
                    .find(|s| s.symbol == init)
                    .unwrap()
                    .id();
                retain(&mut d.symbolic_parameters, &f.budget, |p| {
                    p.signature != signature || p.ordinal != 1
                });
            }
            7 => {
                let mut changed = left.clone();
                changed.declaration = d
                    .symbolic_record_fields
                    .iter()
                    .find(|r| r.class == class_symbol && r.name == "right")
                    .unwrap()
                    .declaration;
                retain(&mut d.symbolic_record_fields, &f.budget, |r| {
                    r.id() != left.id()
                });
                d.symbolic_record_fields.insert(changed).unwrap();
            }
            8 => {
                let supports = d
                    .symbolic_signature_supports
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                d.symbolic_signature_supports = Rows::new(&f.budget);
                for mut support in supports {
                    support.mode = attribution::ExtractionMode::ReportDecode;
                    d.symbolic_signature_supports.insert(support).unwrap();
                }
            }
            9 => {
                let supports = d
                    .symbolic_resolution_supports
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                d.symbolic_resolution_supports = Rows::new(&f.budget);
                for mut support in supports {
                    support.fidelity = attribution::Fidelity::NativeStructural;
                    d.symbolic_resolution_supports.insert(support).unwrap();
                }
            }
            10 => {
                let supports = d
                    .symbolic_binding_supports
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                d.symbolic_binding_supports = Rows::new(&f.budget);
                for mut support in supports {
                    support.mode = attribution::ExtractionMode::NativeTraversal;
                    d.symbolic_binding_supports.insert(support).unwrap();
                }
            }
            11 => retain(&mut d.symbolic_native_signatures, &f.budget, |n| {
                n.signature != signature.id()
            }),
            21 => retain(&mut d.symbolic_native_signature_supports, &f.budget, |n| {
                n.assertion != native.id()
            }),
            12 | 15 | 16 | 18 => {
                let mut changed = native.clone();
                match mutation {
                    12 => changed.complete = false,
                    15 => {
                        changed.term = d
                            .symbolic_native_signatures
                            .iter()
                            .find(|n| n.term != native.term)
                            .unwrap()
                            .term
                    }
                    16 => {
                        changed.scope = d
                            .symbolic_scopes
                            .iter()
                            .find(|s| s.id() != native.scope)
                            .unwrap()
                            .id()
                    }
                    18 => {
                        changed.qualification = d
                            .qualifications
                            .iter()
                            .find(|q| q.id() != native.qualification)
                            .unwrap()
                            .id()
                    }
                    _ => unreachable!(),
                }
                retain(&mut d.symbolic_native_signatures, &f.budget, |n| {
                    n.id() != native.id()
                });
                d.symbolic_native_signatures.insert(changed).unwrap();
            }
            13 | 14 | 19 | 20 => {
                let supports = d
                    .symbolic_native_signature_supports
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                d.symbolic_native_signature_supports = Rows::new(&f.budget);
                for mut support in supports {
                    if support.assertion == native.id() {
                        match mutation {
                            13 => support.fidelity = attribution::Fidelity::ReportProjection,
                            14 => support.mode = attribution::ExtractionMode::Recognizer,
                            19 => {
                                support.run = d
                                    .symbolic_runs
                                    .iter()
                                    .find(|r| r.id() != support.run)
                                    .unwrap()
                                    .id()
                            }
                            20 => support.origin = attribution::Origin::SourceObservation,
                            _ => unreachable!(),
                        }
                    }
                    d.symbolic_native_signature_supports
                        .insert(support)
                        .unwrap();
                }
            }
            17 => {
                let mut changed = signature.clone();
                changed.variant += 1;
                d.symbolic_signatures.insert(changed).unwrap();
            }
            _ => unreachable!(),
        }
        let changed = normalize(&d, &f.budget).unwrap();
        refused(&f, &changed, "Options");
        assert!(
            replay(&d, &out, &f.budget).is_err(),
            "mutation {mutation} preserved stale admission"
        );
        replay(&d, &changed, &f.budget).unwrap();
    }
}
#[tokio::test]
async fn native_standard_target_options_and_reader_controls() {
    let f = fixture::native_from("symbolic_fields").await;
    let d = data(&f);
    let out = normalize(&d, &f.budget).unwrap();
    for name in ["KeywordOptions", "AliasOptions", "QualifiedOptions"] {
        admitted(&f, &out, name);
    }
    for name in [
        "ReplacedAllocation",
        "DynamicOptions",
        "DynamicDefault",
        "ConditionalRecord",
        "RewrittenRecord",
    ] {
        refused(&f, &out, name);
    }
    let unsupported = out
        .symbolic_classes
        .iter()
        .find(|c| text(&f, c.class).contains("class NonstandardTarget:"));
    assert!(unsupported.is_none_or(|c| !c.supported_record));
    let c = class(&f, &out, "DecoratedReader");
    assert!(c.supported_record);
    let associations = out
        .symbolic_associations
        .iter()
        .filter(|a| a.class == c.id())
        .map(Record::id)
        .collect::<Vec<_>>();
    assert!(
        !out.symbolic_links
            .iter()
            .any(|l| associations.contains(&l.association))
    );
    for reference in d
        .references
        .iter()
        .filter(|r| r.name == "record" || r.name == "records")
    {
        let support = d
            .symbolic_reference_supports
            .iter()
            .find(|s| s.assertion == reference.id())
            .unwrap();
        assert_eq!(
            support.fidelity,
            attribution::Fidelity::NormalizedStructural
        );
        assert_eq!(support.mode, attribution::ExtractionMode::Recognizer);
        assert_eq!(support.origin, attribution::Origin::DerivedAnalysis);
    }
    let bare = d
        .targets
        .iter()
        .filter(|t| text(&f, t.site) == "record")
        .collect::<Vec<_>>();
    assert!(!bare.is_empty());
    assert!(
        bare.iter()
            .all(|t| d.qualifications.get(t.qualification).unwrap().modality
                == attribution::Modality::Potential),
        "standard source target recognition never upgrades application execution"
    );
    replay(&d, &out, &f.budget).unwrap();
}
#[tokio::test]
async fn native_store_boundary_has_the_actual_rhs_premise() {
    let f = fixture::native_from("transfer_alternatives").await;
    let d = data(&f);
    let class = d
        .declarations
        .iter()
        .find(|r| {
            r.kind == syntax::DeclarationKind::Class
                && d.spellings
                    .iter()
                    .any(|s| s.occurrence == r.name && s.spelling.as_str() == "RecordHolder")
        })
        .unwrap();
    let init = d
        .declarations
        .iter()
        .find(|r| {
            r.parent == Some(class.declaration)
                && d.spellings
                    .iter()
                    .any(|s| s.occurrence == r.name && s.spelling.as_str() == "__init__")
        })
        .unwrap();
    let body = d.occurrences.get(init.declaration).unwrap();
    let target = d
        .placements
        .iter()
        .find(|p| {
            p.field == lexical::SyntaxField::Target
                && text(&f, p.occurrence) == "self.value"
                && d.occurrences.get(p.occurrence).is_some_and(|o| {
                    o.source == body.source && o.start >= body.start && o.end <= body.end
                })
        })
        .unwrap();
    let rhs = d
        .placements
        .iter()
        .find(|p| p.parent == target.parent && p.field == lexical::SyntaxField::Value)
        .unwrap();
    assert_eq!(text(&f, rhs.occurrence), "value");
    let receiver = d
        .placements
        .iter()
        .find(|p| p.parent == Some(target.occurrence) && p.field == lexical::SyntaxField::Value)
        .unwrap();
    let receiver_uses = f
        .rows::<FlowUse>()
        .into_iter()
        .filter(|u| u.occurrence == receiver.occurrence)
        .collect::<Vec<_>>();
    assert_eq!(
        receiver_uses.len(),
        1,
        "assignment receiver has one actual native read"
    );
    eprintln!("RAW STORE RECEIVER {receiver:?} USES {receiver_uses:?}");
    let uses = f.rows::<FlowUse>();
    let values = f.rows::<FlowValueObservation>();
    let value = values
        .iter()
        .find(|v| {
            v.kind == FlowSinkKind::Definition
                && v.transfer == transfer::TransferKind::Identity
                && !v.through_call
                && v.sink == rhs.occurrence
                && uses
                    .iter()
                    .any(|u| u.id() == v.use_ && u.occurrence == rhs.occurrence)
        })
        .unwrap();
    let usage = uses.iter().find(|u| u.id() == value.use_).unwrap();
    eprintln!("RAW STORE TARGET {target:?} RHS {rhs:?} VALUE {value:?} USE {usage:?}");
    let definitions = f
        .rows::<FlowDefinition>()
        .into_iter()
        .filter(|v| v.occurrence == target.occurrence || v.occurrence == rhs.occurrence)
        .collect::<Vec<_>>();
    eprintln!("RAW STORE FIELD DEFINITIONS {definitions:?}");
    let resolution = d
        .lexical_resolutions
        .iter()
        .find(|r| r.read == rhs.occurrence)
        .unwrap();
    let support = d
        .symbolic_resolution_supports
        .iter()
        .find(|s| {
            s.assertion == resolution.id()
                && s.origin == attribution::Origin::DerivedAnalysis
                && s.mode == attribution::ExtractionMode::Recognizer
                && s.fidelity == attribution::Fidelity::NormalizedStructural
        })
        .unwrap();
    assert_eq!(
        support.fidelity,
        attribution::Fidelity::NormalizedStructural
    );
    assert_eq!(support.mode, attribution::ExtractionMode::Recognizer);
    assert_eq!(support.origin, attribution::Origin::DerivedAnalysis);
    assert!(
        d.symbolic_resolution_supports
            .iter()
            .any(|s| s.assertion == resolution.id()
                && s.origin == attribution::Origin::AnalyzerAssertion
                && s.mode == attribution::ExtractionMode::NativeTraversal
                && s.fidelity == attribution::Fidelity::NativeStructural),
        "the separate native Ruff resolution support remains attributed"
    );
    assert_eq!(
        definitions.len(),
        1,
        "actual field-target definition is retained"
    );
    let places = f.rows::<value::Place>();
    let definition_place = places
        .iter()
        .find(|p| p.id() == definitions[0].place)
        .unwrap();
    let receiver_place = places
        .iter()
        .find(|p| p.id() == receiver_uses[0].place)
        .unwrap();
    assert_eq!(
        definition_place.root, receiver_place.root,
        "target is an attribute of the actual receiver formal"
    );
    let roots = f.rows::<value::PlaceRoot>();
    let root = roots
        .iter()
        .find(|r| r.id() == definition_place.root)
        .unwrap();
    assert!(matches!(root,value::PlaceRoot::Formal{declaration} if text(&f,*declaration)=="self"));
    let paths = f.rows::<value::AccessPath>();
    let path = paths
        .iter()
        .find(|p| p.id() == definition_place.path)
        .unwrap();
    let segments = f.rows::<value::PathSegment>();
    assert!(
        matches!(path.first.and_then(|id|segments.iter().find(|s|s.id()==id)),Some(value::PathSegment::Attribute{name}) if name=="value")
    );
    assert!(path.second.is_none() && !path.unknown_suffix);
    eprintln!("RAW STORE DEFINITION ROOT {root:?} PATH {path:?}");
}
