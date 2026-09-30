use arrow_array::RecordBatch;
use lctx_model::domain::{
    artifact::*,
    assertion::*,
    attribution::*,
    conditions::{rebase::*, *},
    input::*,
    source::*,
    value::*,
    *,
};
use std::collections::BTreeMap;
pub struct Fixture {
    pub model: ValidatedModel,
    pub batches: BTreeMap<&'static str, RecordBatch>,
    pub origin: EvaluationAtom,
    pub calls: Vec<Occurrence>,
    pub diagram: Diagram,
}
impl Fixture {
    pub fn new(foreign: bool) -> Self {
        let model = model().unwrap();
        let caller_bytes = include_bytes!("../../../../fixtures/python/semantic_guards/caller.py");
        let callee_bytes = include_bytes!("../../../../fixtures/python/semantic_guards/callee.py");
        let caller_entry = ManifestEntry {
            path: "caller.py".into(),
            content: ContentHash::of(caller_bytes),
            byte_len: caller_bytes.len() as i64,
        };
        let callee_entry = ManifestEntry {
            path: "callee.py".into(),
            content: ContentHash::of(callee_bytes),
            byte_len: callee_bytes.len() as i64,
        };
        let input = InputRevision::from_entries(if foreign {
            vec![caller_entry]
        } else {
            vec![caller_entry, callee_entry.clone()]
        })
        .unwrap();
        let callee_input = if foreign {
            InputRevision::from_entries(vec![callee_entry]).unwrap()
        } else {
            input.clone()
        };
        let caller =
            SourceArtifact::from_bytes(input.id(), "caller.py".into(), caller_bytes).unwrap();
        let callee =
            SourceArtifact::from_bytes(callee_input.id(), "callee.py".into(), callee_bytes)
                .unwrap();
        let input_origin = InputOrigin::Tree {
            label: "guard contract".into(),
        };
        let acquisitions = vec![
            InputAcquisition {
                input: input.id(),
                origin: input_origin.id(),
            },
            InputAcquisition {
                input: callee_input.id(),
                origin: input_origin.id(),
            },
        ];
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"cfg"),
            environment_digest: input.manifest,
            lock_digest: None,
        };
        let provider = Provider {
            tool: "guard-contract".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"guard"),
        };
        let (run, families) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            [FactFamily::Syntax],
        )
        .unwrap();
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Syntax,
            name: "contract observation".into(),
        };
        let scope = CoverageScope::Input { input: input.id() };
        let calls: Vec<_> = std::str::from_utf8(caller_bytes)
            .unwrap()
            .match_indices("callee()")
            .enumerate()
            .map(|(ordinal, (start, _))| Occurrence {
                source: caller.id(),
                start: start as i64,
                end: (start + 8) as i64,
                syntax_kind: SyntaxKind::ExprCall,
                role: OccurrenceRole::Syntax,
                structural_path: vec![ordinal as i32],
            })
            .collect();
        let start = std::str::from_utf8(callee_bytes)
            .unwrap()
            .find("local_flag")
            .unwrap() as i64;
        let evaluation = Occurrence {
            source: callee.id(),
            start,
            end: start + 10,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Predicate,
            structural_path: vec![0, 0, 0],
        };
        let predicate = Predicate::Opaque {
            text: "local_flag".into(),
        };
        let origin = EvaluationAtom {
            evaluation: evaluation.id(),
            context: context.id(),
            predicate: predicate.id(),
            operand: None,
        };
        let original = Diagram::from_atom(origin.id());
        let atoms = BTreeMap::from([(origin.id(), origin.clone())]);
        let predicates = BTreeMap::from([(predicate.id(), predicate.clone())]);
        let empty_places = BTreeMap::new();
        let empty_roots = BTreeMap::new();
        let rebased = rebase_local_guards(
            &original,
            &calls[0],
            context.id(),
            &GuardCatalog {
                atoms: &atoms,
                predicates: &predicates,
                places: &empty_places,
                roots: &empty_roots,
            },
        )
        .unwrap();
        let diagram = rebased.condition;
        let (condition, nodes) = diagram.records();
        let q = AssertionQualification {
            context: context.id(),
            scope: scope.id(),
            condition: condition.id(),
            modality: Modality::Candidate,
            approximation: Approximation::Unknown,
        };
        let observation = SyntaxObservation {
            qualification: q.id(),
            occurrence: calls[0].id(),
            spelling: "callee".into(),
        };
        let evidence = Evidence::Occurrence {
            occurrence: calls[0].id(),
        };
        let support = SyntaxSupport {
            assertion: observation.id(),
            run: run.id(),
            surface: surface.id(),
            evidence: evidence.id(),
            origin: Origin::DerivedAnalysis,
            mode: ExtractionMode::RelationalDerivation,
            fidelity: Fidelity::NormalizedStructural,
        };
        let mut f = Self {
            model,
            batches: BTreeMap::new(),
            origin,
            diagram,
            calls,
        };
        macro_rules! one { ($($row:expr),+ $(,)?) => { $(f.put(vec![$row.clone()]);)+ }; }
        one!(
            input_origin,
            context,
            provider,
            run,
            surface,
            scope,
            condition,
            q,
            observation,
            evidence,
            support
        );
        f.put(vec![input, callee_input]);
        f.put(acquisitions);
        f.put(families);
        f.put(nodes);
        let mut atoms = rebased.atoms;
        atoms.push(f.origin.clone());
        f.put(atoms);
        let mut predicates = rebased.predicates;
        predicates.push(predicate);
        f.put(predicates);
        let mut occurrences = f.calls.clone();
        occurrences.push(evaluation);
        f.put(occurrences);
        f.put(
            ArtifactChunk::split(&caller, caller_bytes)
                .unwrap()
                .chain(ArtifactChunk::split(&callee, callee_bytes).unwrap())
                .collect(),
        );
        f.put(vec![caller, callee]);
        f
    }
    pub fn put<R: Record>(&mut self, rows: Vec<R>) {
        self.batches.insert(
            R::NAME,
            Batch::new(&self.model, rows, &budget())
                .unwrap()
                .arrow()
                .clone(),
        );
    }
    pub fn rows<R: Record>(&self) -> Vec<R> {
        self.batches
            .get(R::NAME)
            .map(|b| R::decode(b).unwrap())
            .unwrap_or_default()
    }
    pub fn check(&self, invariant: &Invariant) -> Result<(), ModelError> {
        let mut check = (invariant.create)(&budget());
        for input in &invariant.inputs {
            if let Some(batch) = self.batches.get(input.name()) {
                check.visit(input.name(), batch)?;
            }
        }
        check.finish()
    }
}

impl Fixture {
    pub fn operand_origin(&mut self, kind: &str, nested: bool) {
        let root = match kind {
            "formal" => PlaceRoot::Formal {
                declaration: self.origin.evaluation,
            },
            "receiver" => PlaceRoot::Receiver {
                callable: self.origin.evaluation,
            },
            _ => PlaceRoot::Occurrence {
                occurrence: self.origin.evaluation,
            },
        };
        let path = AccessPath::empty();
        let place = Place {
            root: root.id(),
            path: path.id(),
        };
        self.origin.operand = Some(place.id());
        let original = self
            .rows::<Predicate>()
            .into_iter()
            .find(|p| p.id() == self.origin.predicate)
            .unwrap();
        let mut atoms = vec![self.origin.clone()];
        let mut predicates = vec![original];
        for site in self.calls.iter().take(if nested { 2 } else { 1 }) {
            let predicate = Predicate::InvokedGuard {
                source: atoms.last().unwrap().id(),
            };
            atoms.push(EvaluationAtom {
                evaluation: site.id(),
                context: self.origin.context,
                predicate: predicate.id(),
                operand: None,
            });
            predicates.push(predicate);
        }
        self.diagram = Diagram::from_atom(atoms.last().unwrap().id());
        let (condition, nodes) = self.diagram.records();
        let mut q = self.rows::<AssertionQualification>()[0].clone();
        q.condition = condition.id();
        let mut observation = self.rows::<SyntaxObservation>()[0].clone();
        observation.qualification = q.id();
        let mut support = self.rows::<SyntaxSupport>()[0].clone();
        support.assertion = observation.id();
        self.put(vec![root]);
        self.put(vec![path]);
        self.put(vec![place]);
        self.put(atoms);
        self.put(predicates);
        self.put(vec![condition]);
        self.put(nodes);
        self.put(vec![q]);
        self.put(vec![observation]);
        self.put(vec![support]);
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}
