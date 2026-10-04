//! Actual compiler + PG18 + original-evidence packet, with no execution inference.
#[path = "fixtures/serving_support.rs"]
mod support;
use lctx_model::domain::{assertion::*, diagnostics::*, serving::*, source::*, *};
#[tokio::test]
async fn selected_diagnostics_are_cited_inside_the_same_original_source_grant() {
    let source=b"import os\n__all__ = ['api']\nlabel = 'caf\xc3\xa9'\nmissing_name\nsuppressed_name  # noqa: F821\ndef api(value: int) -> int:\n    unused_local = 'kept'\n    return 'wrong'\nimport pytest as ptest\n@ptest.fixture\ndef resource() -> int:\n    return 1\ndef test_resource(resource):\n    pass\n";
    let fixture = source_fixture(source, false).await;
    let execution = fixture.service.execution().await.unwrap();
    let digest = ContentHash::of(source);
    let artifacts = execution.read::<SourceArtifact>().await.unwrap();
    let artifact = artifacts
        .rows()
        .iter()
        .find(|a| a.content == digest)
        .unwrap()
        .id();
    let full = fixture
        .service
        .evidence(
            &execution,
            &GetEvidenceRequest {
                source: OriginalReference::Artifact { artifact },
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(full.evidence.body.bytes, source);
    let items = &full.evidence.source_characterization.items;
    assert!(!items.is_empty());
    assert!(items.iter().any(|i|matches!(&i.payload,SourceCharacterizationPayload::RuffDiagnostic{native_code,channel,message,..} if native_code.as_str()=="F821" && *channel==DiagnosticChannel::RuffNoqaSuppressed && message.as_str().contains("suppressed_name"))));
    assert!(items.iter().any(|i|matches!(&i.payload,SourceCharacterizationPayload::PyreflyDiagnostic{category,channel,..} if category.as_str()=="bad-return"&&*channel==DiagnosticChannel::Emitted)));
    assert!(items.iter().any(|i|matches!(&i.payload,SourceCharacterizationPayload::ParameterDefinition{answer,role,..} if *answer==DefinitionAnswer::Known&&*role==NativeParameterRole::Ordinary)));
    for item in items {
        assert_eq!(item.source.artifact, artifact);
        assert!(
            item.source.start >= full.evidence.original.start
                && item.source.end <= full.evidence.original.end
        );
        assert_eq!(item.support.context, full.evidence.original.context);
        assert!(item.proof.iter().any(|p| p == &item.support.support));
    }
    let spans = execution.read::<Evidence>().await.unwrap();
    let primary=items.iter().find_map(|i|matches!(&i.payload,SourceCharacterizationPayload::RuffDiagnostic{native_code,channel,..} if native_code.as_str()=="F821"&&*channel==DiagnosticChannel::Emitted).then_some(&i.source)).unwrap();
    let span=spans.rows().iter().find(|e|matches!(e,Evidence::SourceSpan{source,start,end} if *source==artifact&&*start==primary.start as i64&&*end==primary.end as i64)).unwrap();
    let id = EvidenceSourceSpanId::of(span).unwrap();
    let bounded = fixture
        .service
        .evidence(
            &execution,
            &GetEvidenceRequest {
                source: OriginalReference::Span { span: id },
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(
        bounded.evidence.body.bytes,
        &source[primary.start as usize..primary.end as usize]
    );
    assert!(
        bounded
            .evidence
            .source_characterization
            .items
            .iter()
            .all(|i| i.source.artifact == artifact
                && i.source.start >= primary.start
                && i.source.end <= primary.end)
    );
    assert!(
        !bounded
            .evidence
            .source_characterization
            .items
            .iter()
            .any(|i| matches!(
                i.payload,
                SourceCharacterizationPayload::ParameterDefinition { .. }
            ))
    );
    let fixture_answer=items.iter().find(|i|matches!(&i.payload,SourceCharacterizationPayload::ParameterDefinition{answer,role,target_location,..} if *answer==DefinitionAnswer::Known&&*role==NativeParameterRole::Fixture&&matches!(target_location,Availability::Available{}))).expect("actual fixture answer retains its native target");
    let occurrences = execution.read::<Occurrence>().await.unwrap();
    let parameter = occurrences
        .rows()
        .iter()
        .find(|o| {
            o.source == artifact
                && o.start == fixture_answer.source.start as i64
                && o.end == fixture_answer.source.end as i64
        })
        .unwrap();
    let parameter_packet = fixture
        .service
        .evidence(
            &execution,
            &GetEvidenceRequest {
                source: OriginalReference::Occurrence {
                    occurrence: parameter.id(),
                },
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(
        parameter_packet.evidence.body.bytes,
        &source[parameter.start as usize..parameter.end as usize]
    );
    assert!(parameter_packet.evidence.source_characterization.items.iter().any(|i|matches!(&i.payload,SourceCharacterizationPayload::ParameterDefinition{answer,role,target_location,target,..} if *answer==DefinitionAnswer::Known&&*role==NativeParameterRole::Fixture&&matches!(target_location,Availability::Unavailable{..})&&target.0.is_none())),"out-of-grant fixture definition remains unavailable without changing the native answer");
    drop(execution);
    fixture.finish().await;
}

// This fixture owns its additional wheel members; the shared serving fixture stays unchanged.
async fn source_fixture(source: &[u8], usage: bool) -> support::ServingFixture {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use lctx_postgres::{
        generations::{CatalogService, GenerationId, GenerationService, GenerationStore},
        testing::DisposableDatabase,
    };
    use sha2::{Digest as _, Sha256};
    use std::sync::Arc;
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    support::input_source(dir.path(), source);
    db.write_configs(dir.path()).unwrap();
    let site = dir.path().join("envs/demo/lib/python3.14/site-packages");
    let record = site.join("demo-1.0.dist-info/RECORD");
    let mut entries = std::fs::read_to_string(&record).unwrap();
    let mut files = vec![(
        "pytest.pyi",
        include_bytes!("../../../fixtures/python/native_diagnostics/pytest.pyi").as_slice(),
    )];
    if usage {
        files.extend([
            (
                "demo/api.py",
                include_bytes!("../../../fixtures/python/native_usage/api.py").as_slice(),
            ),
            (
                "demo/reexport.py",
                b"from .api import parse as public_parse\n__all__ = ['public_parse']\n".as_slice(),
            ),
        ]);
    }
    for (path, bytes) in files {
        support::write(&site.join(path), bytes);
        entries.push_str(&format!(
            "{path},sha256={},{}\n",
            URL_SAFE_NO_PAD.encode(Sha256::digest(bytes)),
            bytes.len()
        ));
    }
    support::write(&record, entries);
    support::write(
        &dir.path().join("libraries/demo/analytics.toml"),
        "version = 1\n[subsystem]\nmodule_prefixes = ['demo']\npublic_roots = ['demo']\n[seeds]\nprimary = []\ndistractors = []\n[pass_a]\nmax_depth = 4\nmax_vertices = 256\nmax_edges = 1024\nmax_witnesses = 4\n[briefs]\nbudget = 0\n",
    );
    let output = support::command(
        dir.path(),
        &dir.path().join("postgres.json"),
        "catalog",
        "catalog",
    )
    .output()
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let generation = GenerationId::from_hex(report["generation"].as_str().unwrap()).unwrap();
    let cfg =
        lctx_postgres::roles::RoleConfig::load(&dir.path().join("postgres-serving.json")).unwrap();
    let service = GenerationService::admit(model, &cfg, Some(generation))
        .await
        .unwrap();
    let catalog = CatalogService::prepare(service.clone()).await.unwrap();
    support::ServingFixture {
        db,
        store,
        service,
        catalog,
        generation,
        dir,
    }
}
#[tokio::test]
async fn actual_usage_aliases_targets_overloads_and_locations_reach_original_evidence() {
    use lctx_model::domain::{calls::*, types::TypeRole};
    let source =
        br#"from .api import parse as parse, replacement as replacement, Box as Box, apply as apply
from .reexport import public_parse as forwarded
from .api import replacement as simple
from .api import parse as aliased
__all__ = ['parse', 'forwarded', 'replacement', 'Box', 'apply', 'example']
opaque = parse
def example(unknown):
    apply(replacement, 7)
    simple(6)
    aliased(1)
    opaque(2)
    forwarded('text')
    rebound = aliased
    rebound(2)
    rebound = replacement
    rebound(3)
    aliased(object())
    box = Box()
    box.run(4)
    unknown(5)
"#;
    let fixture = source_fixture(source, true).await;
    let execution = fixture.service.execution().await.unwrap();
    let artifacts = execution.read::<SourceArtifact>().await.unwrap();
    let artifact = artifacts
        .rows()
        .iter()
        .find(|a| a.content == ContentHash::of(source))
        .unwrap()
        .id();
    let packet = fixture
        .service
        .evidence(
            &execution,
            &GetEvidenceRequest {
                source: OriginalReference::Artifact { artifact },
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(packet.evidence.body.bytes, source);
    let usages = packet
        .evidence
        .source_characterization
        .items
        .iter()
        .filter_map(|i| match &i.payload {
            SourceCharacterizationPayload::Usage { usage } => Some((i, usage)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!usages.is_empty());
    let call = |text: &str| {
        usages
            .iter()
            .find(|(i, _)| {
                &source[i.source.start as usize..i.source.end as usize] == text.as_bytes()
            })
            .copied()
            .unwrap_or_else(|| panic!("missing usage {text}"))
    };
    let symbols = execution.read::<ProviderSymbol>().await.unwrap();
    let names = |u: &SourceUsagePacket| {
        u.targets
            .iter()
            .filter_map(|t| t.symbol.0)
            .map(|id| {
                symbols
                    .rows()
                    .iter()
                    .find(|s| s.id() == id)
                    .unwrap()
                    .name
                    .as_str()
            })
            .collect::<Vec<_>>()
    };
    for text in ["aliased(1)", "forwarded('text')"] {
        let (_, u) = call(text);
        assert!(
            names(u).contains(&"parse"),
            "{text}: names {:?}; targets {:?}",
            names(u),
            u.targets
        );
        assert!(
            matches!(u.association, Availability::Available {}),
            "actual resolved target has a catalog association: {text}; targets {:?}",
            u.targets
        );
        assert!(u.targets.iter().all(|t| !t.support.is_empty()));
    }
    let (_, simple) = call("simple(6)");
    assert!(names(simple).contains(&"replacement"));
    assert!(matches!(simple.association, Availability::Available {}));
    let (_, module_alias) = call("opaque(2)");
    assert!(matches!(
        module_alias.association,
        Availability::Unavailable { .. }
    ));
    assert!(
        module_alias.targets.iter().any(|t| t.native_unresolved.0
            == Some(PysaUnresolvedReason::UnexpectedDefiningClass)
            && t.entity.0.is_none()
            && t.symbol.0.is_none()),
        "native module assignment alias remains unresolved: {:?}",
        module_alias.targets
    );
    let (_, opaque_alias) = call("rebound(2)");
    assert!(matches!(
        opaque_alias.association,
        Availability::Unavailable { .. }
    ));
    assert!(
        opaque_alias.targets.iter().any(|t| t.native_unresolved.0
            == Some(PysaUnresolvedReason::UnexpectedDefiningClass)
            && t.entity.0.is_none()),
        "native overload alias uncertainty never gains an API association from spelling or type shape"
    );
    let (_, rebound) = call("rebound(3)");
    assert!(names(rebound).contains(&"replacement"));
    assert!(!names(rebound).contains(&"parse"));
    let (_, chosen) = call("aliased(1)");
    assert!(matches!(chosen.chosen, Availability::Available {}));
    assert!(
        chosen
            .overloads
            .iter()
            .any(|o| o.role == TypeRole::ChosenOverload)
    );
    assert!(
        chosen.overloads.iter().all(|o| o.variant.0.is_none()
            && matches!(o.variant_availability, Availability::Unavailable { .. })),
        "native trace is not shape-matched to a normalized variant"
    );
    let (_, failed) = call("aliased(object())");
    assert!(matches!(failed.chosen, Availability::Unavailable { .. }));
    assert!(
        failed
            .overloads
            .iter()
            .any(|o| o.role == TypeRole::OverloadCandidates)
    );
    assert!(
        !failed
            .overloads
            .iter()
            .any(|o| o.role == TypeRole::ChosenOverload)
    );
    let (_, unknown) = call("unknown(5)");
    assert!(matches!(
        unknown.association,
        Availability::Unavailable { .. }
    ));
    assert!(
        unknown
            .targets
            .iter()
            .any(|t| t.unresolved.0.is_some() && t.entity.0.is_none())
    );
    let (_, callback) = call("apply(replacement, 7)");
    assert!(callback.targets.iter().any(|t| matches!(
        t.channel_kind,
        UsageChannel::HigherOrder { argument_index: 0 }
    ) && t.modality == attribution::Modality::Potential));
    assert!(
        callback
            .targets
            .iter()
            .all(|t| t.claim_basis.definitions.is_empty()),
        "actual native targets carry the resolved empty claim basis"
    );
    let (_, method) = call("box.run(4)");
    assert!(method.targets.iter().any(|t| {
        matches!(t.receiver_location, Availability::Available {})
            && t.receiver_span
                .0
                .as_ref()
                .is_some_and(|s| &source[s.start as usize..s.end as usize] == b"box")
    }));
    for (i, u) in &usages {
        assert!(i.proof.contains(&i.support.support));
        assert_eq!(i.support.context, packet.evidence.original.context);
        for target in &u.targets {
            assert!(
                target
                    .support
                    .iter()
                    .all(|s| s.context == packet.evidence.original.context)
            );
        }
        for arg in &u.arguments {
            if let Some(span) = &arg.span.0 {
                assert!(
                    span.artifact == artifact
                        && span.start >= i.source.start
                        && span.end <= i.source.end
                );
            }
        }
    }
    // An actual callee-only original grant cannot inherit its enclosing usage or argument bytes.
    let callee = chosen.callee.0.unwrap();
    let narrow = fixture
        .service
        .evidence(
            &execution,
            &GetEvidenceRequest {
                source: OriginalReference::Occurrence { occurrence: callee },
                page: PageRequest {
                    expanded: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(narrow.evidence.body.bytes, b"aliased");
    assert!(
        !narrow
            .evidence
            .source_characterization
            .items
            .iter()
            .any(|i| matches!(i.payload, SourceCharacterizationPayload::Usage { .. }))
    );
    drop(execution);
    fixture.finish().await;
}

#[tokio::test]
async fn diagnostic_primary_argument_has_supported_use_and_route_packets_retain_alias_identity() {
    use lctx_model::domain::catalog::{evidence::DiagnosticUseStatus,access_routes::{RouteHop,RouteStop}};
    let source=br#"from .api import parse as parse
from .reexport import public_parse as forwarded
__all__=['parse','forwarded','example']
def example():
    parse(1.5)
    forwarded('text')
"#;
    let fixture=source_fixture(source,true).await;
    let execution=fixture.service.execution().await.unwrap();
    let artifacts=execution.read::<SourceArtifact>().await.unwrap();
    let artifact=artifacts.rows().iter().find(|a|a.content==ContentHash::of(source)).unwrap().id();
    let response=fixture.service.evidence(&execution,&GetEvidenceRequest {source:OriginalReference::Artifact{artifact},page:PageRequest {expanded:true,..Default::default()}}).await.unwrap();
    let call_start=std::str::from_utf8(source).unwrap().find("parse(1.5)").unwrap() as u64;
    let call_end=call_start+"parse(1.5)".len() as u64;
    let diagnostic=response.evidence.source_characterization.items.iter().find(|item|matches!(&item.payload,SourceCharacterizationPayload::PyreflyDiagnostic{..})&&item.source.start>=call_start&&item.source.end<=call_end).expect("native diagnosis is anchored within the failed call");
    let correlation=diagnostic.diagnostic_correlation.0.as_ref().unwrap();
    assert_eq!(correlation.status,DiagnosticUseStatus::UniqueUse);
    assert!(!correlation.remainder);assert!(!correlation.links.items.is_empty());
    assert!(correlation.links.items.iter().all(|link|!link.proof.is_empty()));
    assert!(correlation.links.items.iter().any(|link|!link.targets.is_empty()),"API relevance cites exact event alternative/scenario association");
    let request=GetOperationRequest {library:Name::new("demo").unwrap(),operation:OperationSelector::PublicPath {path:vec![Name::new("demo").unwrap(),Name::new("forwarded").unwrap()]},comparison:Optional::default(),reference_parameter:Optional::default(),sections:vec![OperationSection::AccessRoutes],page:PageRequest{expanded:true,..Default::default()}};
    let OperationResolution::Unique{packet}=fixture.catalog.operation(&execution,&request).await.unwrap().operation else{panic!("forwarded operation missing")};
    assert!(!packet.access_routes.items.is_empty());
    assert!(packet.access_routes.items.iter().any(|route|route.hops.iter().filter(|hop|matches!(hop,RouteHop::Import{..})).count()==2),"explicit reexport has two ordered supported import-name hops");
    assert!(packet.access_routes.items.iter().any(|route|route.stop==RouteStop::Declaration));
    assert!(packet.access_routes.items.iter().all(|route|route.captured_modules>0));
    drop(execution);fixture.finish().await;
}
