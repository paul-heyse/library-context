//! Pure exact-request controls over current domain identities and independently replayed Entry.
fn snapshot_for(byte: u8) -> lctx_model::domain::serving::SnapshotHandle {
    use lctx_model::domain::serving::{DatabaseIdentity, Name, SnapshotHandle};
    SnapshotHandle {
        semantic: lctx_model::domain::ContentHash([byte; 32]),
        realization: lctx_model::domain::ContentHash([byte; 32]),
        database: DatabaseIdentity {
            namespace: Name::new("lctx").unwrap(),
            database: Name::new(format!("snapshot_{byte}")).unwrap(),
        },
    }
}
#[path = "fixtures/stability.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    analysis::local,
    assertion::AssertionQualification,
    assertion::ProviderSurface,
    attribution::*,
    calls::*,
    conditions::{entry::*, *},
    flow::*,
    lexical::{LexicalResolution, LexicalResolutionSupport, LexicalTarget, SyntaxField},
    local_theory::{TheoryData, TheoryInventory},
    native_requests::*,
    normalized::{Rows, callables::*, entities::*, links::*},
    obligation::{ObligationKind, Verdict},
    resources::ResourceBudget,
    source::{Occurrence, OccurrenceRole, SyntaxKind},
    syntax::{SyntaxPlacement, SyntaxPlacementSupport},
    types::{TypeObservation, TypeRole, TypeSupport, TypeTerm},
    value::*,
    *,
};

struct Case {
    f: Fixture,
    theory: TheoryInventory,
    invocation: local::AnalysisInvocation,
    effective: EffectiveCallableAssessment,
    variant: SignatureVariant,
    slot: SignatureSlot,
}

#[test]
fn guard_bridge_uses_checked_syntax_read_identity_and_rejects_mismatched_premises() {
    let mut case = Case::new(Predicate::IsNone, vec![], None);
    let read = case
        .f
        .data
        .occurrences
        .get(case.f.request.access)
        .unwrap()
        .clone();
    let syntax = Occurrence {
        role: OccurrenceRole::Syntax,
        ..read.clone()
    };
    case.f.data.occurrences.insert(syntax.clone()).unwrap();
    let mut leaf = case.f.data.leaves.iter().next().unwrap().clone();
    leaf.operand = Some(syntax.id());
    let mut support = case.f.data.leaf_supports.iter().next().unwrap().clone();
    support.assertion = leaf.id();
    case.f.data.leaves = Rows::new(&case.f.budget);
    case.f.data.leaves.insert(leaf.clone()).unwrap();
    case.f.data.leaf_supports = Rows::new(&case.f.budget);
    case.f.data.leaf_supports.insert(support.clone()).unwrap();
    let positive = case.run(
        &str_("ready"),
        true,
        Limits::default(),
        complete(),
        &case.f.budget,
    );
    assert_eq!(positive.exact, ExactOutcome::RefutedPathUnderModel);
    assert!(positive.proof.contains(&derivation::RowRef::of(read.id())));
    assert!(positive.proof.contains(&derivation::RowRef::of(leaf.id())));
    let mismatched = Occurrence {
        structural_path: vec![999],
        ..syntax
    };
    case.f.data.occurrences.insert(mismatched.clone()).unwrap();
    leaf.operand = Some(mismatched.id());
    support.assertion = leaf.id();
    case.f.data.leaves = Rows::new(&case.f.budget);
    case.f.data.leaves.insert(leaf.clone()).unwrap();
    case.f.data.leaf_supports = Rows::new(&case.f.budget);
    case.f.data.leaf_supports.insert(support.clone()).unwrap();
    assert!(
        case.f.guard_entry().is_err(),
        "same span cannot replace the checked structural path"
    );
    leaf.operand = Some(read.id());
    case.f.guard.operand = None;
    case.f.data.atoms = Rows::new(&case.f.budget);
    case.f.data.atoms.insert(case.f.guard.clone()).unwrap();
    leaf.atom = case.f.guard.id();
    support.assertion = leaf.id();
    case.f.data.leaves = Rows::new(&case.f.budget);
    case.f.data.leaves.insert(leaf).unwrap();
    case.f.data.leaf_supports = Rows::new(&case.f.budget);
    case.f.data.leaf_supports.insert(support).unwrap();
    assert!(
        case.f.guard_entry().is_err(),
        "missing atom operand cannot be repaired by the input scalar"
    );
}

#[test]
fn guard_path_retains_its_diagram_and_releases_whole_inventory_decode_charge() {
    let mut case = Case::new(Predicate::IsNone, vec![], None);
    for n in 0..64 {
        let predicate = Predicate::Equals {
            value: Literal::Integer {
                decimal: n.to_string(),
            }
            .id(),
        };
        let atom = EvaluationAtom {
            predicate: predicate.id(),
            ..case.f.guard.clone()
        };
        let (condition, nodes) = Diagram::from_atom(atom.id()).records();
        case.f.data.conditions.insert(condition).unwrap();
        for node in nodes {
            case.f.data.condition_nodes.insert(node).unwrap();
        }
    }
    let entry = case.f.guard_entry().unwrap();
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    {
        let path = NativePath::guard(&entry, &case.f.data, &budget).unwrap();
        assert_eq!(
            path.original_condition(),
            case.f
                .data
                .qualifications
                .get(case.f.data.leaves.iter().next().unwrap().qualification)
                .unwrap()
                .condition
        );
        assert!(budget.reserved() > 0);
        assert!(
            budget.reserved() < case.f.data.condition_nodes.len() * 2048,
            "temporary global decoder is not retained for each path"
        );
    }
    assert_eq!(
        budget.reserved(),
        0,
        "path-owned allocation is released with the final path"
    );
}

#[test]
fn canonical_unexamined_path_keeps_original_frame_without_admitting_default_scalar() {
    let mut case = Case::new(Predicate::IsNone, vec![], None);
    case.slot.default = DefaultSlot::DefinitionTime;
    let entry = case.f.guard_entry().unwrap();
    assert_eq!(
        case.context(&entry).err(),
        Some(ObligationKind::DefaultStabilityUnknown)
    );
    let path = NativePath::guard(&entry, &case.f.data, &case.f.budget).unwrap();
    let value = str_("ready");
    let mut request = ExactRequest {
        snapshot: snapshot_for(1),
        owner: case.f.request.owner,
        formal: case.f.request.formal,
        value: &value,
        assumptions: Assumptions::default(),
    };
    let result = unexamined(
        &request,
        &entry,
        ObligationKind::DefaultStabilityUnknown,
        &path,
        &case.f.budget,
    )
    .unwrap();
    assert_eq!(result.verdict, Verdict::Unknown);
    assert_eq!(result.exact, ExactOutcome::Unknown);
    assert_eq!(result.original_condition, path.original_condition());
    assert_eq!(result.path, path.identity());
    assert_eq!(result.reason, Some(ObligationKind::DefaultStabilityUnknown));
    assert!(result.restricted_result.is_none());
    assert_eq!(result.work, Work::default());
    case.assert_proof_resolves(&result);
    request.formal = ParameterEntity::NativeSlot {
        callable: case.effective.callable,
        signature: case.variant.signature,
        parameter: case.slot.parameter,
    }
    .id();
    assert!(
        unexamined(
            &request,
            &entry,
            ObligationKind::DefaultStabilityUnknown,
            &path,
            &case.f.budget
        )
        .is_err(),
        "the unknown boundary does not admit a foreign formal"
    );
}
impl Case {
    fn new(
        predicate: Predicate,
        literals: Vec<Literal>,
        members: Option<(LiteralSet, Vec<LiteralSetMember>)>,
    ) -> Self {
        let mut f = Fixture::new();
        let old_leaf = f.data.leaves.iter().next().unwrap().clone();
        let mut support = f.data.leaf_supports.iter().next().unwrap().clone();
        f.guard.predicate = predicate.id();
        f.data.atoms = Rows::new(&f.budget);
        f.data.atoms.insert(f.guard.clone()).unwrap();
        f.data.predicates = Rows::new(&f.budget);
        f.data.predicates.insert(predicate).unwrap();
        let diagram = Diagram::from_atom(f.guard.id());
        let (condition, nodes) = diagram.records();
        f.data.conditions.insert(condition).unwrap();
        for node in nodes {
            f.data.condition_nodes.insert(node).unwrap();
        }
        let q = AssertionQualification {
            condition: diagram.id(),
            ..f.q.clone()
        };
        f.data.qualifications.insert(q.clone()).unwrap();
        let leaf = FlowTestLeafObservation {
            qualification: q.id(),
            atom: f.guard.id(),
            ..old_leaf
        };
        support.assertion = leaf.id();
        f.data.leaves = Rows::new(&f.budget);
        f.data.leaves.insert(leaf).unwrap();
        f.data.leaf_supports = Rows::new(&f.budget);
        f.data.leaf_supports.insert(support).unwrap();
        let mut theory = TheoryInventory::new(&f.budget);
        for literal in literals {
            theory.literals.insert(literal).unwrap();
        }
        if let Some((set, members)) = members {
            theory.sets.insert(set).unwrap();
            for member in members {
                theory.set_members.insert(member).unwrap();
            }
        }
        let entry = f.guard_entry().unwrap();
        let EntityRef::Callable { callable } = f.data.refs.get(f.request.owner).unwrap() else {
            unreachable!()
        };
        let effective = EffectiveCallableAssessment {
            callable: *callable,
            context: f.request.context,
            decorators: KeySink::new("effective-decorator-chain").finish(),
            policy: ContentHash::of(b"current normalized policy"),
            identity: Knowledge::Known,
            identity_reason: CallableReason::EvidenceAgreement,
            signatures: Knowledge::Known,
            signature_reason: CallableReason::EvidenceAgreement,
            descriptor: Knowledge::Known,
            descriptor_kind: Some(DescriptorKind::Function),
            descriptor_reason: CallableReason::EvidenceAgreement,
            body: Knowledge::Known,
            body_admitted: true,
            body_reason: CallableReason::EvidenceAgreement,
            asynchronous: Some(false),
            generator: Some(false),
        };
        let signature = f.data.parameters.get(entry.parameter()).unwrap().signature;
        let resolution = SymbolEntityResolution {
            symbol: f.data.signatures.get(signature).unwrap().symbol,
            context: f.request.context,
            policy: ContentHash::of(b"current entity policy"),
            status: ResolutionStatus::Resolved,
            entity: Some(f.request.owner),
            reason: EntityReason::DeclarationAgreement,
        };
        let variant = SignatureVariant {
            role: lctx_model::domain::calls::SignatureRole::Source,
            native: None,
            signature,
            context: f.request.context,
            resolution: resolution.id(),
            callable: Some(*callable),
            assessment: Some(effective.id()),
            adjustment: SignatureAdjustment::None,
        };
        let slot = SignatureSlot {
            parameter: entry.parameter(),
            variant: variant.id(),
            ordinal: 0,
            default: DefaultSlot::Required,
        };
        let run = f.data.runs.get(f.request.run).unwrap();
        let invocation = local::AnalysisInvocation::new(
            run.input,
            run.context,
            lctx_model::domain::local_semantics::definition().1.id(),
            Some(f.request.owner),
            [],
        )
        .0;
        Self {
            f,
            theory,
            invocation,
            effective,
            variant,
            slot,
        }
    }
    fn context(&self, entry: &DerivedEntryValue) -> Result<NativeContext, ObligationKind> {
        NativeContext::from_entry(
            entry,
            &self.f.data,
            &self.effective,
            &self.variant,
            &self.slot,
            &Rows::new(&self.f.budget),
        )
    }
    fn run(
        &self,
        value: &ExactScalar,
        bridge: bool,
        limits: Limits,
        coverage: CoverageStatus,
        budget: &ResourceBudget,
    ) -> Assessment {
        self.run_assumed(
            value,
            bridge,
            limits,
            coverage,
            budget,
            Assumptions::default(),
        )
    }
    fn run_assumed(
        &self,
        value: &ExactScalar,
        bridge: bool,
        limits: Limits,
        coverage: CoverageStatus,
        budget: &ResourceBudget,
        assumptions: Assumptions,
    ) -> Assessment {
        let entry = self.f.guard_entry().unwrap();
        let context = self.context(&entry).unwrap();
        let path = NativePath::guard(&entry, &self.f.data, &self.f.budget).unwrap();
        let mut atoms = vec![];
        if bridge
            && let Ok(atom) = CheckedAtom::derive(
                &entry,
                &TheoryData {
                    entry: &self.f.data,
                    inventory: &self.theory,
                },
                &self.invocation,
                &self.f.budget,
            )
            .unwrap()
        {
            atoms.push(atom);
        }
        let result = assess(
            &ExactRequest {
                snapshot: snapshot_for(1),
                owner: self.f.request.owner,
                formal: self.f.request.formal,
                value,
                assumptions,
            },
            &context,
            &path,
            &atoms,
            coverage,
            &[],
            limits,
            budget,
        )
        .unwrap();
        self.assert_proof_resolves(&result);
        result
    }
    fn assert_proof_resolves(&self, result: &Assessment) {
        // Independently enumerate actual input records. Replayed request-only witnesses are
        // deliberately absent; the production proof cannot invent their stored identity.
        let mut actual = std::collections::BTreeSet::new();
        macro_rules! entry_rows { ($($field:ident:$ty:ty,)*) => { $(
            for row in self.f.data.$field.iter() { actual.insert(derivation::RowRef::of(row.id())); }
        )* }; }
        macro_rules! theory_rows { ($($field:ident:$ty:ty,)*) => { $(
            for row in self.theory.$field.iter() { actual.insert(derivation::RowRef::of(row.id())); }
        )* }; }
        lctx_model::entry_value_inputs!(entry_rows);
        lctx_model::local_theory_inputs!(theory_rows);
        actual.extend([
            derivation::RowRef::of(self.effective.id()),
            derivation::RowRef::of(self.variant.id()),
            derivation::RowRef::of(self.slot.id()),
        ]);
        for proof in &result.proof {
            assert!(
                actual.contains(proof),
                "proof row is absent from actual input: {proof:?}"
            );
        }
    }
    fn builtin(class: &str) -> Self {
        let mut case = Self::new(
            Predicate::TypeIs {
                class_expression: class.into(),
            },
            vec![],
            None,
        );
        let run = case.f.data.runs.get(case.f.request.run).unwrap().clone();
        let base = case.f.data.use_supports.iter().next().unwrap().clone();
        let mut surfaces = std::collections::BTreeMap::new();
        for family in [FactFamily::Syntax, FactFamily::Lexical, FactFamily::Types] {
            let surface = ProviderSurface {
                provider: run.provider,
                family,
                name: format!("native {family:?}"),
            };
            surfaces.insert(family as i16, surface.id());
            case.f.data.surfaces.insert(surface).unwrap();
            let coverage = ProviderCoverage {
                family,
                ..case.f.coverage.clone()
            };
            case.f.data.coverage.insert(coverage).unwrap();
        }
        let compare = case
            .f
            .data
            .occurrences
            .get(case.f.guard.evaluation)
            .unwrap()
            .clone();
        let occurrence = |start, end, kind, role, path: Vec<i32>| Occurrence {
            source: compare.source,
            start,
            end,
            syntax_kind: kind,
            role,
            structural_path: path,
        };
        let call_site = occurrence(
            21,
            30,
            SyntaxKind::ExprCall,
            OccurrenceRole::Syntax,
            vec![0, 0, 1, 0, 2],
        );
        let callee = occurrence(
            21,
            25,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            vec![0, 0, 1, 0, 2, 0],
        );
        let class_read = occurrence(
            31,
            34,
            SyntaxKind::ExprName,
            OccurrenceRole::Read,
            vec![0, 0, 1, 0, 3],
        );
        for occurrence in [&call_site, &callee, &class_read] {
            case.f.data.occurrences.insert(occurrence.clone()).unwrap();
        }
        for (occurrence, field) in [
            (&call_site, SyntaxField::Left),
            (&class_read, SyntaxField::Right),
        ] {
            let p = SyntaxPlacement {
                qualification: case.f.q.id(),
                occurrence: occurrence.id(),
                parent: Some(compare.id()),
                field,
                ordinal: 0,
            };
            let s = SyntaxPlacementSupport {
                assertion: p.id(),
                run: run.id(),
                surface: surfaces[&(FactFamily::Syntax as i16)],
                evidence: base.evidence,
                origin: Origin::SourceObservation,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            };
            case.f.data.placements.insert(p).unwrap();
            case.f.data.placement_supports.insert(s).unwrap();
        }
        let (call, arguments) = CallSyntax::new(
            case.f.q.id(),
            call_site.id(),
            callee.id(),
            false,
            &[Actual {
                occurrence: case.f.request.access,
                kind: ArgumentKind::Positional,
                keyword: None,
            }],
        )
        .unwrap();
        let support = CallSyntaxSupport {
            assertion: call.id(),
            run: run.id(),
            surface: surfaces[&(FactFamily::Syntax as i16)],
            evidence: base.evidence,
            origin: Origin::SourceObservation,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        };
        case.theory.call_syntax.insert(call).unwrap();
        case.theory.call_syntax_supports.insert(support).unwrap();
        for argument in arguments {
            case.theory.arguments.insert(argument).unwrap();
        }
        for (read, name) in [(&callee, "type"), (&class_read, class)] {
            let target = LexicalTarget::Builtin {
                name: name.into(),
                variable: false,
            };
            let resolution = LexicalResolution {
                qualification: case.f.q.id(),
                read: read.id(),
                target: target.id(),
                captured: false,
            };
            let support = LexicalResolutionSupport {
                assertion: resolution.id(),
                run: run.id(),
                surface: surfaces[&(FactFamily::Lexical as i16)],
                evidence: base.evidence,
                origin: Origin::DerivedAnalysis,
                mode: ExtractionMode::Recognizer,
                fidelity: Fidelity::NormalizedStructural,
            };
            case.theory.lexical_targets.insert(target).unwrap();
            case.theory.resolutions.insert(resolution).unwrap();
            case.theory.resolution_supports.insert(support).unwrap();
        }
        let module = ProviderModule::Bundled {
            provider: run.provider,
            bundle: ModuleBundle::Typeshed,
            name: "builtins".into(),
        };
        let class_symbol = ProviderSymbol {
            provider: run.provider,
            context: run.context,
            module: module.id(),
            native_key: class.into(),
            name: class.into(),
            kind: SymbolKind::Class,
        };
        let class_term = TypeTerm::ClassObject {
            class: class_symbol.id(),
        };
        case.theory.provider_modules.insert(module).unwrap();
        case.f.data.symbols.insert(class_symbol).unwrap();
        case.theory.terms.insert(class_term.clone()).unwrap();
        let class_observation = TypeObservation {
            qualification: case.f.q.id(),
            subject: class_read.id(),
            role: TypeRole::TestOperand,
            declared: false,
            term: class_term.id(),
        };
        let class_support = TypeSupport {
            assertion: class_observation.id(),
            run: run.id(),
            surface: surfaces[&(FactFamily::Types as i16)],
            evidence: base.evidence,
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        };
        case.theory
            .type_observations
            .insert(class_observation)
            .unwrap();
        case.theory.type_supports.insert(class_support).unwrap();
        let operand_term = TypeTerm::None;
        let operand_observation = TypeObservation {
            qualification: case.f.q.id(),
            subject: case.f.request.access,
            role: TypeRole::TestOperand,
            declared: false,
            term: operand_term.id(),
        };
        case.theory.terms.insert(operand_term).unwrap();
        case.theory
            .type_observations
            .insert(operand_observation.clone())
            .unwrap();
        let leaf = case.f.data.leaves.iter().next().unwrap();
        let assessment = TestOperandTypeAssessment {
            leaf: leaf.id(),
            status: ResolutionStatus::Resolved,
            reason: LinkReason::ExplicitIdentity,
        };
        let link = TestOperandTypeLink {
            assessment: assessment.id(),
            observation: operand_observation.id(),
        };
        case.theory.operand_assessments.insert(assessment).unwrap();
        case.theory.operand_links.insert(link).unwrap();
        case
    }
}
fn str_(s: &str) -> ExactScalar {
    ExactScalar::String { value: s.into() }
}
fn int(s: &str) -> ExactScalar {
    ExactScalar::Integer { decimal: s.into() }
}
fn string(s: &str) -> Literal {
    Literal::String { value: s.into() }
}
fn complete() -> CoverageStatus {
    CoverageStatus::CompleteUnderStatedModel
}

#[test]
fn exact_none_path_refutation_preserves_original_and_may_compatibility() {
    let case = Case::new(Predicate::IsNone, vec![], None);
    let positive = case.run(
        &ExactScalar::None {},
        true,
        Limits::default(),
        complete(),
        &case.f.budget,
    );
    let negative = case.run(
        &int("1"),
        true,
        Limits::default(),
        complete(),
        &case.f.budget,
    );
    assert_eq!(positive.exact, ExactOutcome::CompatibleUnderMayModel);
    assert_eq!(
        positive.verdict,
        Verdict::Conditional,
        "a restricted true diagram does not turn a conditional path into execution"
    );
    assert_eq!(negative.exact, ExactOutcome::RefutedPathUnderModel);
    assert_eq!(negative.verdict, Verdict::RefutedUnderModel);
    assert_eq!(positive.original_condition, negative.original_condition);
    assert_ne!(positive.restricted_result, negative.restricted_result);
    assert_eq!(negative.work.assignments_applied, 1);
    assert!(
        negative
            .proof
            .iter()
            .any(|p| p.relation() == FlowTestLeafSupport::NAME)
    );
    assert!(
        negative
            .proof
            .iter()
            .any(|p| p.relation() == FlowReachingSupport::NAME)
    );
    assert!(
        !negative
            .proof
            .iter()
            .any(|p| p.relation() == EntryValueWitness::NAME),
        "request-only Entry replay cites existing premises instead of an unpublished witness"
    );
}

#[test]
fn finite_two_atom_refutation_keeps_valid_proof_when_minimization_budget_ends() {
    let mut case = Case::new(Predicate::IsNone, vec![], None);
    let second = Case::new(Predicate::Truthy, vec![], None);
    let diagram = Diagram::from_atom(case.f.guard.id())
        .admitted_binary(
            &Diagram::from_atom(second.f.guard.id()),
            BooleanOperation::Conjunction,
            &case.f.budget,
        )
        .unwrap();
    let (condition, nodes) = diagram.records();
    case.f.data.conditions.insert(condition).unwrap();
    for node in nodes {
        case.f.data.condition_nodes.insert(node).unwrap();
    }
    let q = AssertionQualification {
        condition: diagram.id(),
        ..case.f.q.clone()
    };
    case.f.data.qualifications.insert(q.clone()).unwrap();
    case.f.data.atoms.insert(second.f.guard.clone()).unwrap();
    case.f.data.predicates.insert(Predicate::Truthy).unwrap();
    let mut leaves = vec![];
    for source in [&case.f, &second.f] {
        let mut leaf = source.data.leaves.iter().next().unwrap().clone();
        let mut support = source.data.leaf_supports.iter().next().unwrap().clone();
        leaf.qualification = q.id();
        support.assertion = leaf.id();
        leaves.push((leaf, support));
    }
    case.f.data.leaves = Rows::new(&case.f.budget);
    case.f.data.leaf_supports = Rows::new(&case.f.budget);
    for (leaf, support) in &leaves {
        case.f.data.leaves.insert(leaf.clone()).unwrap();
        case.f.data.leaf_supports.insert(support.clone()).unwrap();
    }
    let mut atoms = vec![];
    let mut entries = vec![];
    for (leaf, support) in &leaves {
        let source =
            EntryAccessSource::guard(&case.f.data, case.f.request, leaf.id(), support.id())
                .unwrap();
        let entry =
            EntryValueWitness::derive_for(&case.f.data, case.f.request, &source, &case.f.budget)
                .unwrap()
                .unwrap();
        atoms.push(
            CheckedAtom::derive(
                &entry,
                &TheoryData {
                    entry: &case.f.data,
                    inventory: &case.theory,
                },
                &case.invocation,
                &case.f.budget,
            )
            .unwrap()
            .unwrap(),
        );
        entries.push(entry);
    }
    let context = case.context(&entries[0]).unwrap();
    let path = NativePath::guard(&entries[0], &case.f.data, &case.f.budget).unwrap();
    let value = int("1");
    let request = ExactRequest {
        snapshot: snapshot_for(1),
        owner: case.f.request.owner,
        formal: case.f.request.formal,
        value: &value,
        assumptions: Assumptions::default(),
    };
    let minimal = assess(
        &request,
        &context,
        &path,
        &atoms,
        complete(),
        &[],
        Limits::default(),
        &case.f.budget,
    )
    .unwrap();
    let bounded = assess(
        &request,
        &context,
        &path,
        &atoms,
        complete(),
        &[],
        Limits {
            bdd_pairs: diagram.node_count() * 2,
            ..Limits::default()
        },
        &case.f.budget,
    )
    .unwrap();
    for result in [&minimal, &bounded] {
        assert_eq!(result.verdict, Verdict::RefutedUnderModel);
        assert_eq!(result.work.assignments_applied, 2);
        assert_eq!(result.unexamined, 0);
        assert!(result.restricted_result.is_some());
        case.assert_proof_resolves(result);
    }
    assert!(!minimal.proof_truncated);
    assert!(bounded.proof_truncated);
    assert!(minimal.proof.len() < bounded.proof.len());
}

#[test]
fn retained_string_membership_and_non_bool_integer_equality() {
    let literals = vec![string("http"), string("sse")];
    let (set, members) = LiteralSet::of(literals.iter().map(Record::id));
    let case = Case::new(
        Predicate::MemberOf { values: set.id() },
        literals,
        Some((set, members)),
    );
    for (value, expected) in [
        (str_("http"), ExactOutcome::CompatibleUnderMayModel),
        (str_("stdio"), ExactOutcome::RefutedPathUnderModel),
        (int("1"), ExactOutcome::Unknown),
        (ExactScalar::Bool { value: true }, ExactOutcome::Unknown),
    ] {
        assert_eq!(
            case.run(&value, true, Limits::default(), complete(), &case.f.budget)
                .exact,
            expected
        );
    }
    let literal = Literal::Integer {
        decimal: "2".into(),
    };
    let case = Case::new(
        Predicate::Equals {
            value: literal.id(),
        },
        vec![literal],
        None,
    );
    assert_eq!(
        case.run(
            &int("2"),
            true,
            Limits::default(),
            complete(),
            &case.f.budget
        )
        .exact,
        ExactOutcome::CompatibleUnderMayModel
    );
    assert_eq!(
        case.run(
            &int("3"),
            true,
            Limits::default(),
            complete(),
            &case.f.budget
        )
        .exact,
        ExactOutcome::RefutedPathUnderModel
    );
    assert_eq!(
        case.run(
            &ExactScalar::Bool { value: true },
            true,
            Limits::default(),
            complete(),
            &case.f.budget
        )
        .exact,
        ExactOutcome::Unknown
    );
}

#[test]
fn current_ids_match_independent_recorded_cpython_scalar_controls() {
    // Retain the independent CPython receipt, changing only its current-domain lowering.
    let observations = include_str!(
        "../../../docs/design_review/evidence/2026-09-26_primitive-python-lowering/observations.jsonl"
    );
    for line in observations.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let value = match &row["value"] {
            serde_json::Value::Null => ExactScalar::None {},
            serde_json::Value::Bool(value) => ExactScalar::Bool { value: *value },
            serde_json::Value::Number(value) => int(&value.to_string()),
            serde_json::Value::String(value) => str_(value),
            _ => unreachable!(),
        };
        let (predicate, literals, members) = match row["kind"].as_str().unwrap() {
            "str_member" | "int_member" => {
                let literals = if row["kind"] == "str_member" {
                    vec![string("http"), string("sse")]
                } else {
                    vec![Literal::Integer {
                        decimal: "1".into(),
                    }]
                };
                let (set, members) = LiteralSet::of(literals.iter().map(Record::id));
                (
                    Predicate::MemberOf { values: set.id() },
                    literals,
                    Some((set, members)),
                )
            }
            "int_equal" | "equal_one" | "str_equal" => {
                let literal = match row["kind"].as_str().unwrap() {
                    "int_equal" => Literal::Integer {
                        decimal: "2".into(),
                    },
                    "equal_one" => Literal::Integer {
                        decimal: "1".into(),
                    },
                    _ => string("http"),
                };
                (
                    Predicate::Equals {
                        value: literal.id(),
                    },
                    vec![literal],
                    None,
                )
            }
            "truthy" => (Predicate::Truthy, vec![], None),
            "is_none" => (Predicate::IsNone, vec![], None),
            "type_is_str" => continue, // Exercised with actual checked native builtin syntax below.
            _ => unreachable!(),
        };
        let case = Case::new(predicate, literals, members);
        let result = case.run(&value, true, Limits::default(), complete(), &case.f.budget);
        let expected = if !row["lowered"].as_bool().unwrap() {
            ExactOutcome::Unknown
        } else if row["python"].as_bool().unwrap() {
            ExactOutcome::CompatibleUnderMayModel
        } else {
            ExactOutcome::RefutedPathUnderModel
        };
        assert_eq!(result.exact, expected, "oracle {line}");
    }
}

#[test]
fn absent_unsupported_and_computed_bridges_remain_unknown() {
    let case = Case::new(Predicate::IsNone, vec![], None);
    let missing = case.run(
        &int("1"),
        false,
        Limits::default(),
        complete(),
        &case.f.budget,
    );
    assert_eq!(missing.exact, ExactOutcome::Unknown);
    assert!(missing.proof.contains(&derivation::RowRef::of(
        case.f.data.leaves.iter().next().unwrap().id()
    )));
    assert!(
        missing
            .proof
            .contains(&derivation::RowRef::of(case.effective.id()))
    );
    assert!(
        missing
            .proof
            .contains(&derivation::RowRef::of(case.slot.id()))
    );
    assert!(
        !missing.proof.contains(&derivation::RowRef::of(
            case.f.guard_entry().unwrap().witness().id()
        )),
        "unpublished replay witness is not original stored proof"
    );
    assert_eq!(missing.work.assignments_applied, 0);
    assert!(missing.restricted_result.is_none());
    for predicate in [
        Predicate::Opaque {
            text: "x + 1 == 2".into(),
        },
        Predicate::TypeIs {
            class_expression: "int".into(),
        },
        Predicate::IsInstance {
            class_expression: "int".into(),
        },
    ] {
        let case = Case::new(predicate, vec![], None);
        assert_eq!(
            case.run(
                &int("1"),
                true,
                Limits::default(),
                complete(),
                &case.f.budget
            )
            .exact,
            ExactOutcome::Unknown
        );
    }
}

#[test]
fn scope_and_budget_refusals_keep_work_without_refutation() {
    let case = Case::new(Predicate::IsNone, vec![], None);
    for limits in [
        Limits {
            atoms: 0,
            ..Limits::default()
        },
        Limits {
            assignments: 0,
            ..Limits::default()
        },
        Limits {
            bdd_pairs: 0,
            ..Limits::default()
        },
    ] {
        let result = case.run(&int("1"), true, limits, complete(), &case.f.budget);
        assert_eq!(result.verdict, Verdict::Unknown);
        assert!(result.reason.is_some());
        assert!(result.restricted_result.is_none());
    }
    let small = ResourceBudget::fixed(1).unwrap();
    let result = case.run(&int("1"), true, Limits::default(), complete(), &small);
    assert_eq!(result.reason, Some(ObligationKind::ResourceRefused));
    assert!(result.proof_truncated && result.proof.is_empty());
    let result = case.run(
        &int("1"),
        true,
        Limits::default(),
        CoverageStatus::NotRequested,
        &case.f.budget,
    );
    assert_eq!(result.verdict, Verdict::NotAnalyzed);
    assert_eq!(result.exact, ExactOutcome::NotAnalyzed);
    assert!(result.proof.contains(&derivation::RowRef::of(
        case.f.data.leaves.iter().next().unwrap().id()
    )));
    let result = case.run(
        &ExactScalar::None {},
        true,
        Limits {
            render_terms: 0,
            ..Limits::default()
        },
        complete(),
        &case.f.budget,
    );
    assert_eq!(result.exact, ExactOutcome::CompatibleUnderMayModel);
    assert!(result.presentation_truncated);
}

#[test]
fn unsupported_effective_targets_and_defaults_refuse_context() {
    for mode in 0..4 {
        let mut case = Case::new(Predicate::IsNone, vec![], None);
        match mode {
            0 => case.effective.identity = Knowledge::Unknown,
            1 => case.effective.decorators = ContentHash::of(b"decorated target"),
            2 => {
                case.effective.body = Knowledge::Unknown;
                case.effective.body_admitted = false;
            }
            _ => case.slot.default = DefaultSlot::DefinitionTime,
        }
        let entry = case.f.guard_entry().unwrap();
        assert!(case.context(&entry).is_err(), "mode {mode}");
    }
}

#[test]
fn arbitrary_precision_and_closed_scalar_wire_preserve_bool_int_identity() {
    let huge = int("9999999999999999999999999999999999999999999999999999999999999999");
    huge.validate().unwrap();
    for decimal in ["01", "-0", "+1", "1.0", ""] {
        assert!(int(decimal).validate().is_err());
    }
    assert!(
        serde_json::from_str::<ExactScalar>(r#"{"kind":"bool","value":true,"extra":1}"#).is_err()
    );
    assert!(serde_json::from_str::<ExactScalar>(r#"{"kind":"none","value":true}"#).is_err());
    assert_eq!(
        serde_json::from_str::<ExactScalar>(r#"{"kind":"none"}"#).unwrap(),
        ExactScalar::None {}
    );
    assert_ne!(
        serde_json::to_value(ExactScalar::Bool { value: true }).unwrap(),
        serde_json::to_value(int("1")).unwrap()
    );
}

#[test]
fn checked_builtin_type_identity_requires_namespace_and_native_operand_bridge() {
    let standard = Assumptions {
        builtin_namespace: BuiltinNamespace::StandardCpython,
    };
    for (class, yes, no) in [
        ("str", str_("x"), int("1")),
        ("int", int("1"), ExactScalar::Bool { value: true }),
        ("bool", ExactScalar::Bool { value: true }, int("1")),
    ] {
        let case = Case::builtin(class);
        let entry = case.f.guard_entry().unwrap();
        CheckedAtom::derive(
            &entry,
            &TheoryData {
                entry: &case.f.data,
                inventory: &case.theory,
            },
            &case.invocation,
            &case.f.budget,
        )
        .unwrap()
        .expect("exact builtin syntax and resolution should admit the bridge");
        let positive = case.run_assumed(
            &yes,
            true,
            Limits::default(),
            complete(),
            &case.f.budget,
            standard,
        );
        assert_eq!(positive.exact, ExactOutcome::CompatibleUnderMayModel);
        assert!(
            positive
                .proof
                .iter()
                .any(|row| row.relation() == TestOperandTypeLink::NAME)
        );
        assert_eq!(
            case.run_assumed(
                &no,
                true,
                Limits::default(),
                complete(),
                &case.f.budget,
                standard
            )
            .exact,
            ExactOutcome::RefutedPathUnderModel
        );
        assert_eq!(
            case.run(&yes, true, Limits::default(), complete(), &case.f.budget)
                .exact,
            ExactOutcome::Unknown
        );
        let mut shadowed = Case::builtin(class);
        let mut resolutions = shadowed
            .theory
            .resolutions
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let callee = shadowed.theory.call_syntax.iter().next().unwrap().callee;
        let resolution = resolutions.iter_mut().find(|r| r.read == callee).unwrap();
        let original = resolution.id();
        resolution.captured = true;
        let replacement = resolution.id();
        let mut supports = shadowed
            .theory
            .resolution_supports
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        supports
            .iter_mut()
            .find(|s| s.assertion == original)
            .unwrap()
            .assertion = replacement;
        shadowed.theory.resolutions = Rows::new(&shadowed.f.budget);
        for r in resolutions {
            shadowed.theory.resolutions.insert(r).unwrap();
        }
        shadowed.theory.resolution_supports = Rows::new(&shadowed.f.budget);
        for s in supports {
            shadowed.theory.resolution_supports.insert(s).unwrap();
        }
        assert_eq!(
            shadowed
                .run_assumed(
                    &yes,
                    true,
                    Limits::default(),
                    complete(),
                    &shadowed.f.budget,
                    standard
                )
                .exact,
            ExactOutcome::Unknown
        );
        let mut absent = Case::builtin(class);
        absent.theory.operand_links = Rows::new(&absent.f.budget);
        assert_eq!(
            absent
                .run_assumed(
                    &no,
                    true,
                    Limits::default(),
                    complete(),
                    &absent.f.budget,
                    standard
                )
                .exact,
            ExactOutcome::Unknown
        );
    }
}

#[test]
fn snapshot_assumption_and_path_identity_are_independent_of_display_limits() {
    let case = Case::new(Predicate::IsNone, vec![], None);
    let entry = case.f.guard_entry().unwrap();
    let context = case.context(&entry).unwrap();
    let path = NativePath::guard(&entry, &case.f.data, &case.f.budget).unwrap();
    let atom = CheckedAtom::derive(
        &entry,
        &TheoryData {
            entry: &case.f.data,
            inventory: &case.theory,
        },
        &case.invocation,
        &case.f.budget,
    )
    .unwrap()
    .unwrap();
    let value = ExactScalar::None {};
    let mut request = ExactRequest {
        snapshot: snapshot_for(1),
        owner: context.owner(),
        formal: context.formal(),
        value: &value,
        assumptions: Assumptions::default(),
    };
    let run = |request: &ExactRequest<'_>, limits| {
        assess(
            request,
            &context,
            &path,
            std::slice::from_ref(&atom),
            complete(),
            &[],
            limits,
            &case.f.budget,
        )
        .unwrap()
    };
    let first = run(&request, Limits::default());
    let display = run(
        &request,
        Limits {
            render_terms: 0,
            ..Limits::default()
        },
    );
    assert_eq!(first.restricted_result, display.restricted_result);
    assert!(!first.presentation_truncated && display.presentation_truncated);
    request.snapshot = snapshot_for(2);
    assert_ne!(
        first.restricted_result,
        run(&request, Limits::default()).restricted_result
    );
    request.snapshot = snapshot_for(1);
    request.assumptions.builtin_namespace = BuiltinNamespace::StandardCpython;
    assert_ne!(
        first.restricted_result,
        run(&request, Limits::default()).restricted_result
    );
    let other = ParameterEntity::NativeSlot {
        callable: case.effective.callable,
        signature: case.variant.signature,
        parameter: case.slot.parameter,
    };
    request.formal = other.id();
    assert!(
        assess(
            &request,
            &context,
            &path,
            &[atom],
            complete(),
            &[],
            Limits::default(),
            &case.f.budget
        )
        .is_err()
    );
}

#[test]
fn pure_inventory_uses_current_nominal_records_and_rejects_foreign_loading() {
    let case = Case::new(Predicate::IsNone, vec![], None);
    let mut inventory = NativeInventory::new(&case.f.budget);
    macro_rules! entry_inputs { ($($field:ident:$ty:ty,)*) => { $(
        inventory.visit(<$ty>::NAME, &<$ty as Record>::encode(&case.f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();
    )* }; }
    lctx_model::entry_value_inputs!(entry_inputs);
    for (name, batch) in [
        (
            EffectiveCallableAssessment::NAME,
            EffectiveCallableAssessment::encode(std::slice::from_ref(&case.effective)).unwrap(),
        ),
        (
            SignatureVariant::NAME,
            SignatureVariant::encode(std::slice::from_ref(&case.variant)).unwrap(),
        ),
        (
            SignatureSlot::NAME,
            SignatureSlot::encode(std::slice::from_ref(&case.slot)).unwrap(),
        ),
    ] {
        inventory.visit(name, &batch).unwrap();
    }
    let entry = case.f.guard_entry().unwrap();
    assert_eq!(
        inventory.context(&entry).unwrap().formal(),
        case.f.request.formal
    );
    let inputs = NativeInventory::inputs();
    assert!(
        inputs
            .iter()
            .any(|i| i.name() == FlowTestLeafObservation::NAME)
    );
    assert!(inputs.iter().any(|i| i.name() == TestOperandTypeLink::NAME));
    assert!(inputs.iter().all(|i| !i.name().starts_with("legacy")));
    assert!(
        inventory
            .visit("undeclared", &SignatureSlot::encode(&[]).unwrap())
            .is_err()
    );
}

fn local_value_path(kind: transfer::TransferKind) -> Assessment {
    use lctx_model::domain::{
        analysis::{native::*, policy::EvidenceStatus},
        local_semantics::{LocalContribution, LocalData},
    };
    let mut case = Case::new(Predicate::IsNone, vec![], None);
    let template = case.f.data.use_supports.iter().next().unwrap().clone();
    let sink = case.f.data.regions.iter().next().unwrap().statement;
    let value = FlowValueObservation {
        qualification: case.f.q.id(),
        use_: case.f.use_.id(),
        sink,
        kind: FlowSinkKind::Return,
        transfer: kind,
        through_call: false,
    };
    let support = FlowValueSupport {
        assertion: value.id(),
        run: template.run,
        surface: template.surface,
        evidence: template.evidence,
        origin: template.origin,
        mode: template.mode,
        fidelity: template.fidelity,
    };
    case.f.data.values.insert(value.clone()).unwrap();
    case.f.data.value_supports.insert(support.clone()).unwrap();
    let region = case.f.data.regions.iter().next().unwrap().clone();
    let region_support = case.f.data.region_supports.iter().next().unwrap().clone();
    let guard_entry = case.f.guard_entry().unwrap();
    let context = case.context(&guard_entry).unwrap();
    let mut local = LocalData::new(&case.f.budget);
    local.entry = case.f.data;
    for (premise, qualification) in [
        (
            NativeAssertionPremise::Value {
                assertion: value.id(),
                support: support.id(),
            },
            value.qualification,
        ),
        (
            NativeAssertionPremise::Region {
                assertion: region.id(),
                support: region_support.id(),
            },
            region.qualification,
        ),
    ] {
        local
            .native
            .insert(NativeQualification {
                premise: premise.id(),
                qualification,
                family: FactFamily::Flow,
                fidelity: Fidelity::NativeStructural,
                status: EvidenceStatus::StructurallyObserved,
            })
            .unwrap();
    }
    let emission = LocalContribution::derive(
        &local,
        &case.invocation,
        value.id(),
        support.id(),
        &case.f.budget,
    )
    .unwrap()
    .unwrap();
    let path = NativePath::local(&emission, &case.f.budget).unwrap();
    let input = ExactScalar::None {};
    assess(
        &ExactRequest {
            snapshot: snapshot_for(1),
            owner: context.owner(),
            formal: context.formal(),
            value: &input,
            assumptions: Assumptions::default(),
        },
        &context,
        &path,
        &[],
        complete(),
        &[],
        Limits::default(),
        &case.f.budget,
    )
    .unwrap()
}

#[test]
fn unchanged_local_value_path_keeps_established_separate_from_exact_may_result() {
    let result = local_value_path(transfer::TransferKind::Identity);
    assert_eq!(result.verdict, Verdict::Established);
    assert_eq!(result.exact, ExactOutcome::CompatibleUnderMayModel);
    assert_eq!(result.basis, Basis::AdmittedFinitePath);
    assert_eq!(result.work.assignments_applied, 0);
    assert_eq!(
        result.path.relation(),
        local_semantics::LocalContribution::NAME
    );
}

#[test]
fn admitted_derived_path_is_unknown_with_original_condition_and_finite_proof() {
    let result = local_value_path(transfer::TransferKind::Derived);
    assert_eq!(result.verdict, Verdict::Unknown);
    assert_eq!(result.exact, ExactOutcome::Unknown);
    assert_eq!(
        result.reason,
        Some(ObligationKind::ConditionTransferUnsupported)
    );
    assert_eq!(result.original_condition, Diagram::always().id());
    assert_eq!(result.work.assignments_applied, 0);
    assert!(result.restricted_result.is_none());
    assert!(result.proof.contains(&result.path));
}

#[test]
fn distinct_unexamined_causes_survive_model_packet_serialization_without_assignment() {
    let case = Case::new(Predicate::IsNone, vec![], None);
    let entry = case.f.guard_entry().unwrap();
    let path = NativePath::guard(&entry, &case.f.data, &case.f.budget).unwrap();
    let value = str_("ready");
    let request = ExactRequest {
        snapshot: snapshot_for(1),
        owner: case.f.request.owner,
        formal: case.f.request.formal,
        value: &value,
        assumptions: Assumptions::default(),
    };
    for (cause, code) in [
        (ObligationKind::MissingEvidence, 7),
        (ObligationKind::DefaultStabilityUnknown, 32),
        (ObligationKind::IncompatibleContexts, 44),
        (ObligationKind::EntryValueUnknown, 49),
    ] {
        let assessment = unexamined(&request, &entry, cause, &path, &case.f.budget).unwrap();
        let packet = serving::NativeAssessmentPacket::from_canonical(
            &assessment,
            serving::ClaimBasisPacket::from_canonical(
                &assumptions::ResolvedAssumptions::empty(),
                vec![],
            )
            .unwrap(),
        )
        .unwrap();
        let json = serde_json::to_value(&packet).unwrap();
        assert_eq!(json["reason"], serde_json::json!(code));
        assert_eq!(json["exact"], "unknown");
        assert_eq!(json["basis"], "unexamined");
        assert_eq!(json["work"]["assignments_applied"], 0);
        assert!(json["restricted_result"].is_null());
        assert_eq!(packet.original_condition, path.original_condition());
        assert!(!packet.proof.is_empty());
    }
}

#[test]
fn pure_preparation_preserves_context_causes_and_requires_coverage_before_publication() {
    fn inputs(case: &Case, mutation: &str, coverage: bool) -> PreparationInputs {
        let mut inputs = PreparationInputs::new(&case.f.budget);
        macro_rules! entries {($($field:ident:$ty:ty,)*)=>{$(inputs.native.visit(<$ty>::NAME,&<$ty as Record>::encode(&case.f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        macro_rules! theory {($($field:ident:$ty:ty,)*)=>{$(inputs.native.visit(<$ty>::NAME,&<$ty as Record>::encode(&case.theory.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::entry_value_inputs!(entries);
        lctx_model::local_theory_inputs!(theory);
        if mutation != "missing" {
            inputs
                .native
                .effective
                .insert(case.effective.clone())
                .unwrap();
        }
        let mut variant = case.variant.clone();
        if mutation == "crossed" {
            variant.callable =
                Some(serde_json::from_value(serde_json::json!(vec![255; 16])).unwrap());
        }
        let mut slot = case.slot.clone();
        slot.variant = variant.id();
        if mutation == "default" {
            slot.default = DefaultSlot::DefinitionTime;
        }
        inputs.native.variants.insert(variant).unwrap();
        inputs.native.slots.insert(slot).unwrap();
        let mut invocation = case.invocation.clone();
        invocation.subject = None;
        inputs.rows.invocations.insert(invocation.clone()).unwrap();
        let entry = case.f.guard_entry().unwrap();
        let leaf = case.f.data.leaves.iter().next().unwrap();
        let support = case
            .f
            .data
            .leaf_supports
            .iter()
            .find(|r| r.assertion == leaf.id())
            .unwrap();
        let source =
            EntryAccessSource::guard(&case.f.data, case.f.request, leaf.id(), support.id())
                .unwrap();
        inputs.rows.entries.insert(entry.witness().clone()).unwrap();
        inputs.rows.sources.insert(source).unwrap();
        if coverage {
            inputs
                .rows
                .coverage
                .insert(local::AnalysisCoverage {
                    invocation: invocation.id(),
                    capability: analysis::AnalysisCapability::Transfers,
                    scope: case.f.coverage.scope,
                    context: case.f.request.context,
                    premises: ContentHash::of(b"pure native fixture"),
                    availability: normalized::coverage::EvidenceAvailability::Complete,
                    reason: None,
                })
                .unwrap();
        }
        inputs
    }
    let case = Case::new(Predicate::IsNone, vec![], None);
    let classification = selection::classification::ClassificationData::new(&case.f.budget);
    for (mutation, cause) in [
        ("required", None),
        ("default", Some(ObligationKind::DefaultStabilityUnknown)),
        ("missing", Some(ObligationKind::MissingEvidence)),
        ("crossed", Some(ObligationKind::IncompatibleContexts)),
    ] {
        let prepared = PreparedNativeSemantics::prepare(
            inputs(&case, mutation, true),
            &classification,
            &case.f.budget,
        )
        .unwrap();
        assert_eq!(
            prepared.unexamined(
                case.f.request.owner,
                case.f.request.formal,
                case.f.request.context
            ),
            cause.is_some(),
            "{mutation}"
        );
        assert!(
            prepared
                .path_count(
                    case.f.request.owner,
                    case.f.request.formal,
                    case.f.request.context
                )
                .unwrap()
                > 0
        );
        if let Some(cause) = cause {
            let value = str_("ready");
            let request = ExactRequest {
                snapshot: snapshot_for(1),
                owner: case.f.request.owner,
                formal: case.f.request.formal,
                value: &value,
                assumptions: Assumptions::default(),
            };
            let result = prepared
                .assess_path(&request, case.f.request.context, 0, &case.f.budget)
                .unwrap();
            assert_eq!(result.reason, Some(cause));
            assert_eq!(result.work.assignments_applied, 0);
            assert_eq!(result.exact, ExactOutcome::Unknown);
            assert!(!result.proof.is_empty());
        }
    }
    assert!(
        PreparedNativeSemantics::prepare(
            inputs(&case, "default", false),
            &classification,
            &case.f.budget
        )
        .is_err(),
        "missing required coverage cannot publish an unexamined default path"
    );
}
