//! Id and digest derivation (DESIGN §3.4.1, ADR-0007).
//!
//! `BLAKE3("lctx-id/v1" ‖ len‖kind_tag ‖ len‖field …)` with u64 little-endian lengths. Ids are the
//! first 16 bytes of the hash; digests are all 32. The tag is a contract: changing the encoding is
//! `lctx-id/v2` and a migration (DM-51).

pub use lctx_model::id::{Digest, Id, IdHasher};
/// The legacy encoding tag. Part of every legacy id and digest.
pub use lctx_model::legacy::ID_TAG_V1 as ID_TAG;

/// Kind tags. A new kind is appended; an existing tag never changes meaning.
pub mod kind {
    pub const RELEASE: &str = "release";
    pub const CONTEXT: &str = "context";
    pub const PRODUCER: &str = "producer";
    pub const RUN: &str = "run";
    pub const MODULE: &str = "module";
    pub const DECLARATION: &str = "declaration";
    pub const SYNTAX: &str = "syntax";
    pub const FACT: &str = "fact";
    pub const CONTENT: &str = "content";
    pub const ENVIRONMENT: &str = "environment";
    pub const COMPILER: &str = "compiler";
    pub const SNAPSHOT_CONTENT: &str = "snapshot-content";
    // The graph catalog (DESIGN §3.4.1, §3.8; ADR-0014).
    pub const EDGE: &str = "edge";
    pub const ARGUMENT: &str = "argument";
    pub const EXPORT: &str = "export";
    pub const SYNTHETIC_CALLABLE: &str = "synthetic_callable";
    pub const EXTERNAL_MODULE: &str = "external_module";
    pub const EXTERNAL_SYMBOL: &str = "external_symbol";
    /// A Pysa call row's run-independent payload digest (`pysa_calls.payload_id`).
    pub const PYSA_CALL: &str = "pysa-call";
    // The lexical family (C3).
    pub const SCOPE: &str = "scope";
    pub const BINDING: &str = "binding";
    pub const REFERENCE: &str = "reference";
    pub const TYPE: &str = "type";
    pub const FIELD: &str = "field";
    pub const DOCUMENT: &str = "document";
    pub const PASSAGE: &str = "passage";
    pub const CODE_BLOCK: &str = "code_block";
    // Analysis results (ADR-0019).
    pub const ANALYTICS_CONFIG: &str = "analytics-config";
    pub const INVOCATION: &str = "invocation";
    pub const FINDING: &str = "finding";
    pub const EVIDENCE: &str = "evidence";
    pub const ASSERTION: &str = "assertion";
    pub const BRIEF: &str = "brief";
    pub const SUMMARY_FLOW: &str = "summary-flow";
    pub const SUMMARY_COMPONENT: &str = "summary-component";
    pub const VALUE_FLOW_ORIGIN: &str = "value-flow-origin";
}

/// The id recipes computed in Rust whose inputs are also columns, so the `lctx_id` UDF recomputes
/// them in SQL and a generated rule checks they agree (DESIGN §3.4.1, ADR-0014). Every field uses
/// the `opt_*` encoding, as the UDF does.
pub mod recipe {
    use super::{Digest, Id, IdHasher, kind};

    /// Identity of one unaggregated source contribution. These are precisely the semantic
    /// columns of the `value_flow_contributions` key; a presentation fact may have many origins.
    #[derive(Clone, Copy)]
    pub struct ValueFlowOriginKey<'a> {
        pub fact: Id,
        pub use_id: Id,
        pub source_key: &'a str,
        pub identity: bool,
        pub through_call: bool,
        pub local_through_call: bool,
        pub upstream_identity: bool,
        pub upstream_through_call: bool,
    }

    pub fn value_flow_origin(spec: &ValueFlowOriginKey<'_>) -> Id {
        IdHasher::new(kind::VALUE_FLOW_ORIGIN)
            .opt_id(Some(spec.fact))
            .opt_id(Some(spec.use_id))
            .opt_str(Some(spec.source_key))
            .opt_bool(Some(spec.identity))
            .opt_bool(Some(spec.through_call))
            .opt_bool(Some(spec.local_through_call))
            .opt_bool(Some(spec.upstream_identity))
            .opt_bool(Some(spec.upstream_through_call))
            .finish_id()
    }
    use crate::codebook::{Codebook, SummaryFlowKind, SummaryFlowStepKind};

    /// A call's argument at an ordinal: a role, never the expression's own id.
    pub fn argument(call: Id, ordinal: i64) -> Id {
        IdHasher::new(kind::ARGUMENT)
            .opt_id(Some(call))
            .opt_i64(Some(ordinal))
            .finish_id()
    }

    /// A dependency module: its owner (the distribution whose `RECORD` lists the file, else
    /// `pyrefly-bundled`, else `unowned`), the owner's version (else the fork revision, else the
    /// file's content digest), and its name.
    pub fn external_module(owner: &str, owner_version: &str, module: &str) -> Id {
        IdHasher::new(kind::EXTERNAL_MODULE)
            .opt_str(Some(owner))
            .opt_str(Some(owner_version))
            .opt_str(Some(module))
            .finish_id()
    }

    /// A lexical scope: the node that opens it (the module, a declaration, a lambda or a
    /// comprehension).
    pub fn scope(owner: Id) -> Id {
        IdHasher::new(kind::SCOPE).opt_id(Some(owner)).finish_id()
    }

    /// A binding event: its site (a declaration, a parameter, or the syntax id of the name,
    /// alias, handler or statement that binds) and the name it binds there.
    pub fn binding(site: Id, name: &str) -> Id {
        IdHasher::new(kind::BINDING)
            .opt_id(Some(site))
            .opt_str(Some(name))
            .finish_id()
    }

    /// A name reference: a role of the name's syntax node.
    pub fn reference(name_node: Id) -> Id {
        IdHasher::new(kind::REFERENCE)
            .opt_id(Some(name_node))
            .finish_id()
    }

    /// A record field: a role of its class (C4).
    pub fn field(class: Id, name: &str) -> Id {
        IdHasher::new(kind::FIELD)
            .opt_id(Some(class))
            .opt_str(Some(name))
            .finish_id()
    }

    /// A document of a corpus release (C5).
    pub fn document(release: Id, path: &str) -> Id {
        IdHasher::new(kind::DOCUMENT)
            .opt_id(Some(release))
            .opt_str(Some(path))
            .finish_id()
    }

    /// A passage or code block: a role of its document at an ordinal (C5).
    pub fn passage(document: Id, ordinal: i64) -> Id {
        IdHasher::new(kind::PASSAGE)
            .opt_id(Some(document))
            .opt_i64(Some(ordinal))
            .finish_id()
    }

    pub fn code_block(document: Id, ordinal: i64) -> Id {
        IdHasher::new(kind::CODE_BLOCK)
            .opt_id(Some(document))
            .opt_i64(Some(ordinal))
            .finish_id()
    }

    /// A producer: its tool, its revision and its build digest (DESIGN §3.4.1).
    pub fn producer(tool: &str, revision: &str, build: Digest) -> Id {
        IdHasher::new(kind::PRODUCER)
            .str(tool)
            .str(revision)
            .digest_field(build)
            .finish_id()
    }

    /// A run: its release, context and producer, its sorted families and the producer's own
    /// config digest (DESIGN §3.4.1).
    pub fn run(release: Id, context: Id, producer: Id, families: &[&str], config: Digest) -> Id {
        let mut sorted = families.to_vec();
        sorted.sort_unstable();
        IdHasher::new(kind::RUN)
            .id(release)
            .id(context)
            .id(producer)
            .strs(sorted)
            .digest_field(config)
            .finish_id()
    }

    /// A definition in a dependency module: its module, definition kind code and Pysa key.
    pub fn external_symbol(module: Id, definition_kind: i16, key: &str) -> Id {
        IdHasher::new(kind::EXTERNAL_SYMBOL)
            .opt_id(Some(module))
            .opt_i64(Some(i64::from(definition_kind)))
            .opt_str(Some(key))
            .finish_id()
    }

    /// A finite summary path, not merely its endpoints. Ordered typed evidence keeps parallel
    /// call chains distinct even when they reach the same raw return fact and condition.
    #[derive(Clone, Copy)]
    pub struct SummaryFlowProofStep {
        pub kind: SummaryFlowStepKind,
        pub evidence_id: Id,
        pub condition_id: Id,
    }

    pub struct SummaryFlowIdentity<'a> {
        pub function: Id,
        pub parameter: Id,
        pub source_origin: Id,
        pub input_path: &'a str,
        pub output_path: &'a str,
        pub transfer_kind: SummaryFlowKind,
        pub condition: Id,
        pub return_site: Id,
        pub return_region: Id,
        pub steps: &'a [SummaryFlowProofStep],
    }

    pub fn summary_flow(spec: &SummaryFlowIdentity<'_>) -> Id {
        let mut hasher = IdHasher::new(kind::SUMMARY_FLOW);
        hasher
            .id(spec.function)
            .id(spec.parameter)
            .id(spec.source_origin)
            .str(spec.input_path)
            .str(spec.output_path)
            .i64(i64::from(spec.transfer_kind.code()))
            .id(spec.condition)
            .id(spec.return_site)
            .id(spec.return_region)
            .i64(spec.steps.len() as i64);
        for step in spec.steps {
            hasher
                .i64(i64::from(step.kind.code()))
                .id(step.evidence_id)
                .id(step.condition_id);
        }
        hasher.finish_id()
    }

    /// The identity of a sorted, nonempty SCC member set; schedule order is not identity.
    pub fn summary_component(members: &[Id]) -> Id {
        let mut hasher = IdHasher::new(kind::SUMMARY_COMPONENT);
        hasher.i64(members.len() as i64);
        for member in members {
            hasher.id(*member);
        }
        hasher.finish_id()
    }
}

/// The digest of arbitrary content bytes (source files, configurations, manifests).
pub fn content_digest(bytes: &[u8]) -> Digest {
    IdHasher::new(kind::CONTENT).bytes(bytes).finish_digest()
}
