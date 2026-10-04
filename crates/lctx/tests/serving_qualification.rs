//! The operator runner is also exercised against real compiled disposable PG18 data.
#[path = "fixtures/serving_support.rs"]
mod support;
use std::path::Path;
use support::{ServingFixture, write};

const TECHNIQUES: &str =
    "+communities,+pagerank,+fca,+rca,+type-layer,+mention-layer,-knn,-knn-layer";
const DOCUMENT: &[u8] = b"# Official typed APIs\n\n`demo.first` and `demo.second` accept Token; `demo.first` repeats the same entity.\n";
const SEEDS: &[&str] = &["demo.api", "demo.first"];

async fn assert_selected_analytics(fixture: &ServingFixture) {
    use lctx_model::domain::{
        analysis::{self, AnalysisMethod as M, AnalysisStatus as S, Interpretation},
        analytics::*,
        assertion::AssertionQualification,
        documents::{DocumentMentionObservation, MentionClass},
        input::{ArtifactUse, CorpusLibrary, SourceRole},
        normalized::{
            Rows,
            entities::{PublicExposureCandidate, SymbolEntityResolution},
            links::{MentionEntityAssessment, MentionEntityCandidate},
        },
        source::{CoverageScope, Module, SourceArtifact},
        structural::{PublicCandidate, StructuralFrame},
        types::{TypeObservation, TypeRole},
        *,
    };
    let execution = fixture.service.execution().await.unwrap();
    macro_rules! read {($($var:ident:$ty:ty),*$(,)?) => {$(
        let $var = execution.read::<$ty>().await.unwrap();
    )*};}
    read! {
        settings:analysis::settings::AnalyticsConfiguration, definitions:analysis::AnalysisDefinition,
        parameters:analysis::MethodParameters, frames:AnalyticFrame, structural:StructuralFrame,
        results:TechniqueResult, invocations:analysis::analytic::Invocation,
        outcomes:analysis::analytic::AnalysisOutcome, parents:analysis::analytic::AnalysisInput,
        sources:analysis::analytic::InvocationSource, receipts:analysis::analytic::SourceReceipt,
        universe:UniverseMember, selectors:PublicSelector, public:PublicCandidate,
        pairs:LayerPair, combined:CombinedPair, contributions:PairContribution, provenance:PairSource,
        ranks:RankScore, runs:CommunityRun, scopes:ConceptScope, objects:ConceptObject,
        incidences:Incidence, attributes:Attribute, incidence_sources:IncidenceSource,
        formal_syntax:syntax::ParameterSyntaxObservation, types:TypeObservation,
        mentions:DocumentMentionObservation, artifacts:SourceArtifact, uses:ArtifactUse,
        qualifications:AssertionQualification, coverage_scopes:CoverageScope,
        modules:Module, corpus_libraries:CorpusLibrary,
        structural_invocations:analysis::structural::Invocation,
        mention_assessments:MentionEntityAssessment, mention_candidates:MentionEntityCandidate,
        exposure_candidates:PublicExposureCandidate, resolutions:SymbolEntityResolution,
        embeddings:analysis::analytic_embedding::AnalysisOutcome,
        vectors:embedding::analytic::AnalysisEmbeddingUse,
    }
    assert_eq!(settings.rows().len(), 1);
    let configuration = &settings.rows()[0];
    assert_eq!(
        configuration.configured_seeds,
        SEEDS.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    );
    assert_eq!(
        (
            configuration.depth,
            configuration.vertices,
            configuration.arcs,
            configuration.witnesses,
            configuration.brief_budget
        ),
        (4, 256, 1024, 4, 4)
    );
    assert!(
        configuration.communities
            && configuration.pagerank
            && configuration.fca
            && configuration.rca
            && configuration.type_layer
            && configuration.mention_layer
    );
    assert!(!configuration.knn && !configuration.knn_layer);
    assert!(!frames.rows().is_empty());
    assert!(!embeddings.rows().is_empty());
    assert!(
        embeddings
            .rows()
            .iter()
            .all(|e| e.status == S::NotRequested)
    );
    assert!(
        vectors.rows().is_empty(),
        "Q0 never substitutes vectors for lexical evidence"
    );
    let document = artifacts
        .rows()
        .iter()
        .find(|a| a.content == ContentHash::of(DOCUMENT))
        .expect("official Markdown must be captured through the declared corpus");
    assert!(
        uses.rows()
            .iter()
            .any(|u| u.artifact == document.id() && u.role == SourceRole::Document)
    );
    for frame in frames.rows() {
        assert_eq!(frame.configuration, configuration.id());
        let predecessor = structural
            .rows()
            .iter()
            .find(|s| s.id() == frame.structural)
            .unwrap();
        let candidates = public
            .rows()
            .iter()
            .filter(|p| p.frame == frame.structural && p.in_subsystem)
            .collect::<Vec<_>>();
        let entity = |path: &str| {
            candidates
                .iter()
                .find(|p| p.path == path)
                .unwrap_or_else(|| panic!("missing independent fixture endpoint {path}"))
                .entity
        };
        let (first, second, api) = (
            entity("demo.first"),
            entity("demo.second"),
            entity("demo.api"),
        );
        let endpoints = (first.min(second), first.max(second));
        let members = universe
            .rows()
            .iter()
            .filter(|u| u.frame == frame.id())
            .collect::<Vec<_>>();
        assert!(
            members
                .iter()
                .any(|u| u.entity == first && u.public && u.release_scope)
        );
        assert!(
            members
                .iter()
                .any(|u| u.entity == second && u.public && u.release_scope)
        );
        assert_eq!(
            members
                .iter()
                .map(|u| u.entity)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            members.len(),
            "entity universe must not multiply public paths"
        );
        let selected = selectors
            .rows()
            .iter()
            .filter(|s| s.frame == frame.id())
            .map(|s| s.candidate)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            selected,
            candidates.iter().map(|p| p.id()).collect(),
            "configured seeds cannot shrink public selection"
        );
        // Actual Installed and Corpus inputs remain distinct. The exact recorded linkage is
        // the only cross-input admission authority; lexical matches cannot supply that link.
        let parent = structural_invocations
            .rows()
            .iter()
            .find(|i| i.id() == predecessor.invocation)
            .unwrap();
        assert_ne!(parent.input, document.input);
        assert!(
            corpus_libraries
                .rows()
                .iter()
                .any(|link| link.corpus == document.input && link.library == parent.input),
            "official corpus must name its exact installed library input"
        );
        let budget = resources::ResourceBudget::scoped(execution.budget(), 8 << 20).unwrap();
        let mut data = build::Data::new(&budget);
        data.structural.frames.insert(predecessor.clone()).unwrap();
        data.structural_invocations.insert(parent.clone()).unwrap();
        macro_rules! retain {
            ($target:expr, $stored:expr) => {
                for row in $stored.rows() {
                    $target.insert(row.clone()).unwrap();
                }
            };
        }
        retain!(data.native.qualifications, qualifications);
        retain!(data.native.scopes, coverage_scopes);
        retain!(data.native.modules, modules);
        retain!(data.native.mentions, mentions);
        retain!(
            data.native.relation_mention_entity_assessments,
            mention_assessments
        );
        retain!(
            data.native.relation_mention_entity_candidates,
            mention_candidates
        );
        retain!(data.native.entity_exposure_candidates, exposure_candidates);
        retain!(data.native.symbol_resolutions, resolutions);
        retain!(data.uses, uses);
        retain!(data.corpus_libraries, corpus_libraries);
        let scope = members
            .iter()
            .filter(|u| u.release_scope)
            .map(|u| u.entity)
            .collect::<std::collections::BTreeSet<_>>();
        let mut replay = Output::new(&budget);
        lctx_model::domain::analytics::mention_layer(&data, frame, &scope, &mut replay, &budget)
            .unwrap();
        let expected_mentions = contributions
            .rows()
            .iter()
            .filter(|c| c.frame == frame.id() && c.layer == Layer::Mention)
            .collect::<Vec<_>>();
        assert_eq!(
            expected_mentions.len(),
            1,
            "one exact passage supplies one entity-pair vote"
        );
        assert_eq!(replay.contributions.len(), 1);
        assert_eq!(
            replay.contributions.get(expected_mentions[0].id()),
            Some(expected_mentions[0]),
            "shared production operation must reproduce the actual persisted co-mention premise"
        );
        assert_eq!(replay.pair_sources.len(), 1);
        let original_source = provenance
            .rows()
            .iter()
            .find(|p| p.id() == expected_mentions[0].source)
            .unwrap();
        assert_eq!(
            replay.pair_sources.get(original_source.id()),
            Some(original_source)
        );
        drop(replay);
        data.corpus_libraries = Rows::new(&budget);
        let mut unlinked = Output::new(&budget);
        lctx_model::domain::analytics::mention_layer(&data, frame, &scope, &mut unlinked, &budget)
            .unwrap();
        assert!(
            unlinked.contributions.is_empty() && unlinked.pair_sources.is_empty(),
            "removing actual CorpusLibrary authority must refuse the same stored document and resolutions"
        );
        drop(unlinked);
        for wrong in [
            CorpusLibrary {
                corpus: document.input,
                library: document.input,
            },
            CorpusLibrary {
                corpus: parent.input,
                library: parent.input,
            },
        ] {
            data.corpus_libraries = Rows::new(&budget);
            data.corpus_libraries.insert(wrong).unwrap();
            let mut refused = Output::new(&budget);
            lctx_model::domain::analytics::mention_layer(
                &data,
                frame,
                &scope,
                &mut refused,
                &budget,
            )
            .unwrap();
            assert!(
                refused.contributions.is_empty() && refused.pair_sources.is_empty(),
                "a wrong corpus or library identity must not admit the original document"
            );
        }
        drop(data);
        for (method, on) in [
            (M::Communities, true),
            (M::PageRank, true),
            (M::Concepts, true),
            (M::RelationalConcepts, true),
            (M::Neighbours, false),
        ] {
            let result = results
                .rows()
                .iter()
                .find(|r| r.frame == frame.id() && r.method == method)
                .unwrap();
            assert_eq!(result.selected, on);
            if on {
                assert!(
                    matches!(result.status, S::Completed | S::Partial),
                    "selected method must execute: {result:?}"
                );
                assert_ne!(result.stop, Stop::NotRequested);
            } else {
                assert_eq!(
                    (result.status, result.stop),
                    (S::NotRequested, Stop::NotRequested)
                );
            }
            let invocation = invocations
                .rows()
                .iter()
                .find(|i| i.id() == result.invocation)
                .unwrap();
            let definition = definitions
                .rows()
                .iter()
                .find(|d| d.id() == invocation.definition)
                .unwrap();
            let parameter = parameters
                .rows()
                .iter()
                .find(|p| p.id() == definition.parameters)
                .unwrap();
            assert_eq!(definition.method, method);
            // Shared policy lowering owns the recipe/digest; independent constants pin Q0's fixed inputs.
            assert_eq!(
                *definition,
                build::definition(configuration, method).unwrap().1
            );
            assert_eq!(parameter.depth, Some(256));
            assert_eq!(parameter.work, Some(100_000_000));
            assert_eq!(
                parameter.threshold.unwrap().get(),
                if method == M::Neighbours { 0.5 } else { 1e-10 }
            );
            if method == M::PageRank {
                assert_eq!(parameter.damping.unwrap().get(), 0.85);
            }
            if matches!(method, M::PageRank | M::Communities) {
                assert_eq!(parameter.iterations, Some(100));
            }
            if method == M::Communities {
                assert_eq!(parameter.seed, Some(0));
                assert_eq!(parameter.resolution.unwrap().get(), 1.0);
            }
            if matches!(method, M::Concepts | M::RelationalConcepts) {
                assert_eq!(parameter.proof_steps, Some(20_000));
                assert_eq!(definition.interpretation, Interpretation::ExactUnderContext);
            }
            let outcome = outcomes
                .rows()
                .iter()
                .find(|o| o.invocation == invocation.id())
                .unwrap();
            assert_eq!(
                *outcome,
                lctx_model::domain::analytics::frames::outcome(result)
            );
            let parent_ids = parents
                .rows()
                .iter()
                .filter(|p| p.invocation == invocation.id())
                .map(|p| p.parent)
                .collect::<std::collections::BTreeSet<_>>();
            for expected in [predecessor.invocation, predecessor.usage_invocation]
                .into_iter()
                .chain((method == M::RelationalConcepts).then_some(predecessor.handoff_invocation))
            {
                let parent = analysis::analytic::InvocationSource::Structural {
                    invocation: expected,
                };
                assert!(parent_ids.contains(&parent.id()) && sources.rows().contains(&parent));
            }
            assert!(
                !receipts
                    .rows()
                    .iter()
                    .filter(|r| r.invocation == invocation.id())
                    .collect::<Vec<_>>()
                    .is_empty()
            );
            if matches!(method, M::Concepts | M::RelationalConcepts) {
                let selected_scopes = scopes
                    .rows()
                    .iter()
                    .filter(|s| s.result == result.id())
                    .collect::<Vec<_>>();
                assert!(
                    !selected_scopes.is_empty(),
                    "selected FCA/RCA must retain finite scopes"
                );
                let incidence = incidences.rows().iter().filter(|i| i.entity == api
                    && selected_scopes.iter().any(|s| s.id() == i.scope)
                    && attributes.rows().iter().any(|a| a.id() == i.attribute
                        && matches!(a, Attribute::Parameter { name, .. } if name.as_str() == "flag")))
                    .collect::<Vec<_>>();
                assert_eq!(
                    incidence.len(),
                    1,
                    "defaulted flag supplies one source formal incidence per method"
                );
                let IncidenceSource::Parameter { observation } = incidence_sources
                    .rows()
                    .iter()
                    .find(|s| s.id() == incidence[0].source)
                    .unwrap()
                else {
                    panic!("flag lost its source formal provenance");
                };
                let formal = formal_syntax
                    .rows()
                    .iter()
                    .find(|p| p.id() == *observation)
                    .unwrap();
                assert!(
                    formal.default_literal.is_some(),
                    "source-default parameter is retained"
                );
                assert!(!incidences.rows().iter().any(|i| i.entity == entity("demo.Holder.method")
                    && selected_scopes.iter().any(|s| s.id() == i.scope)
                    && attributes.rows().iter().any(|a| a.id() == i.attribute
                        && matches!(a, Attribute::Parameter { name, .. } if name.as_str() == "self"))),
                    "bound source receiver remains excluded from FCA/RCA votes");
                assert!(objects.rows().iter().any(
                    |o| o.entity == first && selected_scopes.iter().any(|s| s.id() == o.scope)
                ));
            }
        }
        let rank = results
            .rows()
            .iter()
            .find(|r| r.frame == frame.id() && r.method == M::PageRank)
            .unwrap();
        let targets = ranks
            .rows()
            .iter()
            .filter(|r| r.result == rank.id())
            .map(|r| r.target)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            targets,
            members
                .iter()
                .filter(|u| u.graph)
                .map(|u| u.entity)
                .collect()
        );
        let communities = results
            .rows()
            .iter()
            .find(|r| r.frame == frame.id() && r.method == M::Communities)
            .unwrap();
        assert_eq!(
            runs.rows()
                .iter()
                .filter(|r| r.result == communities.id())
                .count(),
            40,
            "fixed four-resolution/ten-seed policy executes without vectors"
        );
        for layer in [Layer::Type, Layer::Mention] {
            let selected = pairs
                .rows()
                .iter()
                .filter(|p| p.frame == frame.id() && p.layer == layer)
                .collect::<Vec<_>>();
            assert_eq!(
                selected.len(),
                1,
                "predeclared Token/co-mention endpoints form one pair: {selected:?}"
            );
            let pair = selected[0];
            assert_eq!((pair.left, pair.right), endpoints);
            assert_eq!(
                pair.count, 1,
                "repeated mentions never multiply entity votes"
            );
            assert_eq!(pair.normalized_weight.get(), 1.0);
            let contribution = contributions
                .rows()
                .iter()
                .find(|c| {
                    c.frame == frame.id() && c.layer == layer && (c.left, c.right) == endpoints
                })
                .unwrap();
            let source = provenance
                .rows()
                .iter()
                .find(|p| p.id() == contribution.source)
                .unwrap();
            match source {
                PairSource::Type { left, right, .. } => {
                    for id in [left, right] {
                        let observation = types.rows().iter().find(|o| o.id() == *id).unwrap();
                        assert!(observation.declared && observation.role == TypeRole::Parameter);
                    }
                }
                PairSource::Mention { left, right } => {
                    for id in [left, right] {
                        let mention = mentions.rows().iter().find(|m| m.id() == *id).unwrap();
                        assert_eq!(mention.class, MentionClass::Exact);
                        let qualification = qualifications
                            .rows()
                            .iter()
                            .find(|q| q.id() == mention.qualification)
                            .unwrap();
                        assert!(coverage_scopes.rows().iter().any(|s| s.id() == qualification.scope
                            && matches!(s, CoverageScope::Artifact { artifact } if *artifact == document.id())),
                            "co-mention provenance must cite the official document artifact");
                    }
                }
                _ => panic!("selected {layer:?} pair has wrong provenance: {source:?}"),
            }
        }
        let pair = combined
            .rows()
            .iter()
            .find(|p| p.frame == frame.id() && (p.left, p.right) == endpoints)
            .unwrap();
        assert_eq!(
            pair.weight.get(),
            0.5,
            "two active unit layers retain their authored quarter weights"
        );
    }
    let mismatches: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM {}.analytic_source_receipts r LEFT JOIN lctx_model_store.stage_receipts c ON c.generation_id=decode($1,'hex') AND c.relation_name=r.relation AND c.stage_name=r.producer WHERE c.content_digest IS DISTINCT FROM r.content OR c.row_count IS DISTINCT FROM r.rows",
        fixture.generation.schema()))).bind(fixture.generation.hex()).fetch_one(fixture.db.owner.pool()).await.unwrap();
    assert_eq!(
        mismatches, 0,
        "analytic provenance must match actual same-generation stage receipts"
    );
}

fn runner(fixture: &ServingFixture, profile: &str, receipts: &Path) -> std::process::Command {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut command = std::process::Command::new("uv");
    command
        .current_dir(&root)
        .args([
            "run",
            "--no-sync",
            "python",
            "scripts/qualify_serving.py",
            "--library",
            "demo",
            "--profile",
            profile,
            "--config",
        ])
        .arg(fixture.dir.path().join("postgres-serving.json"))
        .arg("--generation")
        .arg(fixture.generation.hex())
        .arg("--source-file")
        .arg(fixture.dir.path().join("source.py"))
        .arg("--anchors")
        .arg(fixture.dir.path().join("anchors.json"))
        .arg("--receipt-dir")
        .arg(receipts);
    command
}

#[tokio::test]
async fn runner_uses_actual_stdio_and_challenges_received_generation_evidence_channels() {
    // These bytes and signatures are independent of the runner and server helpers.
    let source = format!(
        "\"\"\"Public api consume method signature source evidence.\n{}\"\"\"\n{}",
        "🦀 café\n".repeat(5000),
        r#"__all__ = ['api', 'consume', 'Holder', 'Token', 'first', 'second']
def api(flag: bool = False) -> bool:
    return flag
def consume(value: int) -> int:
    return value
class Holder:
    def method(self, token: str = 'ready') -> str:
        if not isinstance(token, str):
            raise TypeError('token must be str')
        return token
class Token:
    pass
def first(value: Token) -> Token:
    return value
def second(value: Token) -> Token:
    return value
"#
    )
    .into_bytes();
    let anchors = serde_json::json!([
        {"path":"demo.api", "qualname":"api"},
        {"path":"demo.consume", "qualname":"consume"},
        {"path":"demo.Holder.method", "qualname":"Holder.method"}
    ]);
    for profile in ["catalog", "behavioral"] {
        let fixture = ServingFixture::start_with_analytics(
            &source,
            profile,
            SEEDS,
            4,
            Some(TECHNIQUES),
            Some(DOCUMENT),
        )
        .await;
        assert_selected_analytics(&fixture).await;
        write(&fixture.dir.path().join("source.py"), &source);
        write(
            &fixture.dir.path().join("anchors.json"),
            anchors.to_string(),
        );
        let receipt_root = std::env::var_os("LCTX_SERVING_QUALIFICATION_RECEIPTS_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("lctx-serving-qualification"));
        let receipts = receipt_root.join(format!("{}-{profile}", fixture.generation.hex()));
        let output = runner(&fixture, profile, &receipts).output().unwrap();
        assert!(
            output.status.success(),
            "{profile}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let receipt: serde_json::Value =
            serde_json::from_slice(&std::fs::read(receipts.join("receipt.json")).unwrap()).unwrap();
        assert_eq!(receipt["outcome"], "passed");
        assert_eq!(receipt["generation"], fixture.generation.hex());
        assert_eq!(receipt["tools"].as_array().unwrap().len(), 10);
        assert_eq!(
            std::fs::read(receipts.join("original-source.bin")).unwrap(),
            source
        );
        // Read actual protocol replies, then challenge independent qualification assertions.
        // This proves the oracle detects bad values rather than trusting successful dispatch.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fault = std::process::Command::new("uv").current_dir(root)
            .args(["run", "--no-sync", "python", "-c", r#"
import copy, importlib.util, json, pathlib, sys
spec = importlib.util.spec_from_file_location('qualify', 'scripts/qualify_serving.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)
folder = pathlib.Path(sys.argv[1])
receipt = json.loads((folder/'receipt.json').read_text())
dtos = [json.loads(p.read_bytes()).get('result', {}).get('structuredContent') for p in folder.glob('*.response.jsonl')]
dtos = [d for d in dtos if d]
generation = dtos[0]
q.check_generation(generation, list(bytes.fromhex(receipt['generation'])))
ranked = next(d for d in dtos if 'channels' in d)
q.check_channels(ranked)
original = (folder/'original-source.bin').read_bytes()
q.check_original(original, pathlib.Path(sys.argv[2]).read_bytes())
core = next(d['operation']['packet']['core'] for d in dtos if 'operation' in d and d['operation'].get('packet', {}).get('core', {}).get('name') == 'demo.api')
node = q.source_functions(original)['api']
schema = json.loads(q.wire_tool('get_operation'))['output_schema']
q.check_signature(core, node, schema)
wrong_default = copy.deepcopy(core)
default_id = next(p['default']['literal'] for s in wrong_default['signatures'] for p in s['parameters'] if p['name'] == 'flag')
next(l for l in wrong_default['literal_values'] if l['literal'] == default_id)['value'] = {'kind':'bool','value':True}
wrong_channel = copy.deepcopy(ranked)
wrong_channel['channels']['vector'] = {'status':'available'}
for check, arguments in [(q.check_generation,(generation,[255]*16)),(q.check_channels,(wrong_channel,)),(q.check_original,(original,original+b'\n# wrong source\n')),(q.check_signature,(wrong_default,node,schema))]:
    try:
        check(*arguments)
    except AssertionError:
        pass
    else:
        raise AssertionError('fault escaped independent check')
assert len([d for d in dtos if 'evidence' in d]) > 1, 'actual byte continuation required'
print('actual received generation/evidence/channel/default faults detected')
"#]).arg(&receipts).arg(fixture.dir.path().join("source.py"))
            .output().unwrap();
        assert!(
            fault.status.success(),
            "{}",
            String::from_utf8_lossy(&fault.stderr)
        );
        println!("{profile}: {}", String::from_utf8_lossy(&fault.stdout));
        fixture.finish().await;
    }
}
