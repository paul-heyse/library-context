//! Actual source/effective signature packet closure, original nominal IDs, and indivisible limits.
#[path = "fixtures/serving_support.rs"]
mod support;
use lctx_model::domain::{
    Record,
    calls::ParameterKind,
    catalog::{CatalogDefault, CatalogMember, CatalogOption, CatalogOptionSubject},
    serving::*,
};
use lctx_postgres::generations::Error;
use support::*;

#[tokio::test]
async fn analytical_enrichment_serves_real_comparison_context_and_non_call_references() {
    use lctx_model::domain::{
        calls::SignatureRole, lexical::SyntaxField, normalized::contract_comparison::Difference,
    };
    let source = br#"from typing import Callable
from dataclasses import dataclass
__all__=['decorate','changed','typed','Settings']
def decorate(fn):
    return fn
@decorate
def unchanged(value: int) -> int:
    return value
saved = decorate
saved_again = decorate
def shadow(decorate):
    return decorate
def wrap(fn: Callable[[int],int]):
    def implementation(text: str) -> bytes:
        return text.encode()
    return implementation
@wrap
def changed(value: int) -> int:
    return value
def typed() -> list[int]:
    result: list[int] = []
    consume([])
    return result
def consume(values: list[int]) -> None:
    pass
@dataclass
class Settings:
    timeout: int = 3
    title: str = 'title'
    def read_timeout(self):
        return self.timeout
    def read_title(self):
        return self.title
"#;
    let fixture = ServingFixture::start(source).await;
    println!(
        "API_PACKET_MODEL={} GENERATION={}",
        fixture.store.model().digest().hex(),
        fixture.generation.hex()
    );
    let execution = fixture.service.execution().await.unwrap();
    let request = |value: &str, sections| GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path(value),
        comparison: Optional::default(),
        reference_parameter: Optional::default(),
        sections,
        page: PageRequest {
            expanded: true,
            ..Default::default()
        },
    };
    let mut exchanges = Vec::new();
    let exchange = |request: &GetOperationRequest,
                    response: &GetOperationResponse,
                    sections: &[&str],
                    control: &str| serde_json::json!({"request":request,"native":response,"sections":sections,"control":control});
    let initial = request("demo.changed", vec![]);
    let OperationResolution::Unique { packet } = fixture
        .catalog
        .operation(&execution, &initial)
        .await
        .unwrap()
        .operation
    else {
        panic!("changed missing")
    };
    let left = packet
        .core
        .signatures
        .iter()
        .find(|s| s.role == SignatureRole::Source)
        .unwrap();
    let right = packet
        .core
        .signatures
        .iter()
        .find(|s| s.role == SignatureRole::EffectiveTyped)
        .unwrap();
    let mut comparison = request("demo.changed", vec![OperationSection::CallableComparison]);
    comparison.comparison = Optional(Some(CallableComparisonRequest {
        analysis: left.analysis,
        left: left.variant,
        right: right.variant,
    }));
    let response = fixture
        .catalog
        .operation(&execution, &comparison)
        .await
        .unwrap();
    exchanges.push(exchange(
        &comparison,
        &response,
        &["callable_comparison"],
        "comparison",
    ));
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("comparison missing")
    };
    let answer = &packet.callable_comparison.items[0];
    assert_eq!(answer.left_role, SignatureRole::Source);
    assert_eq!(answer.right_role, SignatureRole::EffectiveTyped);
    assert_eq!(
        answer.ports[0].layout,
        Difference::DifferentRetainedStructure {}
    );
    assert!(!answer.proof.is_empty());
    let typing = request("demo.typed", vec![OperationSection::ContextualTyping]);
    let response = fixture
        .catalog
        .operation(&execution, &typing)
        .await
        .unwrap();
    exchanges.push(exchange(
        &typing,
        &response,
        &["contextual_typing"],
        "context_unknown",
    ));
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("typing missing")
    };
    assert!(!packet.contextual_typing.items.is_empty());
    assert!(
        packet
            .contextual_typing
            .items
            .iter()
            .any(|t| !t.actual.is_empty() && !t.expected.is_empty())
    );
    let argument_start = std::str::from_utf8(source)
        .unwrap()
        .find("consume([])")
        .unwrap()
        + "consume(".len();
    assert!(
        packet
            .contextual_typing
            .items
            .iter()
            .any(|t| t.start == argument_start as i64
                && !t.actual.is_empty()
                && !t.expected.is_empty()),
        "original argument retains native actual and Expected observations"
    );
    assert!(
        packet
            .contextual_typing
            .items
            .iter()
            .all(|t| !t.error_recovery_known && !t.proof.is_empty())
    );
    let mut references = request("demo.decorate", vec![OperationSection::IncomingReferences]);
    references.page.size = 1;
    let response = fixture
        .catalog
        .operation(&execution, &references)
        .await
        .unwrap();
    exchanges.push(exchange(
        &references,
        &response,
        &["incoming_references"],
        "omitted_truncated",
    ));
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("references missing")
    };
    let scope = packet.reference_scope.0.unwrap();
    assert!(scope.lexical_names_only && scope.external_consumers_unknown);
    assert!(!scope.artifacts.is_empty());
    assert_eq!(packet.incoming_references.items.len(), 1);
    let mut incoming = packet.incoming_references.items;
    let mut cursor = packet.incoming_references.continuation;
    while cursor.0.is_some() {
        references.page.cursor = cursor;
        let OperationResolution::Unique { packet } = fixture
            .catalog
            .operation(&execution, &references)
            .await
            .unwrap()
            .operation
        else {
            panic!("reference continuation missing")
        };
        assert_eq!(packet.reference_scope.0.unwrap().identity, scope.identity);
        incoming.extend(packet.incoming_references.items);
        cursor = packet.incoming_references.continuation;
    }
    assert_eq!(
        incoming.len(),
        3,
        "decorator and two value uses; shadowed formal is another entity"
    );
    // A decorator wrapper sits in FunctionDef.Decorator; its ExprName is an
    // immediate Child of that wrapper. Preserve the actual placement field and
    // prove decorator participation through the canonical source ancestry.
    let decorator_start = std::str::from_utf8(source)
        .unwrap()
        .find("@decorate")
        .unwrap()
        + 1;
    let decorator = incoming
        .iter()
        .find(|r| {
            r.start == decorator_start as i64
                && r.end == (decorator_start + "decorate".len()) as i64
        })
        .expect("exact original decorator name is an incoming reference");
    let lexical = execution
        .read::<lctx_model::domain::lexical::ReferenceObservation>()
        .await
        .unwrap();
    let syntax = execution
        .read::<lctx_model::domain::source::Occurrence>()
        .await
        .unwrap();
    let placements = execution
        .read::<lctx_model::domain::syntax::SyntaxPlacement>()
        .await
        .unwrap();
    let reference = lexical
        .rows()
        .iter()
        .find(|r| r.id() == decorator.reference)
        .unwrap();
    let parent = syntax
        .rows()
        .iter()
        .find(|o| o.id() == reference.parent)
        .unwrap();
    assert_eq!(
        parent.syntax_kind,
        lctx_model::domain::source::SyntaxKind::Decorator
    );
    assert_eq!(decorator.field, reference.field);
    assert_eq!(decorator.field, SyntaxField::Child);
    assert!(
        placements
            .rows()
            .iter()
            .any(|p| p.occurrence == reference.parent
                && p.parent.is_some()
                && p.field == SyntaxField::Decorator)
    );
    assert!(
        incoming
            .iter()
            .all(|r| !r.call_target_syntax && !r.proof.is_empty())
    );
    let fields = request("demo.Settings", vec![OperationSection::Relationships]);
    let response = fixture
        .catalog
        .operation(&execution, &fields)
        .await
        .unwrap();
    exchanges.push(exchange(
        &fields,
        &response,
        &["relationships"],
        "runtime_unknown",
    ));
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("Settings missing")
    };
    use lctx_model::domain::{calls, catalog, normalized, types, value};
    macro_rules! stored {($($name:ident:$ty:ty,)*)=>{$(let $name=execution.read::<$ty>().await.unwrap();)*};}
    stored! {
        options:catalog::CatalogOption,
        subjects:catalog::CatalogOptionSubject,
        option_evidence:catalog::CatalogOptionEvidence,
        defaults:catalog::CatalogDefault,
        literals:value::Literal,
        associations:normalized::symbolic_fields::SourceFieldAssociation,
        reader_links:normalized::symbolic_fields::SourceFieldReaderLink,
        readers:normalized::symbolic_fields::SourceFieldReader,
        fields:types::RecordFieldObservation,
        field_links:normalized::entities::FieldEntityLink,
        declarations:normalized::entities::FieldDeclarationLink,
        default_assessments:normalized::callable_aspects::FieldDefaultAssessment,
        parameters:calls::SignatureParameter,
        signatures:calls::Signature,
        slots:normalized::callables::SignatureSlot,
    }
    let mut pairs = std::collections::BTreeMap::new();
    let mut names = std::collections::BTreeSet::new();
    let mut count = 0;
    for relationship in &packet.relationships.items {
        let RelationshipPacket::SourceField {
            association,
            reader_link,
            reader,
            access,
            parameter,
            parameter_option,
            field_option,
            source_association,
            runtime_value,
            proof,
            ..
        } = relationship
        else {
            continue;
        };
        count += 1;
        assert_eq!(*source_association, normalized::callables::Knowledge::Known);
        assert_eq!(*runtime_value, normalized::callables::Knowledge::Unknown);
        assert!(!proof.is_empty());
        let association_row = associations
            .rows()
            .iter()
            .find(|r| r.id() == *association)
            .unwrap();
        assert_eq!(association_row.parameter, *parameter);
        let link = reader_links
            .rows()
            .iter()
            .find(|r| r.id() == *reader_link)
            .unwrap();
        assert_eq!((link.association, link.reader), (*association, *reader));
        let reader_row = readers.rows().iter().find(|r| r.id() == *reader).unwrap();
        assert_eq!(reader_row.access, *access);
        let field = fields
            .rows()
            .iter()
            .find(|r| r.id() == association_row.field)
            .unwrap();
        assert_eq!(reader_row.name, field.name.as_str());
        names.insert(reader_row.name.clone());
        let parameter_option_row = options
            .rows()
            .iter()
            .find(|r| r.id() == *parameter_option)
            .unwrap();
        let catalog::CatalogOptionSubject::Parameter { slot } = subjects
            .rows()
            .iter()
            .find(|r| r.id() == parameter_option_row.subject)
            .unwrap()
        else {
            panic!("generated initializer must retain its native parameter slot")
        };
        assert_eq!(
            option_evidence
                .rows()
                .iter()
                .find(|r| r.id() == parameter_option_row.evidence)
                .unwrap(),
            &catalog::CatalogOptionEvidence::NativeParameter { slot: *slot }
        );
        let slot = slots.rows().iter().find(|r| r.id() == *slot).unwrap();
        assert_eq!(slot.parameter, *parameter);
        let parameter_row = parameters
            .rows()
            .iter()
            .find(|r| r.id() == *parameter)
            .unwrap();
        let signature = signatures
            .rows()
            .iter()
            .find(|r| r.id() == parameter_row.signature)
            .unwrap();
        assert_eq!(signature.role, calls::SignatureRole::Synthesized);
        assert!(signature.native.is_some());
        let field_option_row = options
            .rows()
            .iter()
            .find(|r| r.id() == *field_option)
            .unwrap();
        assert_eq!(field_option_row.member, parameter_option_row.member);
        let catalog::CatalogOptionSubject::Field {
            field: field_entity,
        } = subjects
            .rows()
            .iter()
            .find(|r| r.id() == field_option_row.subject)
            .unwrap()
        else {
            panic!("source association must address the exact field")
        };
        let default = defaults
            .rows()
            .iter()
            .find(|r| r.id() == field_option_row.default)
            .unwrap();
        let purpose = match option_evidence
            .rows()
            .iter()
            .find(|r| r.id() == field_option_row.evidence)
            .unwrap()
        {
            catalog::CatalogOptionEvidence::DeclaredField {
                declaration,
                assessment,
            } => {
                let declaration_row = declarations
                    .rows()
                    .iter()
                    .find(|r| r.id() == *declaration)
                    .unwrap();
                assert_eq!(declaration_row.field, *field_entity);
                let assessment_row = default_assessments
                    .rows()
                    .iter()
                    .find(|r| r.id() == *assessment)
                    .unwrap();
                assert_eq!(assessment_row.declaration, *declaration);
                let catalog::CatalogDefault::Literal { literal } = default else {
                    panic!("declared field retains its actual original default")
                };
                let literal = literals.rows().iter().find(|r| r.id() == *literal).unwrap();
                match reader_row.name.as_str() {
                    "timeout" => assert_eq!(
                        literal,
                        &value::Literal::Integer {
                            decimal: "3".into()
                        }
                    ),
                    "title" => assert_eq!(
                        literal,
                        &value::Literal::String {
                            value: "title".into()
                        }
                    ),
                    _ => panic!("unexpected generated field reader"),
                }
                0
            }
            catalog::CatalogOptionEvidence::NativeField { link, observation } => {
                assert_eq!(*observation, association_row.field);
                let link_row = field_links.rows().iter().find(|r| r.id() == *link).unwrap();
                assert_eq!(
                    (link_row.field, link_row.observation),
                    (*field_entity, *observation)
                );
                assert_eq!(default, &catalog::CatalogDefault::Unknown {});
                1
            }
            _ => panic!("source field requires declared or native field evidence"),
        };
        let pair = pairs
            .entry((*association, *reader_link))
            .or_insert_with(|| {
                (
                    *parameter,
                    *parameter_option,
                    *field_entity,
                    std::collections::BTreeSet::new(),
                )
            });
        assert_eq!(
            (pair.0, pair.1, pair.2),
            (*parameter, *parameter_option, *field_entity)
        );
        assert!(
            pair.3.insert(purpose),
            "same field evidence purpose must not be repeated"
        );
    }
    assert_eq!(
        names,
        std::collections::BTreeSet::from(["timeout".to_owned(), "title".to_owned()])
    );
    assert_eq!(pairs.len(), 2, "two exact parameter-to-reader associations");
    assert_eq!(
        count, 4,
        "each pair preserves declared and native field evidence"
    );
    assert!(
        pairs
            .values()
            .all(|pair| pair.3 == std::collections::BTreeSet::from([0, 1]))
    );
    assert_eq!(
        pairs
            .values()
            .map(|pair| pair.0)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
    assert_eq!(
        pairs
            .values()
            .map(|pair| pair.2)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
    let empty = request("demo.Settings", vec![OperationSection::IncomingReferences]);
    let response = fixture.catalog.operation(&execution, &empty).await.unwrap();
    let OperationResolution::Unique { packet } = &response.operation else {
        panic!("empty reference member missing")
    };
    assert!(
        packet.incoming_references.items.is_empty(),
        "declared class has no represented incoming lexical-name reads"
    );
    exchanges.push(exchange(
        &empty,
        &response,
        &["incoming_references"],
        "empty",
    ));
    let spec = fixture.dir.path().join("enrichment-stdio.json");
    write(&spec,serde_json::to_vec(&serde_json::json!({"config":fixture.dir.path().join("postgres-serving.json"),"generation":fixture.generation.hex(),"unknown_label":lctx_model::domain::normalized::callables::Knowledge::Unknown,"cases":exchanges})).unwrap());
    let python_root = std::env::var_os("LCTX_T0_PYTHON_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let python_paths = std::env::join_paths([
        python_root.join("python/lctx_mcp/src"),
        python_root.join("python/lctx_storage/python"),
        python_root.join("python/lctx_semantics/python"),
    ])
    .unwrap();
    let output = std::process::Command::new("uv")
        .current_dir(&python_root)
        .args([
            "run",
            "--no-sync",
            "python",
            "python/lctx_mcp/tests/analytical_enrichment_stdio.py",
        ])
        .arg(&spec)
        .env("PYTHONPATH", python_paths)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "actual enrichment stdio: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
    drop(execution);
    fixture.finish().await;
}
#[tokio::test]
async fn mandatory_packet_preserves_defaults_formals_contexts_and_set_hydration() {
    let fixture = ServingFixture::start(SOURCE).await;
    let execution = fixture.service.execution().await.unwrap();
    let r = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.api"),
        comparison: Optional::default(),
        reference_parameter: Optional::default(),
        sections: vec![],
        page: PageRequest::default(),
    };
    let response = fixture.catalog.operation(&execution, &r).await.unwrap();
    assert_eq!(response.generation.bytes(), fixture.generation.bytes());
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("api did not resolve uniquely")
    };
    assert_eq!(packet.core.release.distribution.as_str(), "demo");
    assert_eq!(packet.core.release.version.as_str(), "1.0");
    assert!(!packet.core.signatures.is_empty());
    let signature = packet
        .core
        .signatures
        .iter()
        .find(|s| s.role == lctx_model::domain::calls::SignatureRole::Source)
        .expect("actual declared source signature");
    assert!(signature.complete);
    assert_eq!(signature.parameters.len(), 1);
    let source_parameter = &signature.parameters[0];
    let DefaultValue::Literal { literal } = source_parameter.default else {
        panic!(
            "declared False default was erased: {:?}",
            source_parameter.default
        )
    };
    assert!(
        packet.core.literal_values.iter().any(|row| {
            row.literal == literal && row.value == LiteralValue::Bool { value: false }
        })
    );
    assert_eq!(signature.effective_parameters.len(), 1);
    let parameter = &signature.effective_parameters[0];
    assert_eq!(parameter.name.0.as_ref().unwrap().as_str(), "flag");
    assert_eq!(parameter.kind, ParameterKind::PositionalOrKeyword);
    assert!(!parameter.formals.is_empty());
    let option_ids = packet
        .core
        .options
        .iter()
        .map(|o| o.option)
        .collect::<Vec<_>>();
    let slot_ids = parameter.slot.0.into_iter().collect::<Vec<_>>();
    let (options, subjects, defaults, slots) = execution
        .query(move |lease| {
            Box::pin(async move {
                let options = lease.read_ids::<CatalogOption>(&option_ids).await?;
                let subjects = lease
                    .read_ids::<CatalogOptionSubject>(
                        &options.rows().iter().map(|o| o.subject).collect::<Vec<_>>(),
                    )
                    .await?;
                let defaults = lease
                    .read_ids::<CatalogDefault>(
                        &options.rows().iter().map(|o| o.default).collect::<Vec<_>>(),
                    )
                    .await?;
                let slots = lease
                    .read_ids::<lctx_model::domain::normalized::callables::SignatureSlot>(&slot_ids)
                    .await?;
                Ok((options, subjects, defaults, slots))
            })
        })
        .await
        .unwrap();
    let expected=options.rows().iter().filter(|o|subjects.rows().iter().any(|s|s.id()==o.subject&&matches!(s,CatalogOptionSubject::Parameter{slot}if Some(*slot)==parameter.slot.0))).map(|o|DefaultValue::from_canonical(defaults.rows().iter().find(|d|d.id()==o.default).unwrap())).collect::<Vec<_>>();
    if !expected.is_empty() {
        assert!(
            expected.iter().all(|d| d == &parameter.default),
            "packet default differs from canonical catalog: {:?}",
            parameter.default
        );
    } else {
        assert_eq!(slots.rows().len(), 1);
        match slots.rows()[0].default {
            lctx_model::domain::normalized::callables::DefaultSlot::Required
            | lctx_model::domain::normalized::callables::DefaultSlot::Collector => {
                assert_eq!(parameter.default, DefaultValue::Absent {})
            }
            lctx_model::domain::normalized::callables::DefaultSlot::DefinitionTime
            | lctx_model::domain::normalized::callables::DefaultSlot::NativeUnknown => {
                assert_eq!(parameter.default, DefaultValue::Unknown {})
            }
        }
    }
    for option in &packet.core.options {
        let stored = options
            .rows()
            .iter()
            .find(|o| o.id() == option.option)
            .unwrap();
        let default = defaults
            .rows()
            .iter()
            .find(|d| d.id() == stored.default)
            .unwrap();
        assert_eq!(option.default, DefaultValue::from_canonical(default));
        if let CatalogDefault::Literal { literal } = default {
            assert!(
                packet
                    .core
                    .literal_values
                    .iter()
                    .any(|l| l.literal == *literal)
            );
        }
    }
    assert!(
        packet
            .core
            .invocations
            .iter()
            .any(|i| i.analysis == signature.analysis)
    );
    assert!(matches!(
        packet.scenarios.availability,
        Availability::NotRequested {}
    ));
    let members = fixture.members().await;
    let ids = members
        .iter()
        .filter(|m| ["demo.api", "demo.consume"].contains(&m.name.as_str()))
        .map(|m| m.member)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    let cores = fixture
        .catalog
        .operation_cores(&execution, &ids, false)
        .await
        .unwrap();
    assert_eq!(cores.iter().map(|c| c.member).collect::<Vec<_>>(), ids);
    let mut missing = r.clone();
    missing.operation = path("demo.absent");
    assert!(matches!(
        fixture
            .catalog
            .operation(&execution, &missing)
            .await
            .unwrap()
            .operation,
        OperationResolution::Missing { .. }
    ));
    let foreign = serde_json::from_value::<lctx_model::domain::Id<CatalogMember>>(
        serde_json::json!(vec![255u8; 16]),
    )
    .unwrap();
    assert!(
        fixture
            .catalog
            .operation_cores(&execution, &[foreign], false)
            .await
            .is_err()
    );
    drop(execution);
    fixture.finish().await;
}
#[tokio::test]
async fn complete_signature_over_default_budget_refuses_and_expanded_packet_is_complete() {
    // Required native roles and premise dictionaries add bytes; keep the complete packet
    // between the unchanged 32 KiB and 256 KiB response bounds.
    let parameters = (0..20)
        .map(|i| format!("p{i}_{}: int", "x".repeat(350)))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("__all__ = ['huge']\ndef huge({parameters}):\n    return 0\n");
    let fixture = ServingFixture::start(source.as_bytes()).await;
    let execution = fixture.service.execution().await.unwrap();
    let mut request = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.huge"),
        comparison: Optional::default(),
        reference_parameter: Optional::default(),
        sections: vec![],
        page: PageRequest::default(),
    };
    assert!(matches!(
        fixture.catalog.operation(&execution, &request).await,
        Err(Error::ResourceRefused(_))
    ));
    request.page.expanded = true;
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("huge did not resolve uniquely")
    };
    let signature = packet
        .core
        .signatures
        .iter()
        .find(|s| s.role == lctx_model::domain::calls::SignatureRole::Source)
        .unwrap();
    assert_eq!(signature.parameters.len(), 20);
    assert_eq!(signature.effective_parameters.len(), 20);
    assert!(packet.core.limits.signature_indivisible);
    let bytes = serde_json::to_vec(&packet).unwrap().len() as u64;
    let limits = ResourceLimits::default();
    assert!(
        bytes > limits.default_response_bytes && bytes <= limits.expanded_response_bytes,
        "whole packet must lie between the unchanged byte bounds: {bytes}"
    );
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn optional_sections_use_original_scenarios_and_policy_admissions_with_bound_continuations() {
    use lctx_model::domain::{
        normalized::events::{CallPolicy, CallPolicyAdmission, CallPolicyAssessment},
        *,
    };
    let targets = (0..7)
        .map(|i| format!("def t{i}(value: int) -> int:\n    return value\n"))
        .collect::<Vec<_>>()
        .join("\n");
    let callers = (0..4)
        .map(|i| format!("def caller{i}():\n    return api(True)\n"))
        .collect::<Vec<_>>()
        .join("\n");
    let names = std::iter::once("'api'".to_owned())
        .chain(std::iter::once("'relay'".to_owned()))
        .chain((0..7).map(|i| format!("'t{i}'")))
        .chain((0..4).map(|i| format!("'caller{i}'")))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "__all__ = [{names}]\ndef api(flag: bool = False) -> bool:\n    return flag\n{targets}\n{callers}\ndef relay() -> int:\n    return {}\n",
        (0..7)
            .map(|i| format!("t{i}({i})"))
            .collect::<Vec<_>>()
            .join(" + ")
    );
    let fixture = ServingFixture::start(source.as_bytes()).await;
    let execution = fixture.service.execution().await.unwrap();
    let sections = vec![
        OperationSection::Scenarios,
        OperationSection::Deployment,
        OperationSection::Relationships,
        OperationSection::Conflicts,
        OperationSection::Briefs,
        OperationSection::Behavior,
    ];
    let mut request = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.relay"),
        comparison: Optional::default(),
        reference_parameter: Optional::default(),
        sections: sections.clone(),
        page: PageRequest {
            size: 100,
            ..Default::default()
        },
    };
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("relay did not resolve")
    };
    for deployment in &packet.deployment.items {
        let id = deployment.deployment;
        let row = execution
            .query(move |lease| {
                Box::pin(async move {
                    let catalog = lease
                        .read_ids::<lctx_model::domain::catalog::evidence::CatalogDeployment>(&[id])
                        .await?;
                    lease
                        .read_ids::<lctx_model::domain::deployment::DeploymentObservation>(&[
                            catalog.rows()[0].observation,
                        ])
                        .await
                })
            })
            .await
            .unwrap();
        assert_eq!(deployment.field.as_str(), row.rows()[0].field);
        assert_eq!(deployment.value.as_str(), row.rows()[0].original);
        assert!(!deployment.originals.is_empty());
    }
    assert_eq!(packet.relationships.items.len(), 5);
    assert!(packet.relationships.truncated);
    assert_eq!(packet.relationships.omitted, 2);
    let token = packet.relationships.continuation.0.clone().unwrap();
    for relationship in &packet.relationships.items {
        let RelationshipPacket::Invocation {
            role,
            proof,
            witnesses,
            ..
        } = relationship
        else {
            panic!("expected invocation relationship")
        };
        assert_eq!(*role, lctx_model::domain::selection::RelationRole::Invokes);
        assert!(!proof.is_empty());
        assert!(witnesses.is_empty());
        let reference = proof
            .iter()
            .find(|p| p.relation.as_str() == CallPolicyAdmission::NAME)
            .unwrap();
        let admission_id: Id<CallPolicyAdmission> =
            serde_json::from_value(serde_json::json!(reference.row)).unwrap();
        let (admission, policy) = execution
            .query(move |lease| {
                Box::pin(async move {
                    let admission = lease
                        .read_ids::<CallPolicyAdmission>(&[admission_id])
                        .await?;
                    let policy = lease
                        .read_ids::<CallPolicyAssessment>(&[admission.rows()[0].assessment])
                        .await?;
                    Ok((admission, policy))
                })
            })
            .await
            .unwrap();
        assert_eq!(admission.rows().len(), 1);
        assert_eq!(policy.rows()[0].policy, CallPolicy::Invocation);
    }
    request.page.cursor = Optional(Some(token.clone()));
    request.page.size = 1;
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet: next } = response.operation else {
        panic!("relay missing")
    };
    assert_eq!(next.relationships.items.len(), 1);
    assert_eq!(next.relationships.omitted, 1);
    assert!(next.relationships.continuation.0.is_some());
    let mut foreign = request.clone();
    foreign.operation = path("demo.api");
    assert!(
        fixture
            .catalog
            .operation(&execution, &foreign)
            .await
            .is_err()
    );
    foreign = request.clone();
    foreign.sections = vec![OperationSection::Scenarios];
    assert!(
        fixture
            .catalog
            .operation(&execution, &foreign)
            .await
            .is_err()
    );
    let scenarios = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.api"),
        comparison: Optional::default(),
        reference_parameter: Optional::default(),
        sections: vec![OperationSection::Scenarios],
        page: PageRequest {
            size: 100,
            ..Default::default()
        },
    };
    let response = fixture
        .catalog
        .operation(&execution, &scenarios)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("api missing")
    };
    assert_eq!(packet.scenarios.items.len(), 2);
    assert!(packet.scenarios.truncated);
    assert!(packet.scenarios.omitted >= 2);
    assert!(matches!(
        packet.relationships.availability,
        Availability::NotRequested {}
    ));
    for scenario in packet.scenarios.items {
        assert_eq!(
            scenario.basis,
            lctx_model::domain::catalog::evidence::AssociationBasis::ResolvedTarget
        );
        assert!(matches!(
            scenario.intent,
            lctx_model::domain::catalog::evidence::Intent::Demonstration
                | lctx_model::domain::catalog::evidence::Intent::AssertionTest
        ));
        assert!(!scenario.spans.is_empty());
        for original in scenario.spans {
            let replay = fixture
                .service
                .original_range(&execution, original.source.clone(), Some(original.context))
                .await
                .unwrap();
            assert_eq!(replay, original);
        }
    }
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn behavioral_packet_preserves_the_stored_five_verdict_condition_and_model_question() {
    use lctx_model::domain::{
        execution::summary_consequences::ClaimConclusion, synthesis::summary::SummaryFacet, *,
    };
    let fixture = ServingFixture::start_profile(SOURCE, "behavioral").await;
    let execution = fixture.service.execution().await.unwrap();
    let request = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.api"),
        comparison: Optional::default(),
        reference_parameter: Optional::default(),
        sections: vec![OperationSection::Behavior],
        page: PageRequest {
            size: 100,
            ..Default::default()
        },
    };
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("api missing")
    };
    for behavior in &packet.behavior.items {
        let facet_reference = behavior
            .proof
            .iter()
            .find(|r| r.relation.as_str() == SummaryFacet::NAME)
            .unwrap();
        let facet_id: Id<SummaryFacet> =
            serde_json::from_value(serde_json::json!(facet_reference.row)).unwrap();
        let (facet, conclusion, qualification, parameters) = execution
            .query(move |lease| {
                Box::pin(async move {
                    let facet = lease.read_ids::<SummaryFacet>(&[facet_id]).await?;
                    let conclusion = lease
                        .read_ids::<ClaimConclusion>(&[facet.rows()[0].conclusion])
                        .await?;
                    let qualification = lease
                        .read_ids::<assertion::AssertionQualification>(&[conclusion.rows()[0]
                            .qualification
                            .unwrap()])
                        .await?;
                    let invocation = lease
                        .read_ids::<analysis::summary::AnalysisInvocation>(&[
                            conclusion.rows()[0].invocation
                        ])
                        .await?;
                    let definition = lease
                        .read_ids::<analysis::AnalysisDefinition>(
                            &[invocation.rows()[0].definition],
                        )
                        .await?;
                    let parameters = lease
                        .read_ids::<analysis::MethodParameters>(&[definition.rows()[0].parameters])
                        .await?;
                    Ok((facet, conclusion, qualification, parameters))
                })
            })
            .await
            .unwrap();
        assert_eq!(behavior.verdict, facet.rows()[0].verdict);
        assert_eq!(behavior.verdict, conclusion.rows()[0].verdict);
        assert_eq!(behavior.condition, qualification.rows()[0].condition);
        assert_eq!(Some(behavior.model), parameters.rows()[0].model_catalog);
        assert_eq!(
            behavior.presentation_truncated,
            behavior.presentation.truncated
        );
        assert!(behavior.presentation.terms.len() <= 16);
    }
    // Qualification absence stays a visible boundary rather than acquiring a made-up condition.
    let member = packet.core.member;
    let facets = execution
        .query(move |lease| {
            Box::pin(async move {
                let members = lease
                    .read_for::<catalog::CatalogMemberInvocation, catalog::CatalogMember>(
                        "member",
                        &[member],
                    )
                    .await?;
                let ids = members.rows().iter().map(Record::id).collect::<Vec<_>>();
                lease
                    .read_for::<SummaryFacet, catalog::CatalogMemberInvocation>("member", &ids)
                    .await
            })
        })
        .await
        .unwrap();
    let mut canonical = facets.rows().iter().collect::<Vec<_>>();
    canonical.sort_by_key(|f| f.id());
    let qualified = canonical
        .iter()
        .filter(|f| f.qualification.is_some())
        .count();
    assert_eq!(packet.behavior.items.len(), qualified.min(100));
    if facets.rows().iter().any(|f| f.qualification.is_none()) {
        assert!(matches!(
            packet.behavior.availability,
            Availability::Partial { .. }
        ));
        assert!(packet.behavior.omitted > 0);
    }
    assert!(matches!(
        packet.scenarios.availability,
        Availability::NotRequested {}
    ));
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn large_optional_brief_keeps_the_complete_core_and_resumes_with_expanded_budget() {
    let prose = format!(
        "Return the supplied value. {}",
        "Authored context. ".repeat(2200)
    );
    let source = format!(
        "__all__ = ['api']\ndef api(flag: bool = False) -> bool:\n    \"\"\"{prose}\"\"\"\n    return flag\n"
    );
    let fixture =
        ServingFixture::start_with_seeds(source.as_bytes(), "catalog", &["demo.api"], 1).await;
    let execution = fixture.service.execution().await.unwrap();
    let mut request = GetOperationRequest {
        library: Name::new("demo").unwrap(),
        operation: path("demo.api"),
        comparison: Optional::default(),
        reference_parameter: Optional::default(),
        sections: vec![OperationSection::Briefs],
        page: PageRequest::default(),
    };
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("api missing")
    };
    let signature = packet
        .core
        .signatures
        .iter()
        .find(|s| s.role == lctx_model::domain::calls::SignatureRole::Source)
        .unwrap();
    assert_eq!(signature.parameters.len(), 1);
    assert!(signature.complete);
    assert!(packet.briefs.items.is_empty());
    assert!(matches!(
        packet.briefs.availability,
        Availability::Partial { .. }
    ));
    assert!(packet.briefs.truncated);
    assert_eq!(packet.briefs.omitted, 1);
    let token = packet.briefs.continuation.0.unwrap();
    request.page.expanded = true;
    request.page.cursor = Optional(Some(token));
    let response = fixture
        .catalog
        .operation(&execution, &request)
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("api missing")
    };
    assert_eq!(packet.briefs.items.len(), 1);
    let capability = &packet.briefs.items[0];
    assert!(capability.rendered.as_str().contains(&prose));
    assert!(capability.unreviewed);
    assert!(capability.documentation_only);
    assert!(!capability.originals.is_empty());
    let standalone = fixture
        .service
        .capability(
            &execution,
            &GetCapabilityRequest {
                capability: capability.capability,
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(standalone.capability, *capability);
    use lctx_model::domain::{
        analysis::policy::EvidenceStatus, synthesis::assertions::ProgrammaticAssertion,
    };
    assert!(!capability.assertions.is_empty());
    let statuses = capability
        .assertions
        .iter()
        .map(|a| a.status)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(statuses.contains(&EvidenceStatus::Documented));
    assert!(statuses.contains(&EvidenceStatus::StructurallyObserved));
    let ids = capability
        .assertions
        .iter()
        .map(|a| a.assertion)
        .collect::<Vec<_>>();
    let originals = execution
        .query(move |lease| {
            Box::pin(async move { lease.read_ids::<ProgrammaticAssertion>(&ids).await })
        })
        .await
        .unwrap();
    for claim in &capability.assertions {
        let original = originals
            .rows()
            .iter()
            .find(|a| a.id() == claim.assertion)
            .unwrap();
        assert_eq!(claim.status, original.status());
        assert_eq!(claim.kind, original.kind());
        assert_eq!(claim.section, original.section());
        assert_eq!(claim.qualification, original.qualification());
        assert_eq!(claim.text.as_str(), original.text());
        assert!(!claim.supports.is_empty());
        for support in &claim.supports {
            assert_eq!(support.proof.len(), 3);
        }
    }
    let resource = standalone.resource_text().unwrap();
    assert!(resource.starts_with(capability.rendered.as_str()));
    for claim in &capability.assertions {
        assert!(resource.contains(&format!("\"status_name\":\"{:?}\"", claim.status)));
        assert!(resource.contains(&serde_json::to_string(&claim.assertion).unwrap()));
    }
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn typed_packets_keep_wrapper_roles_generated_slots_and_native_proofs() {
    use lctx_model::domain::{assumptions::AssumptionSet, calls::SignatureRole};
    let source = br#"from typing import Callable
from dataclasses import dataclass
__all__=['changed','Settings']
def wrap(fn: Callable[[int],int]):
    def implementation(text: str) -> bytes:
        return text.encode()
    return implementation
@wrap
def changed(value: int) -> int:
    return value
@dataclass
class Settings:
    host: str
    port: int = 80
"#;
    let fixture = ServingFixture::start(source).await;
    let execution = fixture.service.execution().await.unwrap();
    let response = fixture
        .catalog
        .operation(
            &execution,
            &GetOperationRequest {
                library: Name::new("demo").unwrap(),
                operation: path("demo.changed"),
                comparison: Optional::default(),
                reference_parameter: Optional::default(),
                sections: vec![],
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("wrapped operation missing");
    };
    let declared = packet
        .core
        .signatures
        .iter()
        .find(|s| s.role == SignatureRole::Source)
        .unwrap();
    let effective = packet
        .core
        .signatures
        .iter()
        .find(|s| s.role == SignatureRole::EffectiveTyped)
        .unwrap();
    assert!(
        declared.parameters.iter().any(|p| p
            .name
            .0
            .as_ref()
            .is_some_and(|n| n.as_str() == "value"))
    );
    assert!(
        effective.effective_parameters.iter().any(|p| p
            .name
            .0
            .as_ref()
            .is_some_and(|n| n.as_str() == "text")
            && !p.types.is_empty()
            && p.formals.is_empty()),
        "native text slot retains typing without invented source formal"
    );
    assert_ne!(
        declared.return_types, effective.return_types,
        "declared int is not native effective bytes"
    );
    assert!(declared.native.0.is_none());
    assert!(effective.native.0.is_some());
    assert!(effective.complete);
    for signature in &packet.core.signatures {
        for answer in &signature.typing {
            assert_eq!(answer.claim_basis.set, AssumptionSet::empty().id());
            assert!(
                answer
                    .proof
                    .iter()
                    .any(|p| p.relation.as_str().ends_with("supports"))
            );
            match answer.origin {
                SignatureTypingOrigin::SourceDeclared { .. } => {
                    assert_eq!(signature.role, SignatureRole::Source)
                }
                SignatureTypingOrigin::NativeObserved { .. } => {
                    assert_ne!(signature.role, SignatureRole::Source)
                }
            }
        }
        assert!(
            signature
                .parameters
                .iter()
                .chain(signature.effective_parameters.iter())
                .flat_map(|p| p.type_evidence.iter())
                .chain(signature.return_evidence.iter())
                .all(|proof| signature
                    .typing
                    .iter()
                    .any(|answer| answer.proof.contains(proof))),
            "every exposed type points to its actual qualified proof"
        );
    }
    // Native/source facet witnesses returned by discovery must exist in this generation.
    for value in [
        lctx_model::domain::selection::FacetValue::Async {
            asynchronous: false,
        },
        lctx_model::domain::selection::FacetValue::DecoratorQualifiedName {
            module: "demo".into(),
            path: vec!["wrap".into()],
        },
    ] {
        use lctx_model::domain::selection::*;
        let facet = if matches!(value, FacetValue::Async { .. }) {
            Facet::Async
        } else {
            Facet::Decorator
        };
        let response = fixture
            .catalog
            .find(
                &execution,
                &FindOperationsRequest {
                    library: Name::new("demo").unwrap(),
                    selection: SelectionInput(Selection {
                        requirements: vec![Requirement {
                            predicate: Predicate::FacetMembership { facet, value },
                            quantifier: Quantifier::AnyApplicable,
                        }],
                        mode: Mode::Discovery,
                        joint: JointPolicy::IndependentRecords,
                    }),
                    page: PageRequest {
                        size: 100,
                        ..Default::default()
                    },
                },
            )
            .await
            .unwrap();
        let result = response
            .supported
            .items
            .iter()
            .find(|r| r.name.as_str() == "demo.changed")
            .expect("actual source facet supports changed");
        let requirement = &result.requirements[0];
        assert!(!requirement.positive.is_empty());
        for id in requirement
            .positive
            .iter()
            .chain(&requirement.negative)
            .chain(&requirement.closure)
        {
            let present: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT EXISTS(SELECT 1 FROM {}.selection_witnesses WHERE id=$1)",
                fixture.generation.schema()
            )))
            .bind(id.bytes().to_vec())
            .fetch_one(fixture.db.owner.pool())
            .await
            .unwrap();
            assert!(present, "request-time witness has no persisted row: {id:?}");
        }
    }
    // Independently specified source/effective return questions retain distinct answers through service preparation.
    {
        use lctx_model::domain::selection::*;
        let returns = |role, name: &str| Predicate::VariantReturnType {
            role,
            r#type: StructuralType::NominalIdentity {
                module: "builtins".into(),
                name: name.into(),
            },
        };
        for (predicate, quantifier, expected) in [
            (
                returns(SignatureRole::Source, "int"),
                Quantifier::AnyApplicable,
                Outcome::Supported,
            ),
            (
                returns(SignatureRole::Source, "bytes"),
                Quantifier::AllApplicable,
                Outcome::Contradicted,
            ),
            (
                returns(SignatureRole::EffectiveTyped, "bytes"),
                Quantifier::AnyApplicable,
                Outcome::Supported,
            ),
            (
                returns(SignatureRole::EffectiveTyped, "int"),
                Quantifier::AllApplicable,
                Outcome::Contradicted,
            ),
            (
                Predicate::FacetMembership {
                    facet: Facet::Async,
                    value: FacetValue::Async { asynchronous: true },
                },
                Quantifier::AllApplicable,
                Outcome::Contradicted,
            ),
            (
                Predicate::FacetMembership {
                    facet: Facet::ClassMetadata,
                    value: FacetValue::ClassMetadata {
                        trait_kind: ClassFacet::Enumeration,
                        present: true,
                    },
                },
                Quantifier::AnyApplicable,
                Outcome::Unresolved,
            ),
            // Negative evidence cannot close an existential question under Partial provider coverage.
            (
                returns(SignatureRole::Source, "bytes"),
                Quantifier::AnyApplicable,
                Outcome::Unresolved,
            ),
        ] {
            let result = fixture
                .catalog
                .compare(
                    &execution,
                    &CompareOperationsRequest {
                        library: Name::new("demo").unwrap(),
                        operations: vec![path("demo.changed")],
                        selection: SelectionInput(Selection {
                            requirements: vec![Requirement {
                                predicate: predicate.clone(),
                                quantifier,
                            }],
                            mode: Mode::Discovery,
                            joint: JointPolicy::IndependentRecords,
                        }),
                        page: PageRequest {
                            expanded: true,
                            ..Default::default()
                        },
                    },
                )
                .await
                .unwrap();
            assert!(!result.operations[0].candidates.is_empty());
            if expected == Outcome::Contradicted
                || matches!(
                    predicate,
                    Predicate::VariantReturnType {
                        role: SignatureRole::Source,
                        ..
                    }
                ) && expected == Outcome::Unresolved
            {
                assert!(
                    result.operations[0]
                        .candidates
                        .iter()
                        .all(|r| !r.requirements[0].negative.is_empty()),
                    "actual counterexample evidence must survive"
                );
            }
            assert!(
                result.operations[0]
                    .candidates
                    .iter()
                    .all(|r| r.requirements[0].outcome == expected),
                "role/facet service answer differs for {predicate:?}: {:?}",
                result.operations[0].candidates
            );
        }
    }
    let encoded = serde_json::to_value(&packet).unwrap();
    assert!(
        encoded["core"]["signatures"][0].get("source").is_none(),
        "hard wire migration replaces misleading source-only field"
    );
    let response = fixture
        .catalog
        .operation(
            &execution,
            &GetOperationRequest {
                library: Name::new("demo").unwrap(),
                operation: path("demo.Settings"),
                comparison: Optional::default(),
                reference_parameter: Optional::default(),
                sections: vec![],
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let OperationResolution::Unique { packet } = response.operation else {
        panic!("generated constructor missing");
    };
    let generated = packet
        .core
        .signatures
        .iter()
        .find(|s| {
            s.role == SignatureRole::Synthesized
                && s.effective_parameters
                    .iter()
                    .any(|p| p.name.0.as_ref().is_some_and(|n| n.as_str() == "host"))
        })
        .unwrap();
    let host = generated
        .effective_parameters
        .iter()
        .find(|p| p.name.0.as_ref().is_some_and(|n| n.as_str() == "host"))
        .unwrap();
    assert!(
        !host.types.is_empty(),
        "generated host slot retains its native str typing"
    );
    // Native slot entities are actual normalized identities, not source-formal declarations.
    let formals = host.formals.clone();
    let entities = execution
        .query(move |lease| {
            Box::pin(async move {
                lease
                    .read_ids::<lctx_model::domain::normalized::entities::ParameterEntity>(&formals)
                    .await
            })
        })
        .await
        .unwrap();
    assert_eq!(entities.rows().len(), host.formals.len());
    for entity in entities.rows() {
        let lctx_model::domain::normalized::entities::ParameterEntity::NativeSlot {
            signature,
            parameter,
            ..
        } = entity
        else {
            panic!("generated slot invented a source formal: {entity:?}");
        };
        assert_eq!(*signature, generated.signature);
        assert_eq!(*parameter, host.parameter);
    }
    assert!(
        generated
            .typing
            .iter()
            .any(|t| matches!(t.origin, SignatureTypingOrigin::NativeObserved { .. }))
    );
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn original_evidence_explains_complete_native_use_inventory() {
    use lctx_model::domain::{flow::*, flow_inventory::*, source::Occurrence};
    let source = br#"__all__=['choose','echo','conditional']
def choose(flag: bool):
    if flag:
        value = 1
    else:
        value = 2
    return value
def echo(value: int) -> int:
    return value
def conditional(value: int, flag: bool) -> int:
    if flag:
        return value
    return 0
"#;
    let fixture = ServingFixture::start_profile(source, "behavioral").await;
    let execution = fixture.service.execution().await.unwrap();
    let start = std::str::from_utf8(source)
        .unwrap()
        .find("return value")
        .unwrap()
        + "return ".len();
    let occurrence = execution
        .query(move |lease| {
            Box::pin(async move {
                let inventories = lease.read::<FlowUseInventoryObservation>().await?;
                let uses = lease
                    .read_ids::<FlowUse>(
                        &inventories
                            .rows()
                            .iter()
                            .map(|i| i.use_)
                            .collect::<Vec<_>>(),
                    )
                    .await?;
                let occurrences = lease
                    .read_ids::<Occurrence>(
                        &uses.rows().iter().map(|u| u.occurrence).collect::<Vec<_>>(),
                    )
                    .await?;
                Ok(occurrences
                    .rows()
                    .iter()
                    .find(|o| o.start == start as i64)
                    .expect("original return read has a native inventory")
                    .id())
            })
        })
        .await
        .unwrap();
    let response = fixture
        .service
        .evidence(
            &execution,
            &GetEvidenceRequest {
                source: OriginalReference::Occurrence { occurrence },
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let items = &response.evidence.flow_inventory.items;
    assert!(
        !items.is_empty(),
        "actual provider inventory reaches original evidence"
    );
    for inventory in items {
        assert_eq!(inventory.occurrence, occurrence);
        assert_eq!(inventory.start, start as u64);
        assert_eq!(inventory.native_count, inventory.candidates.len() as u64);
        assert_eq!(inventory.mapped_count, inventory.members.len() as u64);
        assert_eq!(
            inventory.entry_value_reason.0,
            Some(lctx_model::domain::obligation::ObligationKind::EntryValueUnknown)
        );
        assert!(inventory.entry_outcomes.items.is_empty());
        assert!(!inventory.members.is_empty() && !inventory.proof.is_empty());
        assert!(!inventory.view.coverage.is_empty());
        assert!(
            inventory
                .members
                .iter()
                .all(|m| matches!(m.target, FlowOriginTarget::Bound { .. }))
        );
    }
    assert_eq!(response.evidence.flow_inventory.omitted, 0);
    assert!(!response.evidence.flow_inventory.truncated);
    let echo_start = std::str::from_utf8(source)
        .unwrap()
        .find("def echo")
        .unwrap()
        + "def echo(value: int) -> int:\n    return ".len();
    let echo = execution
        .query(move |lease| {
            Box::pin(async move {
                let inventories = lease.read::<FlowUseInventoryObservation>().await?;
                let uses = lease
                    .read_ids::<FlowUse>(
                        &inventories
                            .rows()
                            .iter()
                            .map(|i| i.use_)
                            .collect::<Vec<_>>(),
                    )
                    .await?;
                let occurrences = lease
                    .read_ids::<Occurrence>(
                        &uses.rows().iter().map(|u| u.occurrence).collect::<Vec<_>>(),
                    )
                    .await?;
                Ok(occurrences
                    .rows()
                    .iter()
                    .find(|o| o.start == echo_start as i64)
                    .expect("echo return inventory")
                    .id())
            })
        })
        .await
        .unwrap();
    let echo = fixture
        .service
        .evidence(
            &execution,
            &GetEvidenceRequest {
                source: OriginalReference::Occurrence { occurrence: echo },
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert!(
        echo.evidence
            .flow_inventory
            .items
            .iter()
            .any(|i| !i.entry_outcomes.items.is_empty() && i.entry_value_reason.0.is_none()),
        "admitted singleton parameter outcome survives alongside enumeration"
    );
    assert!(
        echo.evidence
            .flow_inventory
            .items
            .iter()
            .flat_map(|i| &i.entry_outcomes.items)
            .all(|o| !o.premises.is_empty() && !o.proof.is_empty())
    );
    let conditional_start = std::str::from_utf8(source)
        .unwrap()
        .rfind("return value")
        .unwrap()
        + "return ".len();
    let conditional = execution
        .query(move |lease| {
            Box::pin(async move {
                let inventories = lease.read::<FlowUseInventoryObservation>().await?;
                let uses = lease
                    .read_ids::<FlowUse>(
                        &inventories
                            .rows()
                            .iter()
                            .map(|i| i.use_)
                            .collect::<Vec<_>>(),
                    )
                    .await?;
                let occurrences = lease
                    .read_ids::<Occurrence>(
                        &uses.rows().iter().map(|u| u.occurrence).collect::<Vec<_>>(),
                    )
                    .await?;
                let occurrence = occurrences
                    .rows()
                    .iter()
                    .find(|o| o.start == conditional_start as i64)
                    .expect("conditional native return inventory")
                    .id();
                let use_ = uses
                    .rows()
                    .iter()
                    .find(|u| u.occurrence == occurrence)
                    .unwrap();
                assert!(
                    inventories.rows().iter().any(|i| i.use_ == use_.id()
                        && i.complete
                        && i.native_count == 1
                        && i.mapped_count == 1),
                    "native conditional access has a complete singleton inventory"
                );
                Ok(occurrence)
            })
        })
        .await
        .unwrap();
    let conditional = fixture
        .service
        .evidence(
            &execution,
            &GetEvidenceRequest {
                source: OriginalReference::Occurrence {
                    occurrence: conditional,
                },
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let contributions = conditional
        .evidence
        .flow_inventory
        .items
        .iter()
        .flat_map(|i| &i.entry_outcomes.items)
        .flat_map(|o| &o.contributions)
        .collect::<Vec<_>>();
    assert!(
        !contributions.is_empty(),
        "actual conditional singleton has a stored Local proof"
    );
    assert!(
        contributions
            .iter()
            .all(|c| c.condition != lctx_model::domain::conditions::Diagram::always().id()),
        "serving cannot replace a native conditional Local proof by an unconditional claim"
    );
    assert!(
        conditional
            .evidence
            .flow_inventory
            .items
            .iter()
            .flat_map(|i| &i.entry_outcomes.items)
            .all(|o| !o.proof.is_empty() && !o.premises.is_empty())
    );
    drop(execution);
    fixture.finish().await;
}

/// Native PostgreSQL explanation and wire parity; no MCP proof-request interface is implied.
#[tokio::test]
async fn composed_summary_explanation_retains_exact_selected_signature_domain() {
    use lctx_model::domain::{
        calls::{
            Signature, SignatureEnumerationMember, SignatureEnumerationObservation,
            SignatureEnumerationSupport, SignatureRole,
        },
        derivation::RowRef,
        transfer::summary::SummaryWitness,
    };
    let source = br#"__all__=['identity','relay']
def identity(value):
    return value
def relay(value):
    return identity(value)
@unknown_decorator
def unavailable():
    return 1
"#;
    let fixture = ServingFixture::start_with_seeds(source, "behavioral", &["demo.relay"], 4).await;
    let execution = fixture.service.execution().await.unwrap();
    let witnesses = execution.read::<SummaryWitness>().await.unwrap();
    let witness = witnesses
        .rows()
        .iter()
        .find(|row| row.selected_signature_enumeration.is_some())
        .expect("actual composed source witness with selected domain");
    let enumeration = witness.selected_signature_enumeration.unwrap();
    let native_support = witness
        .selected_signature_enumeration_support
        .expect("selected domain retains its exact native support");
    let (header, support, members) = execution
        .query(move |lease| {
            Box::pin(async move {
                let headers = lease
                    .read_ids::<SignatureEnumerationObservation>(&[enumeration])
                    .await?;
                let supports = lease
                    .read_ids::<SignatureEnumerationSupport>(&[native_support])
                    .await?;
                let members = lease
                    .read_for::<SignatureEnumerationMember, SignatureEnumerationObservation>(
                        "enumeration",
                        &[enumeration],
                    )
                    .await?;
                Ok((
                    headers.rows()[0].clone(),
                    supports.rows()[0].clone(),
                    members.rows().to_vec(),
                ))
            })
        })
        .await
        .unwrap();
    assert!(header.complete);
    assert_eq!(header.role, SignatureRole::Source);
    assert_eq!(support.assertion, enumeration);
    assert!(!members.is_empty());
    let root = ProofReference::from_canonical(RowRef::of(witness.id()));
    let proof = fixture
        .service
        .explanation(&execution, root.clone())
        .await
        .unwrap();
    let step = proof
        .items
        .iter()
        .find(|step| step.source == root && step.rule.as_str() == "compose_through_call")
        .expect("canonical composed witness explanation");
    for (role, reference) in [
        (
            "selected_signature_enumeration",
            ProofReference::from_canonical(RowRef::of(enumeration)),
        ),
        (
            "selected_signature_enumeration_support",
            ProofReference::from_canonical(RowRef::of(native_support)),
        ),
    ] {
        assert!(
            step.premises
                .iter()
                .any(|premise| premise.role.as_str() == role && premise.premise == reference)
        );
    }
    let mut members = members;
    members.sort_by_key(|member| member.ordinal);
    let enumeration_ref = ProofReference::from_canonical(RowRef::of(enumeration));
    let membership_steps = proof
        .items
        .iter()
        .filter(|step| {
            step.rule.as_str() == "signature_enumeration_member"
                && step.conclusion == enumeration_ref
        })
        .collect::<Vec<_>>();
    assert_eq!(membership_steps.len(), members.len());
    for (ordinal, member) in members.iter().enumerate() {
        assert_eq!(member.ordinal, ordinal as i64);
        let reference = ProofReference::from_canonical(RowRef::of(member.id()));
        let step = membership_steps
            .iter()
            .find(|step| step.source == reference)
            .expect("every exact selected enumeration member is visible");
        assert_eq!(
            step.premises,
            vec![PremisePacket {
                role: Name::new("signature").unwrap(),
                premise: ProofReference::from_canonical(RowRef::of(member.signature)),
            }]
        );
    }
    let member_count = members.len();
    let signatures = execution
        .query(move |lease| {
            Box::pin(async move {
                lease
                    .read_ids::<Signature>(
                        &members
                            .iter()
                            .map(|member| member.signature)
                            .collect::<Vec<_>>(),
                    )
                    .await
            })
        })
        .await
        .unwrap();
    assert_eq!(signatures.rows().len(), member_count);
    assert!(
        signatures
            .rows()
            .iter()
            .all(|signature| signature.role == header.role
                && signature.symbol == header.symbol
                && signature.qualification == header.qualification)
    );
    let serialized = serde_json::to_value(&proof).unwrap();
    let restored: SectionPage<DerivationStep> = serde_json::from_value(serialized).unwrap();
    assert_eq!(restored, proof);
    drop(execution);
    fixture.finish().await;
}
