//! Serving ownership intent. Native graph layout and SurrealQL are mechanical lowerings.
use super::{charged::StateCharge, resources::ResourceBudget, *};
use std::sync::Arc;

/// Incoming ownership deliberately excludes shared capture/context/provider/type identities.
pub const OWNED_FIELDS: &[&str] = &[
    "member",
    "occurrence",
    "declaration",
    "exposure",
    "candidate",
    "callable",
    "invocation",
    "variant",
    "signature",
    "parameter",
    "field_option",
    "parameter_option",
    "slot",
    "domain",
    "witness",
    "assessment",
    "observation",
    "subject",
    "conclusion",
    "proof",
    "assertion",
    "universe",
    "set",
    "sequence",
    "list",
    "field_list",
    "parameters",
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServingEdge {
    Reference,
    Participant,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncomingOwner {
    ProviderRun,
    ProviderCoverage,
    RunFamily,
    GuardSubstitution,
    TypeBinderAssessment,
    TypeEntityLink,
    PlaceEntityLink,
    CallResolutionEvidence,
}
impl IncomingOwner {
    pub fn name(self) -> &'static str {
        match self {
            Self::ProviderRun => attribution::ProviderRun::NAME,
            Self::ProviderCoverage => attribution::ProviderCoverage::NAME,
            Self::RunFamily => attribution::RunFamily::NAME,
            Self::GuardSubstitution => conditions::stability::GuardSubstitution::NAME,
            Self::TypeBinderAssessment => normalized::links::TypeBinderAssessment::NAME,
            Self::TypeEntityLink => normalized::links::TypeEntityLink::NAME,
            Self::PlaceEntityLink => normalized::links::PlaceEntityLink::NAME,
            Self::CallResolutionEvidence => normalized::events::CallEventResolutionEvidence::NAME,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct IncomingRule {
    pub edge: ServingEdge,
    pub owner: IncomingOwner,
    pub fields: &'static [&'static str],
}
pub const INCOMING_RULES: &[IncomingRule] = &[
    IncomingRule {
        edge: ServingEdge::Reference,
        owner: IncomingOwner::ProviderRun,
        fields: &["input"],
    },
    IncomingRule {
        edge: ServingEdge::Participant,
        owner: IncomingOwner::ProviderCoverage,
        fields: &["scope"],
    },
    IncomingRule {
        edge: ServingEdge::Participant,
        owner: IncomingOwner::RunFamily,
        fields: &["run"],
    },
    IncomingRule {
        edge: ServingEdge::Participant,
        owner: IncomingOwner::GuardSubstitution,
        fields: &["atom", "binding"],
    },
    IncomingRule {
        edge: ServingEdge::Participant,
        owner: IncomingOwner::TypeBinderAssessment,
        fields: &["variable"],
    },
    IncomingRule {
        edge: ServingEdge::Participant,
        owner: IncomingOwner::TypeEntityLink,
        fields: &["term"],
    },
    IncomingRule {
        edge: ServingEdge::Participant,
        owner: IncomingOwner::PlaceEntityLink,
        fields: &["place"],
    },
    IncomingRule {
        edge: ServingEdge::Participant,
        owner: IncomingOwner::CallResolutionEvidence,
        fields: &["resolution"],
    },
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureSource {
    Artifact,
    Member,
    Unit,
}
impl CaptureSource {
    pub fn name(self) -> &'static str {
        match self {
            Self::Artifact => source::SourceArtifact::NAME,
            Self::Member => catalog::CatalogMember::NAME,
            Self::Unit => retrieval::Unit::NAME,
        }
    }
}
/// Corpus companions are discovered even when corpus rows are outside output inventory.
/// Only artifact/unit sources bridge corpus membership to distribution input. A member's input
/// selects its direct distribution and never expands another corpus's libraries.
#[derive(Debug, Clone, Copy)]
pub struct CaptureCompanions {
    pub direct_sources: &'static [CaptureSource],
    pub corpus_sources: &'static [CaptureSource],
    pub source_input: &'static str,
    pub corpus_relation: &'static str,
    pub corpus_input: &'static str,
    pub corpus_library: &'static str,
    pub distribution_relation: &'static str,
    pub distribution_input: &'static str,
}
pub const CAPTURE_COMPANIONS: CaptureCompanions = CaptureCompanions {
    direct_sources: &[
        CaptureSource::Artifact,
        CaptureSource::Member,
        CaptureSource::Unit,
    ],
    corpus_sources: &[CaptureSource::Artifact, CaptureSource::Unit],
    source_input: "input",
    corpus_relation: input::CorpusLibrary::NAME,
    corpus_input: "corpus",
    corpus_library: "library",
    distribution_relation: input::InputDistribution::NAME,
    distribution_input: "input",
};
impl CaptureCompanions {
    pub fn direct_input(self, relation: &str) -> bool {
        self.direct_sources
            .iter()
            .any(|source| source.name() == relation)
    }
    pub fn corpus_input(self, relation: &str) -> bool {
        self.corpus_sources
            .iter()
            .any(|source| source.name() == relation)
    }
    /// Finite companion interpretation, independent of native graph storage.
    pub fn matches_corpus(
        self,
        relation: &str,
        source_input: Id<input::InputRevision>,
        corpus: &input::CorpusLibrary,
    ) -> bool {
        self.corpus_input(relation) && corpus.corpus == source_input
    }
    pub fn matches_distribution(
        self,
        relation: &str,
        source_input: Id<input::InputRevision>,
        corpora: &[input::CorpusLibrary],
        distribution: &input::InputDistribution,
    ) -> bool {
        self.direct_input(relation)
            && (distribution.input == source_input
                || corpora.iter().any(|corpus| {
                    self.matches_corpus(relation, source_input, corpus)
                        && corpus.library == distribution.input
                }))
    }
}
/// Root-independent request shape. Ordered declarations/fields retain their consumer's binding
/// and cardinality policy; the lowerer may derive the existing nominal output set from them.
/// This is lifetime-owned preparation, without result caching or an incomplete program hash.
pub struct ServingScopeProgram {
    inputs: Vec<ValidationInput>,
    incoming: Vec<ValidationInput>,
    fields: Vec<String>,
    _charge: StateCharge,
}
impl ServingScopeProgram {
    pub fn new(
        inputs: &[ValidationInput],
        incoming: &[ValidationInput],
        fields: &[&str],
        budget: &ResourceBudget,
    ) -> Result<Arc<Self>, ModelError> {
        let declarations = inputs
            .iter()
            .chain(incoming)
            .map(|input| {
                512usize
                    .saturating_add(input.name().len())
                    .saturating_add(usize::from(input.prefix().is_some()))
                    .saturating_add(
                        input
                            .order()
                            .iter()
                            .map(|field| field.len() + 64)
                            .sum::<usize>(),
                    )
            })
            .sum::<usize>();
        let mut charge = StateCharge::new(budget, "serving-scope-program");
        charge.grow(
            1024usize
                .saturating_add(declarations)
                .saturating_add(fields.iter().map(|field| field.len() + 64).sum::<usize>()),
        )?;
        Ok(Arc::new(Self {
            inputs: inputs.to_vec(),
            incoming: incoming.to_vec(),
            fields: fields.iter().map(|field| (*field).to_owned()).collect(),
            _charge: charge,
        }))
    }
    pub fn inputs(&self) -> &[ValidationInput] {
        &self.inputs
    }
    pub fn incoming_inputs(&self) -> &[ValidationInput] {
        &self.incoming
    }
    pub fn owned_fields(&self) -> &[String] {
        &self.fields
    }
    pub fn outgoing_edges(&self) -> &[ServingEdge] {
        &[ServingEdge::Reference, ServingEdge::Participant]
    }
    pub fn incoming_rules(&self) -> &[IncomingRule] {
        INCOMING_RULES
    }
    pub fn capture(&self) -> CaptureCompanions {
        CAPTURE_COMPANIONS
    }
    /// Finite interpretation of the same incoming rule declarations used by native lowering.
    pub fn incoming(&self, edge: ServingEdge, relation: &str, field: &str) -> bool {
        self.incoming.iter().any(|input| input.name() == relation)
            && (self.fields.iter().any(|owned| owned == field)
                || self.incoming_rules().iter().any(|rule| {
                    rule.edge == edge
                        && rule.owner.name() == relation
                        && rule.fields.contains(&field)
                }))
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    #[test]
    fn special_incoming_rules_preserve_direction_inventory_and_shared_identity_exclusion() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let inputs = [
            ValidationInput::of::<attribution::ProviderRun>(&["id"]),
            ValidationInput::of::<attribution::ProviderCoverage>(&["id"]),
            ValidationInput::of::<conditions::stability::GuardSubstitution>(&["id"]),
            ValidationInput::of::<normalized::links::TypeEntityLink>(&["id"]),
        ];
        let program = ServingScopeProgram::new(&inputs, &inputs, OWNED_FIELDS, &budget).unwrap();
        assert!(program.incoming(
            ServingEdge::Reference,
            attribution::ProviderRun::NAME,
            "input"
        ));
        assert!(!program.incoming(
            ServingEdge::Participant,
            attribution::ProviderRun::NAME,
            "input"
        ));
        assert!(program.incoming(
            ServingEdge::Participant,
            attribution::ProviderCoverage::NAME,
            "scope"
        ));
        assert!(!program.incoming(
            ServingEdge::Reference,
            attribution::ProviderCoverage::NAME,
            "scope"
        ));
        for field in ["atom", "binding"] {
            assert!(program.incoming(
                ServingEdge::Participant,
                conditions::stability::GuardSubstitution::NAME,
                field
            ));
        }
        assert!(!program.incoming(
            ServingEdge::Participant,
            conditions::stability::GuardSubstitution::NAME,
            "context"
        ));
        assert!(program.incoming(
            ServingEdge::Participant,
            normalized::links::TypeEntityLink::NAME,
            "term"
        ));
        assert!(
            !program.incoming(
                ServingEdge::Participant,
                normalized::links::TypeBinderAssessment::NAME,
                "variable"
            ),
            "special rule cannot widen incoming inventory"
        );
        assert!(!program.incoming(
            ServingEdge::Reference,
            attribution::ProviderRun::NAME,
            "provider"
        ));
        assert!(budget.reserved() > 0);
        drop(program);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn capture_companions_cross_only_the_selected_corpus_even_when_it_is_not_an_output() {
        fn id<R>(byte: u8) -> Id<R> {
            serde_json::from_value(serde_json::to_value([byte; 16]).unwrap()).unwrap()
        }
        let source = id::<input::InputRevision>(1);
        let library = id::<input::InputRevision>(2);
        let unrelated = id::<input::InputRevision>(3);
        let corpora = [
            input::CorpusLibrary {
                corpus: source,
                library,
            },
            input::CorpusLibrary {
                corpus: unrelated,
                library: unrelated,
            },
        ];
        let direct = input::InputDistribution {
            input: source,
            release: id(4),
            role: input::DistributionRole::FirstParty,
        };
        let linked = input::InputDistribution {
            input: library,
            release: id(4),
            role: input::DistributionRole::Dependency,
        };
        let foreign = input::InputDistribution {
            input: unrelated,
            release: id(4),
            role: input::DistributionRole::FirstParty,
        };
        for relation in [source::SourceArtifact::NAME, retrieval::Unit::NAME] {
            assert!(CAPTURE_COMPANIONS.matches_distribution(relation, source, &corpora, &direct));
            assert!(CAPTURE_COMPANIONS.matches_distribution(relation, source, &corpora, &linked));
            assert!(!CAPTURE_COMPANIONS.matches_distribution(relation, source, &corpora, &foreign));
        }
        assert!(CAPTURE_COMPANIONS.matches_distribution(
            catalog::CatalogMember::NAME,
            source,
            &corpora,
            &direct
        ));
        assert!(!CAPTURE_COMPANIONS.matches_distribution(
            catalog::CatalogMember::NAME,
            source,
            &corpora,
            &linked
        ));
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let inventory = [ValidationInput::of::<input::InputDistribution>(&["id"])];
        let program = ServingScopeProgram::new(&inventory, &[], OWNED_FIELDS, &budget).unwrap();
        assert!(
            !program
                .inputs()
                .iter()
                .any(|input| input.name() == input::CorpusLibrary::NAME)
        );
        assert!(program.capture().matches_distribution(
            source::SourceArtifact::NAME,
            source,
            &corpora,
            &linked
        ));
    }
    #[test]
    fn capture_source_rules_keep_direct_and_corpus_companions_distinct() {
        assert!(CAPTURE_COMPANIONS.direct_input(catalog::CatalogMember::NAME));
        assert!(!CAPTURE_COMPANIONS.corpus_input(catalog::CatalogMember::NAME));
        for relation in [source::SourceArtifact::NAME, retrieval::Unit::NAME] {
            assert!(CAPTURE_COMPANIONS.direct_input(relation));
            assert!(CAPTURE_COMPANIONS.corpus_input(relation));
        }
        assert!(!CAPTURE_COMPANIONS.direct_input(attribution::ProviderRun::NAME));
        assert!(!CAPTURE_COMPANIONS.corpus_input(input::InputDistribution::NAME));
        assert_eq!(
            CAPTURE_COMPANIONS.corpus_relation,
            input::CorpusLibrary::NAME
        );
        assert_eq!(
            CAPTURE_COMPANIONS.distribution_relation,
            input::InputDistribution::NAME
        );
    }
}
