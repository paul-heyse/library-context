//! Producer identity for the dormant analysis stages (plan P1.3): the compiler output version, its
//! locked engines and the digests over its sources and declarations. The Delta compile
//! orchestration that used to own this is removed; `analyze` still records it as the compiler
//! producer's build digest until phases 3–5 replace that producer.
use cpg_schema::derived::derivations;
use cpg_schema::id::{Digest, IdHasher, kind};
use cpg_schema::rules::rules;
use cpg_schema::tables::contracts;

/// Bumped by hand whenever the derive, cast, sort or analysis code changes output for the same
/// inputs; the derived-table and analysis snapshots are what show such a change. 2: Stage E
/// (ADR-0019). 3: definition arcs, the witness-cap kind fix (slice 1.4 review F1, F3). 4: the
/// documents' spec hash and `embedding_specs` (slice 1.7). 5: Pass B and parameter docs (slice
/// 2.1). 6: Pass C (slice 2.2). 7: the slice 2.1 review: handler, conditional and tested calls,
/// unfollowed arguments, supported predicates, receivers by kind, header-anchored descriptions.
/// 8: communities and their invocations' diagnostics (slice 2.3). 9: centrality (slice 2.4).
/// 10: the slice 2.2 review: release handoff endpoints, one-target producers, the narrowed
/// receiver exclusion, doc blocks by document, self-contained usage patterns. 11: the ADR-0011
/// review: γ = 1 fixed, the public co-assignment score, named layer policies. 12: FCA (slice 2.5).
/// 13: seed selection within the brief budget (slice 2.6). 14: E0 and kNN (slice 3.1). 15: the
/// increment-2 review: direct usage and selection by it, one preferred path per callable, FCA
/// scopes keyed by node, attributes from term structure (no `Unknown`, one raised class). 16: the
/// selection invocation records the whole technique set (the ADR-0020 review's F3).
/// 17: direct entry-formal/test-use links (Stage 3.0). 18: resolved builtin `type(x) is C`
/// atoms, their guarded entry-value links and exact-class origins. 19: path-stable later-use
/// links under an exact type guard. 20: committed typed model catalog identity and validation.
/// 21: pinned-source model target bindings and their publication contract. 22: compiled authored
/// transfer rows, gated on those target bindings. 23: external Pysa signatures and model formal
/// path validation. 24: attributed explicit exits and finally-body actions. 25: attributed
/// except clauses and direct handler actions. 26: authored typed effects and their pinned binding.
/// 27: typed callback, resource and exception model assertions. 28: cited handler type status.
/// 29: unresolved try/with frames withhold definite raise escape (ADR-0027).
/// 30: cited model applications at source call sites. 31: typed model formal paths.
/// 32: exact per-signature model formal to source-argument bindings.
/// 33: candidate-local modeled callback sites and binding boundaries.
/// 34: candidate-local modeled resource sites. 35: candidate-local modeled transfer sites.
/// 36: candidate-local modeled effect sites. 37: exact flow-call source links (ADR-0028).
/// 38: pinned exception class identities and candidate-local modeled exception sites.
/// 41: preserve unaggregated provider-value contributions for exact call-path composition.
/// 42: decide clause order within a modeled exception's candidate try frame.
/// 43: name the sink callable separately from a captured source parameter's owner.
/// 44: preserve the upstream transfer before each raw value fact's local call path.
/// 45: direct, pre-finally return-None witnesses for handler bodies.
/// 46: candidate modeled raises compose through a direct first handler to a return-None path.
/// 47: direct one-call return candidates join raw parameter flows to pinned model transfers.
/// 48: raw predecessor candidates retain reaching-definition identity and separate conditions.
/// 49: persist the BDD closures of recomposed flow-analysis conditions.
/// 50: a direct modeled return requires the call result to be the entire sink expression.
/// 51: bounded BDD compatibility of cited predecessor-path conditions.
/// 52: exact model-call steps apply to whole definition values as well as whole returns.
/// 53: cite a condition-checked identity return from a whole-assignment modeled value.
/// 54: finite direct identity-return summary seeds with structural condition decisions.
/// 55: explicit incomplete parameter-to-return summary boundaries.
/// 56: pinned model targets carry an explicit total-normal-return assertion.
/// 57: model applications retain actual target count and selected target completion assertion.
/// 58: finite summary paths gain canonical ids and ordered, typed proof steps.
/// 59: each modeled exact-value candidate records ordered argument evaluation evidence.
/// 60: exact unshadowed builtin names gain local normal-evaluation evidence.
/// 61: first finite modeled identity-return paths carry complete ordered call proofs.
/// 62: unique assignment-to-return predecessors gain finite model-call summaries.
/// 63: attributed source call SCCs have canonical callee-first component rows.
/// 64: acyclic exact local wrappers compose unconditional callee value summaries.
/// 65: bounded return-frame statuses gate nested finite normal-return summary seeds.
/// 66: one cited literal-finally-pass frame admits a pending normal return.
/// 67: an admitted summary cannot erase a sibling source contribution's open boundary.
/// 68: pinned Pydantic TypeAdapter validation candidate model and fixture coverage.
/// 69: finite returns through a sole pass finalizer cite the pass in ordered summary steps.
/// 70: nested pass-only finalizers cite every source action in inner-to-outer proof order.
/// 71: direct and modeled finite flows in recursive SCCs remain explicit unknown paths.
/// 72: direct paths screen earlier source calls, admitting a recursive base before its self-call.
/// 73: incompatible earlier call regions no longer block a finite direct return path.
/// 74: raw value-flow contribution keys retain local and upstream transfer provenance;
/// multi-release validation accepts one snapshot with multiple releases.
/// 76: normalized local-call arguments, multi-control Boolean specialization and bounded
/// closed-expression completion with separately represented exact Boolean values.
/// 77: shared source-admitted expression evaluation and ordered operand proofs.
/// 78: condition-keyed ordered return-entry completion proofs.
/// 79: typed handler/re-raise completion and native context-MRO completeness.
/// 80: exact completion exceptions and origin/channel coverage.
/// 81: default-availability admission and semantic/witness-separated value fixed point.
/// 82: definition-time header completion, typed site/origin coverage and shared callee proof admission.
/// 83: independently authored model phases and catalog format 2.
/// 84: source binding and call-specific fresh default availability/value/stability evidence.
/// 85: typed static/runtime/unresolved validation schema contracts and catalog format 3.
/// 86: condition-safe transfer alternatives and typed behavior transfer/scope in FORMAT 9.
/// 87: occurrence-specific source parameter identity independent of provider reach approximation.
/// 88: typed current FCA/RCA attributes, incidence evidence and presentation-only labels.
/// 89: independently bound synchronous class protocols and mandatory return obligations (catalog format 4).
/// 90: separate context entry-value identity and mandatory base value witnesses.
/// 91: named-handler binding/implicit-deletion boundary retained through serving.
/// 92: invocation prefixes and source call execution independent of callee completion.
/// 93: independent lexical input/model identity for direct modeled returns.
/// 94: explicit authored action triggers and candidate-backed action assessments.
/// 95: pinned default availability and independently committed omitted-formal obligations.
/// 96: normal-exit action implications with mandatory undischarged outcome obligations.
/// 97–100: frame/body/fresh-call completion and independent source invocation.
/// 101: nondominated depth/proof-cost progress through shared bounded scheduling helpers.
/// 102: agree on repeated pinned context observations without counting them as overloads.
/// 103: builtin binding-preserving descriptors no longer write the decorator boundary.
/// 104: summaries neither start in nor compose through a decorated function (ADR-0064).
/// 105: call-transfer return claims graded after summaries with claim-keyed discharges (ADR-0064).
/// 112: canonical semantic producer identity and validated coarse-stage reuse (ADR-0081).
pub const COMPILER_OUTPUT_VERSION: u32 = 112;

/// The locked engines (DataFusion, Arrow, Parquet, object_store, delta-rs, its kernel), read from
/// `Cargo.lock` at build time (`build.rs`).
pub const ENGINES: &str = env!("LCTX_ENGINES");

/// The compiler's identity from its parts (review F4).
pub fn compiler_digest_of(
    engines: &str,
    output_version: u32,
    udf_version: u32,
    template_version: i64,
    derivations: &[(&str, String)],
    contracts: &[(&str, String)],
    rules: &[(String, String)],
) -> Digest {
    let mut h = IdHasher::new(kind::COMPILER);
    h.str(engines)
        .i64(i64::from(output_version))
        .i64(i64::from(udf_version))
        .i64(template_version);
    for (name, text) in derivations.iter().chain(contracts) {
        h.str(name).str(text);
    }
    for (name, sql) in rules {
        h.str(name).str(sql);
    }
    h.finish_digest()
}

/// The identity of the code that derives, analyzes, validates and publishes: the locked engines
/// and analysis libraries, the output version, the synthesis template version (which stands for
/// Stage F's queries and templates: the analysis ledger fails any output change without a bump;
/// increment-1 deep review F1), every derivation query, declared projection, Pass B and community
/// relation, the pre-registered community parameters (slice 2.3),
/// every table contract and every validation rule. Stored on each `snapshots` row and folded into
/// `content_digest`.
/// The digest of the compiler's sources (`build.rs`): every `.rs` file of `cpg-core`,
/// `lctx-analytics` and `cpg-schema`.
pub const SOURCE_DIGEST: &str = env!("LCTX_SOURCE_DIGEST");

/// Identity of canonical semantic production; serving-only code is still full compiler provenance.
pub fn semantic_digest() -> Digest {
    compiler_digest_for(env!("LCTX_SEMANTIC_SOURCE_DIGEST"))
}

/// Producer identity (DESIGN §15.3; review F11): every canonical producer's code. Recorded as
/// the compiler producer's `build_digest`, so a canonical row names the code that produced it.
pub fn producer_digest() -> Digest {
    compiler_digest_for(env!("LCTX_PRODUCER_SOURCE_DIGEST"))
}

/// The files producer identity leaves out: serving-only realization code.
pub const PRODUCER_EXCLUDED: &str = env!("LCTX_PRODUCER_EXCLUDED");

pub fn compiler_digest() -> Digest {
    compiler_digest_for(SOURCE_DIGEST)
}

fn compiler_digest_for(source: &str) -> Digest {
    let rules: Vec<(String, String)> = rules().into_iter().map(|r| (r.name, r.sql)).collect();
    let mut queries = derivations();
    for spec in cpg_schema::projection::projections() {
        queries.push((spec.name, spec.digest().hex()));
    }
    queries.push(("pass_b_relations", cpg_schema::flows::digest().hex()));
    // The compiler's own sources (the holistic assessment's A2(e); `build.rs`).
    queries.push(("compiler_sources", source.to_owned()));
    // The one public-path authority (the holistic assessment's A1): seeds resolve by it.
    for relation in cpg_schema::public::all()
        .into_iter()
        .chain(cpg_schema::public::candidate_relations())
        .chain(cpg_schema::public::member_relations())
    {
        queries.push((relation.name, relation.sql));
    }
    queries.push((
        "community_relations",
        cpg_schema::communities::digest().hex(),
    ));
    queries.push((
        "community_parameters",
        lctx_analytics::communities::Params::preregistered()
            .digest()
            .hex(),
    ));
    queries.push((
        "usage_projection",
        lctx_analytics::ranking::projection_digest(cpg_schema::projection::invocation().digest())
            .hex(),
    ));
    queries.push(("concept_relations", cpg_schema::concepts::digest().hex()));
    queries.push((
        "behavior_models_catalog",
        cpg_schema::models::Catalog::committed_digest().hex(),
    ));
    queries.push((
        "neighbour_relations",
        cpg_schema::neighbours::digest().hex(),
    ));
    queries.push((
        "knn_parameters",
        lctx_analytics::neighbours::Params::preregistered()
            .digest()
            .hex(),
    ));
    queries.push((
        "fca_parameters",
        lctx_analytics::concepts::Params::preregistered()
            .digest()
            .hex(),
    ));
    queries.push((
        "pagerank_parameters",
        lctx_analytics::ranking::Params::preregistered()
            .digest()
            .hex(),
    ));
    compiler_digest_of(
        &format!("{ENGINES}; {}", lctx_analytics::LIBRARIES),
        COMPILER_OUTPUT_VERSION,
        crate::udf::VERSION,
        crate::synth::TEMPLATE_VERSION,
        &queries,
        &contracts(),
        &rules,
    )
}
